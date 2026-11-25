use std::time::{Duration, SystemTime};

use fm_core::FileEntry;
use regex::RegexBuilder;

use crate::search::{DateFilter, FileTypeFilter, SizeFilter, fuzzy_rank_entries, matches_type_ext};

/// Critérios extraídos de uma consulta de filtro rápido (`Ctrl+K`) em memória.
#[derive(Debug, Clone, Default)]
pub struct QuickFilterSpec {
    pub free_text: String,
    pub extensions: Vec<String>,
    pub size_filter: SizeFilter,
    pub date_filter: DateFilter,
    pub type_filter: FileTypeFilter,
    pub regex_pattern: Option<String>,
}

/// Faz o parsing de tokens de filtro rápido (`ext:rs`, `>5MB`, `<100KB`, `hoje`, `7d`, `type:image`, `regex:...`).
pub fn parse_quick_filter(raw: &str) -> QuickFilterSpec {
    let mut spec = QuickFilterSpec::default();
    let mut free_parts: Vec<String> = Vec::new();

    for token in split_query_tokens(raw) {
        let lower = token.to_lowercase();
        if let Some(exts) = lower.strip_prefix("ext:") {
            for ext in exts.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                spec.extensions.push(ext.trim_start_matches('.').to_owned());
            }
        } else if let Some(kind) = lower
            .strip_prefix("type:")
            .or_else(|| lower.strip_prefix("tipo:"))
        {
            if let Some(parsed) = parse_type_filter(kind) {
                spec.type_filter = parsed;
            } else {
                free_parts.push(token);
            }
        } else if let Some(pat) = token
            .strip_prefix("regex:")
            .or_else(|| token.strip_prefix("REGEX:"))
        {
            if !pat.is_empty() {
                spec.regex_pattern = Some(pat.to_owned());
            }
        } else if matches!(lower.as_str(), "hoje" | "today") {
            spec.date_filter = DateFilter::Today;
        } else if matches!(lower.as_str(), "7d" | "7dias") {
            spec.date_filter = DateFilter::Last7Days;
        } else if matches!(lower.as_str(), "30d" | "30dias") {
            spec.date_filter = DateFilter::Last30Days;
        } else if let Some(size) = parse_size_token(&lower) {
            spec.size_filter = size;
        } else {
            free_parts.push(token);
        }
    }

    spec.free_text = free_parts.join(" ");
    spec
}

/// Filtra entradas já carregadas na aba atual usando tokens rápidos sem tocar no disco.
pub fn quick_filter_entries(entries: &[FileEntry], raw_query: &str) -> Vec<FileEntry> {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return entries.to_vec();
    }

    let spec = parse_quick_filter(trimmed);
    let compiled_regex = spec
        .regex_pattern
        .as_deref()
        .and_then(|pat| RegexBuilder::new(pat).case_insensitive(true).build().ok());

    if spec.regex_pattern.is_some() && compiled_regex.is_none() {
        return Vec::new();
    }

    let now = SystemTime::now();
    let prefiltered: Vec<FileEntry> = entries
        .iter()
        .filter(|entry| {
            if !spec.extensions.is_empty() {
                let ext = entry
                    .extension
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase();
                if !spec.extensions.iter().any(|wanted| wanted == &ext) {
                    return false;
                }
            }

            if !matches_size_filter(entry.size_bytes, spec.size_filter, entry.kind.is_dir_like()) {
                return false;
            }

            if !matches_date_filter(entry.modified_at, spec.date_filter, now) {
                return false;
            }

            if spec.type_filter != FileTypeFilter::Any {
                if entry.kind.is_dir_like() {
                    return false;
                }
                let ext = entry.extension.as_deref().unwrap_or_default();
                if !matches_type_ext(ext, spec.type_filter) {
                    return false;
                }
            }

            if let Some(re) = &compiled_regex
                && !re.is_match(entry.name.as_str())
            {
                return false;
            }

            true
        })
        .cloned()
        .collect();

    if spec.free_text.is_empty() {
        prefiltered
    } else {
        fuzzy_rank_entries(&prefiltered, &spec.free_text)
    }
}

pub(crate) fn split_query_tokens(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in raw.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub(crate) fn parse_type_filter(raw: &str) -> Option<FileTypeFilter> {
    match raw.trim().to_lowercase().as_str() {
        "code" | "codigo" | "código" | "src" => Some(FileTypeFilter::Code),
        "doc" | "docs" | "document" | "documents" | "documento" | "documentos" => {
            Some(FileTypeFilter::Documents)
        }
        "img" | "image" | "images" | "imagem" | "imagens" => Some(FileTypeFilter::Images),
        "audio" | "áudio" | "music" | "musica" | "música" => Some(FileTypeFilter::Audio),
        "video" | "vídeo" | "videos" | "vídeos" => Some(FileTypeFilter::Video),
        "archive" | "archives" | "zip" | "compactado" => Some(FileTypeFilter::Archives),
        _ => None,
    }
}

pub(crate) fn parse_size_token(token: &str) -> Option<SizeFilter> {
    if let Some(rest) = token.strip_prefix(">=").or_else(|| token.strip_prefix('>')) {
        parse_byte_size(rest).map(SizeFilter::Gt)
    } else if let Some(rest) = token.strip_prefix("<=").or_else(|| token.strip_prefix('<')) {
        parse_byte_size(rest).map(SizeFilter::Lt)
    } else {
        None
    }
}

pub(crate) fn parse_byte_size(raw: &str) -> Option<u64> {
    let s = raw.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    let split_idx = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let (num_str, unit) = s.split_at(split_idx);
    let value: f64 = num_str.parse().ok()?;
    if value < 0.0 {
        return None;
    }

    let multiplier: f64 = match unit.trim() {
        "" | "b" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((value * multiplier) as u64)
}

pub(crate) fn matches_size_filter(size_bytes: u64, filter: SizeFilter, is_dir: bool) -> bool {
    match filter {
        SizeFilter::Any => true,
        SizeFilter::Gt(min) => !is_dir && size_bytes > min,
        SizeFilter::Lt(max) => !is_dir && size_bytes < max,
        SizeFilter::Range { min, max } => {
            if is_dir {
                return false;
            }
            let min_ok = min.is_none_or(|m| size_bytes >= m);
            let max_ok = max.is_none_or(|m| size_bytes <= m);
            min_ok && max_ok
        }
    }
}

pub(crate) fn matches_date_filter(
    modified_at: Option<SystemTime>,
    filter: DateFilter,
    now: SystemTime,
) -> bool {
    let max_age = match filter {
        DateFilter::Any => return true,
        DateFilter::Today => Duration::from_secs(86_400),
        DateFilter::Last7Days => Duration::from_secs(7 * 86_400),
        DateFilter::Last30Days => Duration::from_secs(30 * 86_400),
    };

    let Some(modified) = modified_at else {
        return false;
    };
    now.duration_since(modified)
        .map(|age| age <= max_age)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;
    use fm_core::{EntryId, EntryKind, PermissionsMeta};
    use std::path::PathBuf;

    fn entry(name: &str, ext: Option<&str>, size: u64, age_secs: u64) -> FileEntry {
        FileEntry {
            id: EntryId(1),
            name: CompactString::new(name),
            path: PathBuf::from(name),
            extension: ext.map(CompactString::new),
            kind: EntryKind::File,
            size_bytes: size,
            created_at: None,
            modified_at: Some(SystemTime::now() - Duration::from_secs(age_secs)),
            is_hidden: false,
            permissions: PermissionsMeta::PosixMode(0o644),
            child_count: None,
            mime_badge: None,
        }
    }

    #[test]
    fn quick_filter_handles_ext_size_date_and_type_without_disk() {
        let entries = vec![
            entry("main.rs", Some("rs"), 8 * 1024, 600),
            entry("photo.png", Some("png"), 3 * 1024 * 1024, 3600),
            entry("old_log.txt", Some("txt"), 20 * 1024, 20 * 86_400),
        ];

        let rs_big = quick_filter_entries(&entries, "ext:rs >5KB");
        assert_eq!(rs_big.len(), 1);
        assert_eq!(rs_big[0].name, "main.rs");

        let images = quick_filter_entries(&entries, "type:image hoje");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].name, "photo.png");

        let old_regex = quick_filter_entries(&entries, "regex:^old_.* 30d");
        assert_eq!(old_regex.len(), 1);
        assert_eq!(old_regex[0].name, "old_log.txt");
    }
}
