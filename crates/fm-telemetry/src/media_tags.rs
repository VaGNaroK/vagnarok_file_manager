//! Leitura assíncrona e em streaming parcial de metadados EXIF e ID3 para renomeação em lote.
//!
//! Todos os arquivos são abertos via [`crate::open_shared::open_shared_read`] para não
//! bloquear arquivos em uso no Windows e respeitam [`CancellationToken`].

use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use compact_str::CompactString;
use exif::{In, Reader as ExifReader, Tag, Value};
use fm_core::BulkRenameMediaTags;
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;

use crate::open_shared::open_shared_read;

/// Limite máximo lido do cabeçalho de imagem para localizar segmentos EXIF (512 KiB).
const MAX_EXIF_HEADER_BYTES: u64 = 512 * 1024;
/// Limite de segurança para percorrer frames ID3v2 sem varrer arquivos corrompidos (4 MiB).
const MAX_ID3V2_SCAN_BYTES: u64 = 4 * 1024 * 1024;
/// Limite de payload de frame textual ID3 (evita alocar frames malformados enormes).
const MAX_ID3_TEXT_FRAME_BYTES: usize = 8 * 1024;
/// Concorrência máxima de leituras em disco simultâneas.
const MAX_CONCURRENT_TAG_READS: usize = 6;
/// Tamanho do lote enviado à UI para reduzir redraws durante escaneamentos grandes.
const UI_BATCH_FLUSH_SIZE: usize = 20;

/// Resultado de extração associado ao índice original do item na lista do modal.
#[derive(Debug, Clone)]
pub struct ExtractedMediaTagItem {
    pub input_index: usize,
    pub tags: BulkRenameMediaTags,
}

/// Atualiza incrementalmente tags EXIF/ID3 para uma lista de arquivos em background.
pub async fn extract_media_tags_stream(
    items: Vec<(usize, PathBuf)>,
    need_exif: bool,
    need_id3: bool,
    cancel: CancellationToken,
    tx: mpsc::Sender<Vec<ExtractedMediaTagItem>>,
) {
    if items.is_empty() || (!need_exif && !need_id3) {
        return;
    }

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TAG_READS));
    let (item_tx, mut item_rx) = mpsc::channel::<ExtractedMediaTagItem>(64);

    let producer_cancel = cancel.clone();
    let producer = tokio::spawn(async move {
        for (input_index, path) in items {
            if producer_cancel.is_cancelled() {
                break;
            }
            let Ok(permit) = Arc::clone(&semaphore).acquire_owned().await else {
                break;
            };
            if producer_cancel.is_cancelled() {
                drop(permit);
                break;
            }
            let item_tx = item_tx.clone();
            let job_cancel = producer_cancel.clone();
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                if job_cancel.is_cancelled() {
                    return;
                }
                let tags = extract_media_tags_for_path(&path, need_exif, need_id3);
                if !job_cancel.is_cancelled() {
                    let _ = item_tx.blocking_send(ExtractedMediaTagItem { input_index, tags });
                }
            });
        }
    });

    let mut buffer = Vec::with_capacity(UI_BATCH_FLUSH_SIZE);
    while let Some(item) = item_rx.recv().await {
        if cancel.is_cancelled() {
            break;
        }
        buffer.push(item);
        if buffer.len() >= UI_BATCH_FLUSH_SIZE {
            let chunk = std::mem::take(&mut buffer);
            if tx.send(chunk).await.is_err() {
                cancel.cancel();
                break;
            }
        }
    }

    if !buffer.is_empty() && !cancel.is_cancelled() {
        let _ = tx.send(buffer).await;
    }

    producer.abort();
}

/// Extrai síncrona e com leitura parcial tags EXIF e/ou ID3 de um único arquivo.
#[must_use]
pub fn extract_media_tags_for_path(
    path: &Path,
    need_exif: bool,
    need_id3: bool,
) -> BulkRenameMediaTags {
    let mut tags = BulkRenameMediaTags::default();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    if need_exif && is_exif_candidate_ext(&ext) {
        fill_exif_tags(path, &mut tags);
    }
    if need_id3 && is_id3_candidate_ext(&ext) {
        fill_id3_tags(path, &mut tags);
    }
    tags
}

fn is_exif_candidate_ext(ext: &str) -> bool {
    matches!(
        ext,
        "jpg" | "jpeg" | "tif" | "tiff" | "webp" | "png" | "heic" | "heif" | "avif" | "dng" | "cr2"
            | "nef" | "arw"
    )
}

fn is_id3_candidate_ext(ext: &str) -> bool {
    matches!(ext, "mp3" | "mp2" | "aac" | "wav" | "aiff" | "aif")
}

fn fill_exif_tags(path: &Path, out: &mut BulkRenameMediaTags) {
    let Ok(file) = open_shared_read(path) else {
        return;
    };
    let mut limited = BufReader::new(file.take(MAX_EXIF_HEADER_BYTES));
    let Ok(exif_data) = ExifReader::new().read_from_container(&mut limited) else {
        return;
    };

    let raw_date = exif_ascii_field(&exif_data, Tag::DateTimeOriginal)
        .or_else(|| exif_ascii_field(&exif_data, Tag::DateTimeDigitized))
        .or_else(|| exif_ascii_field(&exif_data, Tag::DateTime));

    if let Some(date_str) = raw_date
        && let Some((ymd, full)) = normalize_exif_datetime(&date_str)
    {
        out.exif_date_ymd = Some(CompactString::from(ymd));
        out.exif_date_taken = Some(CompactString::from(full));
    }

    if let Some(model) = exif_ascii_field(&exif_data, Tag::Model) {
        let cleaned = sanitize_tag_text(&model);
        if !cleaned.is_empty() {
            out.exif_camera_model = Some(CompactString::from(cleaned));
        }
    }
}

fn exif_ascii_field(exif_data: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif_data.get_field(tag, In::PRIMARY)?;
    if let Value::Ascii(ref vecs) = field.value {
        for bytes in vecs {
            let s = String::from_utf8_lossy(bytes)
                .trim_matches('\0')
                .trim()
                .to_owned();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    let display = field.display_value().to_string();
    let trimmed = display.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Converte `"2026:08:14 15:30:22"` para `("2026-08-14", "2026-08-14_15-30-22")`.
fn normalize_exif_datetime(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim();
    let (date_part, time_part) = trimmed
        .split_once([' ', 'T'])
        .unwrap_or((trimmed, ""));
    let date_norm: String = date_part
        .chars()
        .map(|c| if c == ':' { '-' } else { c })
        .collect();
    if date_norm.len() < 8 {
        return None;
    }
    let time_clean = time_part.split(['+', '-', 'Z']).next().unwrap_or("").trim();
    let full = if time_clean.is_empty() {
        date_norm.clone()
    } else {
        let time_norm: String = time_clean
            .chars()
            .map(|c| if c == ':' { '-' } else { c })
            .collect();
        format!("{date_norm}_{time_norm}")
    };
    Some((date_norm, full))
}

fn fill_id3_tags(path: &Path, out: &mut BulkRenameMediaTags) {
    let Ok(mut file) = open_shared_read(path) else {
        return;
    };

    let _ = parse_id3v2_frames(&mut file, out);

    // Se faltou título/artista/álbum/ano/faixa, tenta fallback em ID3v1 nos últimos 128 bytes.
    if out.id3_title.is_none()
        || out.id3_artist.is_none()
        || out.id3_album.is_none()
        || out.id3_year.is_none()
        || out.id3_track.is_none()
    {
        let _ = parse_id3v1_tail(&mut file, out);
    }
}

fn parse_id3v2_frames<R: Read + Seek>(reader: &mut R, out: &mut BulkRenameMediaTags) -> std::io::Result<()> {
    let mut header = [0u8; 10];
    reader.read_exact(&mut header)?;
    if &header[0..3] != b"ID3" {
        return Ok(());
    }

    let version_major = header[3];
    if version_major != 3 && version_major != 4 {
        return Ok(());
    }
    let flags = header[5];
    let tag_size = u64::from(decode_synchsafe_u32(&header[6..10]));
    if tag_size == 0 {
        return Ok(());
    }

    let scan_end = 10 + tag_size.min(MAX_ID3V2_SCAN_BYTES);
    let mut cursor = 10u64;

    // Cabeçalho estendido opcional (bit 6 da flag do ID3v2)
    if (flags & 0x40) != 0 && cursor + 4 <= scan_end {
        let mut ext_len_buf = [0u8; 4];
        reader.read_exact(&mut ext_len_buf)?;
        let ext_len = if version_major == 4 {
            u64::from(decode_synchsafe_u32(&ext_len_buf)).saturating_sub(4)
        } else {
            u64::from(u32::from_be_bytes(ext_len_buf))
        };
        cursor = cursor.saturating_add(4 + ext_len);
        reader.seek(SeekFrom::Start(cursor))?;
    }

    while cursor + 10 <= scan_end {
        let mut frame_hdr = [0u8; 10];
        if reader.read_exact(&mut frame_hdr).is_err() {
            break;
        }
        cursor += 10;

        // Padding zerado marca fim real das frames antes do tamanho declarado
        if frame_hdr[0..4] == [0, 0, 0, 0] {
            break;
        }
        if !frame_hdr[0..4]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            break;
        }

        let frame_size = if version_major == 4 {
            u64::from(decode_synchsafe_u32(&frame_hdr[4..8]))
        } else {
            u64::from(u32::from_be_bytes([
                frame_hdr[4],
                frame_hdr[5],
                frame_hdr[6],
                frame_hdr[7],
            ]))
        };
        if frame_size == 0 || cursor + frame_size > scan_end {
            break;
        }

        let frame_id = &frame_hdr[0..4];
        let is_wanted_text = matches!(
            frame_id,
            b"TIT2" | b"TPE1" | b"TALB" | b"TRCK" | b"TDRC" | b"TYER"
        );

        if is_wanted_text && frame_size <= MAX_ID3_TEXT_FRAME_BYTES as u64 {
            let mut payload = vec![0u8; frame_size as usize];
            if reader.read_exact(&mut payload).is_err() {
                break;
            }
            cursor += frame_size;

            if let Some(decoded) = decode_id3_text_payload(&payload) {
                let cleaned = sanitize_tag_text(&decoded);
                if !cleaned.is_empty() {
                    match frame_id {
                        b"TIT2" => {
                            out.id3_title.get_or_insert_with(|| CompactString::from(cleaned));
                        }
                        b"TPE1" => {
                            out.id3_artist.get_or_insert_with(|| CompactString::from(cleaned));
                        }
                        b"TALB" => {
                            out.id3_album.get_or_insert_with(|| CompactString::from(cleaned));
                        }
                        b"TRCK" => {
                            if out.id3_track.is_none()
                                && let Some(track_num) = parse_track_number(&cleaned)
                            {
                                out.id3_track = Some(track_num);
                            }
                        }
                        b"TDRC" | b"TYER" => {
                            let year_part: String =
                                cleaned.chars().take_while(char::is_ascii_digit).take(4).collect();
                            if !year_part.is_empty() {
                                out.id3_year.get_or_insert_with(|| CompactString::from(year_part));
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Pula frames pesados (como capas `APIC` ou `PRIV`) sem alocar payload na RAM.
            let skip = i64::try_from(frame_size).unwrap_or(0);
            if skip <= 0 {
                break;
            }
            reader.seek(SeekFrom::Current(skip))?;
            cursor += frame_size;
        }
    }

    Ok(())
}

fn parse_id3v1_tail<R: Read + Seek>(reader: &mut R, out: &mut BulkRenameMediaTags) -> std::io::Result<()> {
    let file_len = reader.seek(SeekFrom::End(0))?;
    if file_len < 128 {
        return Ok(());
    }
    reader.seek(SeekFrom::End(-128))?;
    let mut buf = [0u8; 128];
    reader.read_exact(&mut buf)?;
    if &buf[0..3] != b"TAG" {
        return Ok(());
    }

    if out.id3_title.is_none() {
        let s = decode_latin1_trimmed(&buf[3..33]);
        if !s.is_empty() {
            out.id3_title = Some(CompactString::from(s));
        }
    }
    if out.id3_artist.is_none() {
        let s = decode_latin1_trimmed(&buf[33..63]);
        if !s.is_empty() {
            out.id3_artist = Some(CompactString::from(s));
        }
    }
    if out.id3_album.is_none() {
        let s = decode_latin1_trimmed(&buf[63..93]);
        if !s.is_empty() {
            out.id3_album = Some(CompactString::from(s));
        }
    }
    if out.id3_year.is_none() {
        let s = decode_latin1_trimmed(&buf[93..97]);
        if !s.is_empty() {
            out.id3_year = Some(CompactString::from(s));
        }
    }
    // ID3v1.1: byte 125 == 0, byte 126 == número da faixa (1..=255)
    if out.id3_track.is_none() && buf[125] == 0 && buf[126] > 0 {
        out.id3_track = Some(u32::from(buf[126]));
    }

    Ok(())
}

fn decode_synchsafe_u32(bytes: &[u8]) -> u32 {
    if bytes.len() < 4 {
        return 0;
    }
    ((u32::from(bytes[0]) & 0x7F) << 21)
        | ((u32::from(bytes[1]) & 0x7F) << 14)
        | ((u32::from(bytes[2]) & 0x7F) << 7)
        | (u32::from(bytes[3]) & 0x7F)
}

fn decode_id3_text_payload(payload: &[u8]) -> Option<String> {
    if payload.is_empty() {
        return None;
    }
    let encoding = payload[0];
    let body = &payload[1..];
    match encoding {
        0 => Some(decode_latin1_trimmed(body)),
        1 => decode_utf16_with_bom(body),
        2 => decode_utf16_units(body, true),
        3 => {
            let trimmed = body.split(|&b| b == 0).next().unwrap_or(body);
            Some(String::from_utf8_lossy(trimmed).trim().to_owned())
        }
        _ => None,
    }
}

fn decode_latin1_trimmed(bytes: &[u8]) -> String {
    let slice = bytes.split(|&b| b == 0).next().unwrap_or(bytes);
    let raw: String = slice.iter().map(|&b| char::from(b)).collect();
    raw.trim().to_owned()
}

fn decode_utf16_with_bom(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 {
        return Some(String::new());
    }
    match (bytes[0], bytes[1]) {
        (0xFE, 0xFF) => decode_utf16_units(&bytes[2..], true),
        (0xFF, 0xFE) => decode_utf16_units(&bytes[2..], false),
        _ => decode_utf16_units(bytes, false),
    }
}

fn decode_utf16_units(bytes: &[u8], big_endian: bool) -> Option<String> {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.as_chunks::<2>().0 {
        let u = if big_endian {
            u16::from_be_bytes([pair[0], pair[1]])
        } else {
            u16::from_le_bytes([pair[0], pair[1]])
        };
        if u == 0 {
            break;
        }
        units.push(u);
    }
    Some(String::from_utf16_lossy(&units).trim().to_owned())
}

fn parse_track_number(raw: &str) -> Option<u32> {
    let head = raw.split('/').next().unwrap_or(raw).trim();
    let digits: String = head.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u32>().ok().filter(|&n| n > 0)
}

fn sanitize_tag_text(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vfm_media_tag_test_{nanos}_{name}"))
    }

    fn synchsafe(val: u32) -> [u8; 4] {
        [
            ((val >> 21) & 0x7F) as u8,
            ((val >> 14) & 0x7F) as u8,
            ((val >> 7) & 0x7F) as u8,
            (val & 0x7F) as u8,
        ]
    }

    #[test]
    fn parses_id3v2_and_skips_large_apic_frame() {
        let path = temp_file("song.mp3");
        let mut frames = Vec::new();

        // APIC frame gigante simulado (512 bytes de zeros) logo no começo
        let apic_payload = vec![0xAAu8; 512];
        frames.extend_from_slice(b"APIC");
        frames.extend_from_slice(&(apic_payload.len() as u32).to_be_bytes());
        frames.extend_from_slice(&[0, 0]);
        frames.extend_from_slice(&apic_payload);

        // TPE1 UTF-8
        let mut tpe1 = vec![3u8];
        tpe1.extend_from_slice(b"Daft Punk");
        frames.extend_from_slice(b"TPE1");
        frames.extend_from_slice(&(tpe1.len() as u32).to_be_bytes());
        frames.extend_from_slice(&[0, 0]);
        frames.extend_from_slice(&tpe1);

        // TIT2 UTF-16LE with BOM
        let mut tit2 = vec![1u8, 0xFF, 0xFE];
        for u in "Voyager".encode_utf16() {
            tit2.extend_from_slice(&u.to_le_bytes());
        }
        frames.extend_from_slice(b"TIT2");
        frames.extend_from_slice(&(tit2.len() as u32).to_be_bytes());
        frames.extend_from_slice(&[0, 0]);
        frames.extend_from_slice(&tit2);

        // TRCK "04/14"
        let mut trck = vec![0u8];
        trck.extend_from_slice(b"04/14");
        frames.extend_from_slice(b"TRCK");
        frames.extend_from_slice(&(trck.len() as u32).to_be_bytes());
        frames.extend_from_slice(&[0, 0]);
        frames.extend_from_slice(&trck);

        let mut file = std::fs::File::create(&path).expect("create test mp3");
        file.write_all(b"ID3").expect("id3 sig");
        file.write_all(&[3, 0, 0]).expect("v2.3 flags");
        file.write_all(&synchsafe(frames.len() as u32))
            .expect("tag len");
        file.write_all(&frames).expect("frames");
        drop(file);

        let tags = extract_media_tags_for_path(&path, false, true);
        let _ = std::fs::remove_file(&path);

        assert_eq!(tags.id3_artist.as_deref(), Some("Daft Punk"));
        assert_eq!(tags.id3_title.as_deref(), Some("Voyager"));
        assert_eq!(tags.id3_track, Some(4));
    }

    #[test]
    fn parses_id3v1_1_fallback_at_end_of_file() {
        let path = temp_file("legacy.mp3");
        let mut tail = [0u8; 128];
        tail[0..3].copy_from_slice(b"TAG");
        tail[3..12].copy_from_slice(b"Nostalgia");
        tail[33..40].copy_from_slice(b"ArtistX");
        tail[63..69].copy_from_slice(b"AlbumY");
        tail[93..97].copy_from_slice(b"1999");
        tail[125] = 0;
        tail[126] = 9;

        let mut file = std::fs::File::create(&path).expect("create test mp3");
        file.write_all(&[0xFFu8; 64]).expect("audio bytes");
        file.write_all(&tail).expect("id3v1");
        drop(file);

        let tags = extract_media_tags_for_path(&path, false, true);
        let _ = std::fs::remove_file(&path);

        assert_eq!(tags.id3_title.as_deref(), Some("Nostalgia"));
        assert_eq!(tags.id3_artist.as_deref(), Some("ArtistX"));
        assert_eq!(tags.id3_album.as_deref(), Some("AlbumY"));
        assert_eq!(tags.id3_year.as_deref(), Some("1999"));
        assert_eq!(tags.id3_track, Some(9));
    }

    #[test]
    fn normalizes_exif_datetime_safely() {
        let (ymd, full) =
            normalize_exif_datetime("2026:08:14 15:30:22").expect("valid exif date");
        assert_eq!(ymd, "2026-08-14");
        assert_eq!(full, "2026-08-14_15-30-22");
    }
}
