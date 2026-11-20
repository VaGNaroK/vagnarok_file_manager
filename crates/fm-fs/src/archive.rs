//! Provedor somente leitura para navegação virtual dentro de `.zip`, `.tar`, `.tar.gz`, `.tgz`, `.7z` e `.rar`.

use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use compact_str::CompactString;
use flate2::read::GzDecoder;
use fm_core::{
    DirectorySnapshot, EntryKind, FileEntry, FsError, PermissionsMeta, compute_entry_id,
};
use sevenz_rust2::{ArchiveReader as SevenZReader, Password as SevenZPassword};
use tar::Archive;
use tokio_util::sync::CancellationToken;
use zip::ZipArchive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
    SevenZ,
    #[cfg(feature = "rar-extract")]
    Rar,
}

impl ArchiveKind {
    pub fn fs_label(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::SevenZ => "7z",
            #[cfg(feature = "rar-extract")]
            Self::Rar => "rar",
        }
    }
}

/// Detecta pelo nome se o arquivo é um contêiner suportado (`zip`, `tar`, `tar.gz`, `tgz`, `7z`, `rar`).
pub fn detect_archive_kind(path: &Path) -> Option<ArchiveKind> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(ArchiveKind::TarGz)
    } else if name.ends_with(".zip") {
        Some(ArchiveKind::Zip)
    } else if name.ends_with(".tar") {
        Some(ArchiveKind::Tar)
    } else if name.ends_with(".7z") {
        Some(ArchiveKind::SevenZ)
    } else {
        #[cfg(feature = "rar-extract")]
        if name.ends_with(".rar") {
            return Some(ArchiveKind::Rar);
        }
        None
    }
}

/// Retorna `true` quando `path` aponta para um arquivo compactado suportado existente em disco.
pub fn is_archive_file(path: &Path) -> bool {
    detect_archive_kind(path).is_some() && path.is_file()
}

/// Retorna `true` apenas quando `path` aponta para um item virtual **dentro** de um arquivo
/// compactado (`inner_path` não vazio), e `false` para o próprio arquivo `.zip`/`.7z`/`.rar` em disco.
pub fn is_inside_archive_path(path: &Path) -> bool {
    split_archive_path(path).is_some_and(|(_, inner, _)| !inner.is_empty())
}

/// Divide um caminho possivelmente virtual em `(caminho_arquivo_compactado, prefixo_interno_posix, tipo)`.
pub fn split_archive_path(path: &Path) -> Option<(PathBuf, String, ArchiveKind)> {
    for ancestor in path.ancestors() {
        let Some(kind) = detect_archive_kind(ancestor) else {
            continue;
        };
        if !ancestor.is_file() {
            continue;
        }
        let rel = path.strip_prefix(ancestor).ok()?;
        let inner = normalize_rel_path(rel);
        return Some((ancestor.to_path_buf(), inner, kind));
    }
    None
}

fn normalize_rel_path(rel: &Path) -> String {
    let parts: Vec<String> = rel
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(p) => Some(p.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    parts.join("/")
}

fn normalize_archive_entry_name(raw: &str) -> (String, bool) {
    let is_dir_hint = raw.ends_with('/');
    let trimmed = raw
        .trim_matches('/')
        .trim_start_matches("./")
        .replace('\\', "/");
    let clean: Vec<&str> = trimmed
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    (clean.join("/"), is_dir_hint)
}

#[derive(Debug, Clone)]
struct RawArchiveItem {
    is_dir: bool,
    size_bytes: u64,
    modified_at: Option<SystemTime>,
    mode: Option<u32>,
}

/// Lista os filhos diretos de `inner_prefix` dentro do arquivo compactado em `archive_path`.
pub fn list_archive_dir(
    archive_path: &Path,
    inner_prefix: &str,
    kind: ArchiveKind,
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let mut children: BTreeMap<String, RawArchiveItem> = BTreeMap::new();
    let prefix_with_slash = if inner_prefix.is_empty() {
        String::new()
    } else {
        format!("{inner_prefix}/")
    };

    let mut feed_entry =
        |raw_name: &str, is_dir: bool, size: u64, mtime: Option<SystemTime>, mode: Option<u32>| {
            let (norm, dir_hint) = normalize_archive_entry_name(raw_name);
            if norm.is_empty() {
                return;
            }
            let rest = if prefix_with_slash.is_empty() {
                norm.as_str()
            } else if let Some(stripped) = norm.strip_prefix(&prefix_with_slash) {
                stripped
            } else {
                return;
            };
            if rest.is_empty() {
                return;
            }

            let mut split = rest.splitn(2, '/');
            let child_name = split.next().unwrap_or_default();
            if child_name.is_empty() || child_name == ".." {
                return;
            }
            let has_nested = split.next().is_some();
            let entry_is_dir = has_nested || is_dir || dir_hint;

            children
                .entry(child_name.to_owned())
                .and_modify(|existing| {
                    if entry_is_dir {
                        existing.is_dir = true;
                        existing.size_bytes = 0;
                    } else if !existing.is_dir {
                        existing.size_bytes = size;
                        existing.modified_at = existing.modified_at.or(mtime);
                    }
                })
                .or_insert(RawArchiveItem {
                    is_dir: entry_is_dir,
                    size_bytes: if entry_is_dir { 0 } else { size },
                    modified_at: mtime,
                    mode,
                });
        };

    match kind {
        ArchiveKind::Zip => {
            let file = open_shared_archive(archive_path)?;
            let archive = ZipArchive::new(BufReader::new(file)).map_err(|e| FsError::Io {
                path: archive_path.to_path_buf(),
                message: format!("arquivo ZIP inválido: {e}"),
            })?;
            for idx in 0..archive.len() {
                if idx.is_multiple_of(256) && cancel.is_cancelled() {
                    return Err(FsError::Cancelled);
                }
                let Ok(entry) = archive.by_index_data(idx) else {
                    continue;
                };
                let is_dir = entry.is_dir();
                let size = entry.size();
                let mode = entry.unix_mode();
                let mtime = entry.last_modified().and_then(zip_datetime_to_system_time);
                let Ok(name_cow) = entry.name() else {
                    continue;
                };
                feed_entry(&name_cow, is_dir, size, mtime, mode);
            }
        }
        ArchiveKind::Tar => {
            let file = open_shared_archive(archive_path)?;
            let mut archive = Archive::new(BufReader::new(file));
            scan_tar_entries(&mut archive, archive_path, cancel, &mut feed_entry)?;
        }
        ArchiveKind::TarGz => {
            let file = open_shared_archive(archive_path)?;
            let gz = GzDecoder::new(BufReader::new(file));
            let mut archive = Archive::new(gz);
            scan_tar_entries(&mut archive, archive_path, cancel, &mut feed_entry)?;
        }
        ArchiveKind::SevenZ => {
            let file = open_shared_archive(archive_path)?;
            let archive = SevenZReader::new(BufReader::new(file), SevenZPassword::empty())
                .map_err(|e| FsError::Io {
                    path: archive_path.to_path_buf(),
                    message: format!("arquivo 7z inválido ou protegido: {e}"),
                })?;
            for (idx, entry) in archive.archive().files.iter().enumerate() {
                if idx.is_multiple_of(256) && cancel.is_cancelled() {
                    return Err(FsError::Cancelled);
                }
                feed_entry(entry.name(), entry.is_directory, entry.size, None, None);
            }
        }
        #[cfg(feature = "rar-extract")]
        ArchiveKind::Rar => {
            let file = open_shared_archive(archive_path)?;
            let archive = unrar_rs::RarArchive::open(file).map_err(|e| FsError::Io {
                path: archive_path.to_path_buf(),
                message: format!("arquivo RAR inválido ou protegido: {e}"),
            })?;
            for (idx, member) in archive.entries().enumerate() {
                if idx.is_multiple_of(256) && cancel.is_cancelled() {
                    return Err(FsError::Cancelled);
                }
                let size = member.unpacked_size.unwrap_or(0);
                feed_entry(
                    &member.raw_name,
                    member.is_directory,
                    size,
                    member.mtime,
                    None,
                );
            }
        }
    }

    let virtual_dir_path = if inner_prefix.is_empty() {
        archive_path.to_path_buf()
    } else {
        let mut p = archive_path.to_path_buf();
        for seg in inner_prefix.split('/').filter(|s| !s.is_empty()) {
            p.push(seg);
        }
        p
    };

    let entries = children
        .into_iter()
        .map(|(name_str, meta)| {
            let entry_path = virtual_dir_path.join(&name_str);
            let name = CompactString::from(name_str.as_str());
            let is_hidden = name.starts_with('.');
            let kind_entry = if meta.is_dir {
                EntryKind::Directory
            } else {
                EntryKind::File
            };
            let extension = if meta.is_dir {
                None
            } else {
                entry_path
                    .extension()
                    .map(|e| CompactString::from(e.to_string_lossy().as_ref()))
                    .filter(|s| !s.is_empty())
            };
            let mime_badge = extension.as_ref().map(|e| e.to_ascii_uppercase());
            let permissions = PermissionsMeta::PosixMode(meta.mode.unwrap_or(if meta.is_dir {
                0o755
            } else {
                0o644
            }));
            FileEntry {
                id: compute_entry_id(&entry_path),
                name,
                path: entry_path,
                extension,
                kind: kind_entry,
                size_bytes: meta.size_bytes,
                modified_at: meta.modified_at,
                created_at: None,
                is_hidden,
                permissions,
                child_count: None,
                mime_badge,
            }
        })
        .collect();

    Ok(DirectorySnapshot {
        request_id,
        path: virtual_dir_path,
        entries,
        fs_type: Some(CompactString::from(kind.fs_label())),
    })
}

/// Lê no máximo `max_bytes` de um arquivo específico dentro do arquivo compactado.
pub fn read_archive_bytes(
    archive_path: &Path,
    inner_path: &str,
    kind: ArchiveKind,
    max_bytes: usize,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }
    if inner_path.is_empty() {
        return Err(FsError::NotFound(archive_path.to_path_buf()));
    }

    match kind {
        ArchiveKind::Zip => {
            let file = open_shared_archive(archive_path)?;
            let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|e| FsError::Io {
                path: archive_path.to_path_buf(),
                message: e.to_string(),
            })?;
            for idx in 0..archive.len() {
                if idx.is_multiple_of(128) && cancel.is_cancelled() {
                    return Err(FsError::Cancelled);
                }
                let Ok(entry) = archive.by_index(idx) else {
                    continue;
                };
                if entry.is_dir() {
                    continue;
                }
                let Ok(name_cow) = entry.name() else {
                    continue;
                };
                let (norm, _) = normalize_archive_entry_name(&name_cow);
                if norm == inner_path {
                    let mut out = Vec::with_capacity(max_bytes.min(64 * 1024));
                    entry
                        .take(max_bytes as u64)
                        .read_to_end(&mut out)
                        .map_err(|e| crate::map_io_err(archive_path, e))?;
                    return Ok(out);
                }
            }
            Err(FsError::NotFound(archive_path.join(inner_path)))
        }
        ArchiveKind::Tar => {
            let file = open_shared_archive(archive_path)?;
            let mut archive = Archive::new(BufReader::new(file));
            read_tar_entry_bytes(&mut archive, archive_path, inner_path, max_bytes, cancel)
        }
        ArchiveKind::TarGz => {
            let file = open_shared_archive(archive_path)?;
            let gz = GzDecoder::new(BufReader::new(file));
            let mut archive = Archive::new(gz);
            read_tar_entry_bytes(&mut archive, archive_path, inner_path, max_bytes, cancel)
        }
        ArchiveKind::SevenZ => {
            let file = open_shared_archive(archive_path)?;
            let mut reader = SevenZReader::new(BufReader::new(file), SevenZPassword::empty())
                .map_err(|e| FsError::Io {
                    path: archive_path.to_path_buf(),
                    message: format!("arquivo 7z inválido ou protegido: {e}"),
                })?;
            let mut found: Option<Vec<u8>> = None;
            reader
                .for_each_entries(|entry, r| {
                    if cancel.is_cancelled() {
                        return Ok(false);
                    }
                    if !entry.is_directory {
                        let (norm, _) = normalize_archive_entry_name(entry.name());
                        if norm == inner_path {
                            let mut out = Vec::with_capacity(max_bytes.min(64 * 1024));
                            r.take(max_bytes as u64).read_to_end(&mut out)?;
                            found = Some(out);
                            return Ok(false);
                        }
                    }
                    Ok(true)
                })
                .map_err(|e| FsError::Io {
                    path: archive_path.to_path_buf(),
                    message: e.to_string(),
                })?;
            if cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            found.ok_or_else(|| FsError::NotFound(archive_path.join(inner_path)))
        }
        #[cfg(feature = "rar-extract")]
        ArchiveKind::Rar => {
            let file = open_shared_archive(archive_path)?;
            let mut archive = unrar_rs::RarArchive::open(file).map_err(|e| FsError::Io {
                path: archive_path.to_path_buf(),
                message: format!("arquivo RAR inválido ou protegido: {e}"),
            })?;
            let target_idx = archive.entries().enumerate().find_map(|(idx, m)| {
                if m.is_directory {
                    return None;
                }
                let (norm, _) = normalize_archive_entry_name(&m.raw_name);
                (norm == inner_path).then_some(idx)
            });
            let Some(idx) = target_idx else {
                return Err(FsError::NotFound(archive_path.join(inner_path)));
            };
            let mut sink = BoundedPreviewWriter {
                buf: Vec::with_capacity(max_bytes.min(64 * 1024)),
                max_bytes,
                cancel,
            };
            let res = archive
                .by_index(idx)
                .map_err(|e| FsError::Io {
                    path: archive_path.to_path_buf(),
                    message: e.to_string(),
                })?
                .copy_to(&mut sink);
            if cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            match res {
                Ok(_) => Ok(sink.buf),
                Err(_) if sink.buf.len() >= max_bytes => Ok(sink.buf),
                Err(e) => Err(FsError::Io {
                    path: archive_path.to_path_buf(),
                    message: e.to_string(),
                }),
            }
        }
    }
}

#[cfg(feature = "rar-extract")]
struct BoundedPreviewWriter<'a> {
    buf: Vec<u8>,
    max_bytes: usize,
    cancel: &'a CancellationToken,
}

#[cfg(feature = "rar-extract")]
impl std::io::Write for BoundedPreviewWriter<'_> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled",
            ));
        }
        let remaining = self.max_bytes.saturating_sub(self.buf.len());
        if remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "limit_reached",
            ));
        }
        let take = remaining.min(data.len());
        self.buf.extend_from_slice(&data[..take]);
        if self.buf.len() >= self.max_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "limit_reached",
            ));
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

type TarEntryVisitor<'a> = &'a mut dyn FnMut(&str, bool, u64, Option<SystemTime>, Option<u32>);

fn scan_tar_entries<R: Read>(
    archive: &mut Archive<R>,
    archive_path: &Path,
    cancel: &CancellationToken,
    feed: TarEntryVisitor<'_>,
) -> Result<(), FsError> {
    let entries = archive.entries().map_err(|e| FsError::Io {
        path: archive_path.to_path_buf(),
        message: format!("arquivo TAR inválido: {e}"),
    })?;
    let mut visited = HashSet::new();
    for (idx, item) in entries.enumerate() {
        if idx.is_multiple_of(256) && cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let Ok(entry) = item else {
            continue;
        };
        let Ok(path_cow) = entry.path() else {
            continue;
        };
        let path_str = path_cow.to_string_lossy().into_owned();
        if !visited.insert(path_str.clone()) {
            continue;
        }
        let header = entry.header();
        let is_dir = header.entry_type().is_dir();
        let size = header.size().unwrap_or(0);
        let mtime = header
            .mtime()
            .ok()
            .and_then(|s| UNIX_EPOCH.checked_add(Duration::from_secs(s)));
        let mode = header.mode().ok();
        feed(&path_str, is_dir, size, mtime, mode);
    }
    Ok(())
}

fn read_tar_entry_bytes<R: Read>(
    archive: &mut Archive<R>,
    archive_path: &Path,
    target_inner: &str,
    max_bytes: usize,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, FsError> {
    let entries = archive
        .entries()
        .map_err(|e| crate::map_io_err(archive_path, e))?;
    for (idx, item) in entries.enumerate() {
        if idx.is_multiple_of(128) && cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let Ok(entry) = item else {
            continue;
        };
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let Ok(path_cow) = entry.path() else {
            continue;
        };
        let (norm, _) = normalize_archive_entry_name(&path_cow.to_string_lossy());
        if norm == target_inner {
            let mut out = Vec::with_capacity(max_bytes.min(64 * 1024));
            entry
                .take(max_bytes as u64)
                .read_to_end(&mut out)
                .map_err(|e| crate::map_io_err(archive_path, e))?;
            return Ok(out);
        }
    }
    Err(FsError::NotFound(archive_path.join(target_inner)))
}

pub(crate) fn open_shared_archive(path: &Path) -> Result<File, FsError> {
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0x7);
    }
    opts.open(path).map_err(|e| crate::map_io_err(path, e))
}

fn zip_datetime_to_system_time(dt: zip::DateTime) -> Option<SystemTime> {
    let year = i64::from(dt.year());
    let month = i64::from(dt.month());
    let day = i64::from(dt.day());
    if !(1970..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    if days < 0 {
        return None;
    }
    let secs = (days as u64) * 86_400
        + u64::from(dt.hour()) * 3_600
        + u64::from(dt.minute()) * 60
        + u64::from(dt.second());
    UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}

// Algoritmo civil-from-days / days-from-civil de Howard Hinnant (sem dependências externas).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn lists_zip_archive_hierarchy_and_reads_file_bytes() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vagnarok_zip_test_{nanos}"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let zip_path = dir.join("bundle.zip");

        {
            let file = File::create(&zip_path).expect("create zip");
            let mut writer = zip::ZipWriter::new(file);
            let opts = SimpleFileOptions::default();
            writer.start_file("README.md", opts).expect("start file");
            writer.write_all(b"# Hello Archive").expect("write readme");
            writer
                .start_file("src/main.rs", opts)
                .expect("start nested");
            writer.write_all(b"fn main() {}").expect("write main.rs");
            writer.finish().expect("finish zip");
        }

        let cancel = CancellationToken::new();
        let root_snap =
            list_archive_dir(&zip_path, "", ArchiveKind::Zip, 1, &cancel).expect("list root");
        assert_eq!(root_snap.fs_type.as_deref(), Some("zip"));
        assert_eq!(root_snap.entries.len(), 2);
        assert!(
            root_snap
                .entries
                .iter()
                .any(|e| e.name == "README.md" && !e.kind.is_dir_like())
        );
        assert!(
            root_snap
                .entries
                .iter()
                .any(|e| e.name == "src" && e.kind.is_dir_like())
        );

        let src_virtual = zip_path.join("src");
        let (detected_arch, inner_prefix, kind) =
            split_archive_path(&src_virtual).expect("split nested");
        assert_eq!(detected_arch, zip_path);
        assert_eq!(inner_prefix, "src");
        assert_eq!(kind, ArchiveKind::Zip);

        let src_snap =
            list_archive_dir(&zip_path, &inner_prefix, kind, 2, &cancel).expect("list src");
        assert_eq!(src_snap.entries.len(), 1);
        assert_eq!(src_snap.entries[0].name, "main.rs");

        let bytes = read_archive_bytes(&zip_path, "src/main.rs", kind, 1024, &cancel)
            .expect("read zipped bytes");
        assert_eq!(bytes, b"fn main() {}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lists_tar_gz_archive_and_reads_bytes() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vagnarok_tgz_test_{nanos}"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let tgz_path = dir.join("pack.tar.gz");

        {
            let file = File::create(&tgz_path).expect("create tgz");
            let gz = GzEncoder::new(file, Compression::default());
            let mut tar_builder = tar::Builder::new(gz);
            let payload = b"cargo = 1";
            let mut header = tar::Header::new_gnu();
            header.set_size(payload.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar_builder
                .append_data(&mut header, "cfg/settings.toml", &payload[..])
                .expect("append entry");
            let gz = tar_builder.into_inner().expect("finish tar");
            gz.finish().expect("finish gz");
        }

        let cancel = CancellationToken::new();
        let snap =
            list_archive_dir(&tgz_path, "", ArchiveKind::TarGz, 1, &cancel).expect("list tgz root");
        assert_eq!(snap.entries.len(), 1);
        assert!(snap.entries[0].kind.is_dir_like());
        assert_eq!(snap.entries[0].name, "cfg");

        let bytes = read_archive_bytes(
            &tgz_path,
            "cfg/settings.toml",
            ArchiveKind::TarGz,
            1024,
            &cancel,
        )
        .expect("read tgz file");
        assert_eq!(bytes, b"cargo = 1");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
