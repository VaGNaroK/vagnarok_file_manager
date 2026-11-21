//! Listagem assíncrona da visão virtual de Arquivos Recentes (`recent://`).
//!
//! Combina o histórico MRU do `AppConfig` com o catálogo `recently-used.xbel` do desktop no Linux.

use std::collections::HashSet;
use std::path::PathBuf;

use compact_str::CompactString;
use fm_core::{
    DirectorySnapshot, EntryKind, FileEntry, FsError, MAX_RECENT_FILES, VIRTUAL_RECENT_PATH,
};
use tokio_util::sync::CancellationToken;

use crate::{make_file_entry, platform};

/// Constrói o snapshot do diretório virtual `recent://` filtrando apenas arquivos existentes.
pub fn list_virtual_recent(
    app_recent: &[PathBuf],
    include_os_recent: bool,
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(app_recent.len() + 32);

    for path in app_recent {
        if seen.insert(path.clone()) {
            candidates.push(path.clone());
        }
    }

    if include_os_recent {
        for path in os_recent_file_candidates() {
            if candidates.len() >= MAX_RECENT_FILES * 2 {
                break;
            }
            if seen.insert(path.clone()) {
                candidates.push(path);
            }
        }
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    for (idx, path) in candidates.into_iter().enumerate() {
        if idx.is_multiple_of(32) && cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        if entries.len() >= MAX_RECENT_FILES {
            break;
        }
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let Some(os_name) = path.file_name() else {
            continue;
        };
        let name = CompactString::from(os_name.to_string_lossy().as_ref());
        let size_bytes = meta.len();
        let modified_at = meta.modified().ok();
        let created_at = meta.created().ok();
        let (perms, is_hidden) = platform::extract_permissions_and_hidden(&name, &meta);
        entries.push(make_file_entry(
            name,
            path,
            EntryKind::File,
            size_bytes,
            modified_at,
            created_at,
            perms,
            is_hidden,
        ));
    }

    Ok(DirectorySnapshot {
        path: PathBuf::from(VIRTUAL_RECENT_PATH),
        entries,
        fs_type: Some(CompactString::new("recent")),
        request_id,
    })
}

fn os_recent_file_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let Some(base) = directories::BaseDirs::new() else {
            return Vec::new();
        };
        let xbel_path = base.data_local_dir().join("recently-used.xbel");
        let Ok(raw) = std::fs::read_to_string(xbel_path) else {
            return Vec::new();
        };
        parse_xbel_recent_paths(&raw)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

/// Extrai caminhos locais `file://...` de um documento `recently-used.xbel` (do mais recente para o mais antigo).
pub fn parse_xbel_recent_paths(xml: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut rest = xml;
    while let Some(pos) = rest.find("href=\"file://") {
        let after = &rest[pos + "href=\"file://".len()..];
        let Some(end_quote) = after.find('"') else {
            break;
        };
        let raw_uri_path = &after[..end_quote];
        let decoded = percent_decode_uri_and_xml(raw_uri_path);
        if !decoded.is_empty() {
            paths.push(PathBuf::from(decoded));
        }
        rest = &after[end_quote + 1..];
    }
    // No XBEL, novos bookmarks costumam ser anexados ao final; invertemos para priorizar os mais recentes.
    paths.reverse();
    paths
}

fn percent_decode_uri_and_xml(input: &str) -> String {
    let unescaped = input
        .replace("&amp;", "&")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    let bytes = unescaped.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2]))
        {
            out.push((h << 4) | l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xbel_file_uris_with_percent_encoding_and_xml_entities() {
        let sample = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <xbel version="1.0">
          <bookmark href="file:///home/user/Documentos/relat%C3%B3rio%20final.pdf" added="2026-11-20T10:00:00Z"/>
          <bookmark href="https://example.com/ignored.png" added="2026-11-20T11:00:00Z"/>
          <bookmark href="file:///home/user/Imagens/foto&amp;capa.jpg" added="2026-11-20T12:00:00Z"/>
        </xbel>
        "#;
        let parsed = parse_xbel_recent_paths(sample);
        assert_eq!(
            parsed,
            vec![
                PathBuf::from("/home/user/Imagens/foto&capa.jpg"),
                PathBuf::from("/home/user/Documentos/relatório final.pdf"),
            ]
        );
    }
}
