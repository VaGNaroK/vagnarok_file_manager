use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use compact_str::CompactString;
use fm_core::FileEntry;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use regex::{Regex, RegexBuilder};
use tokio_util::sync::CancellationToken;

use crate::quick_filter::{
    matches_date_filter, matches_size_filter, parse_size_token, parse_type_filter,
    split_query_tokens,
};

/// Limite padrão de tamanho de arquivo para leitura de conteúdo (16 MiB).
pub const DEFAULT_MAX_CONTENT_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SNIPPETS_PER_FILE: usize = 5;
const BINARY_PROBE_BYTES: usize = 8192;
const MAX_SNIPPET_CHARS: usize = 160;

/// Trecho de linha encontrada dentro de um arquivo na busca de conteúdo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentMatchSnippet {
    pub line_number: usize,
    pub line_text: CompactString,
}

/// Filtro de tamanho de arquivo para busca avançada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeFilter {
    #[default]
    Any,
    Gt(u64),
    Lt(u64),
    Range {
        min: Option<u64>,
        max: Option<u64>,
    },
}

/// Filtro de data de modificação para busca avançada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateFilter {
    #[default]
    Any,
    Today,
    Last7Days,
    Last30Days,
}

/// Filtro de categoria / metadado de arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileTypeFilter {
    #[default]
    Any,
    Code,
    Documents,
    Images,
    Audio,
    Video,
    Archives,
}

/// Critérios completos da busca avançada sem indexação prévia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedSearchQuery {
    pub name_query: String,
    pub name_regex: bool,
    pub content_query: String,
    pub content_case_sensitive: bool,
    pub content_regex: bool,
    pub size_filter: SizeFilter,
    pub date_filter: DateFilter,
    pub type_filter: FileTypeFilter,
    pub include_hidden: bool,
    pub max_file_size_bytes: u64,
}

impl Default for AdvancedSearchQuery {
    fn default() -> Self {
        Self {
            name_query: String::new(),
            name_regex: false,
            content_query: String::new(),
            content_case_sensitive: false,
            content_regex: false,
            size_filter: SizeFilter::Any,
            date_filter: DateFilter::Any,
            type_filter: FileTypeFilter::Any,
            include_hidden: false,
            max_file_size_bytes: DEFAULT_MAX_CONTENT_FILE_BYTES,
        }
    }
}

impl AdvancedSearchQuery {
    /// Retorna `true` quando nenhum termo nem filtro estruturado foi informado.
    pub fn is_empty(&self) -> bool {
        self.name_query.trim().is_empty()
            && self.content_query.trim().is_empty()
            && self.size_filter == SizeFilter::Any
            && self.date_filter == DateFilter::Any
            && self.type_filter == FileTypeFilter::Any
    }
}

/// Resultado de busca profunda em subdiretórios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    pub relative_path: CompactString,
    pub name: CompactString,
    pub is_dir: bool,
    pub score: u32,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<SystemTime>,
    pub content_matches: Vec<ContentMatchSnippet>,
}

/// Filtra e ordena entradas do diretório atual usando `nucleo-matcher`.
pub fn fuzzy_rank_entries(entries: &[FileEntry], query: &str) -> Vec<FileEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return entries.to_vec();
    }

    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::parse(trimmed, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let mut scored: Vec<(u32, usize)> = Vec::new();

    for (idx, entry) in entries.iter().enumerate() {
        buf.clear();
        let haystack = Utf32Str::new(entry.name.as_str(), &mut buf);
        if let Some(score) = pattern.score(haystack, &mut matcher) {
            scored.push((score, idx));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored
        .into_iter()
        .map(|(_, idx)| entries[idx].clone())
        .collect()
}

/// Executa busca profunda em subpastas (por padrão ignorando diretórios/arquivos ocultos).
pub async fn deep_search(
    root: &Path,
    query: &str,
    max_results: usize,
    cancel: CancellationToken,
) -> Vec<SearchHit> {
    deep_search_with_hidden(root, query, max_results, false, cancel).await
}

/// Executa busca profunda em largura (BFS) em subpastas, priorizando correspondências no nome
/// do arquivo e visitando pastas rasas do usuário antes de árvores profundas.
pub async fn deep_search_with_hidden(
    root: &Path,
    query: &str,
    max_results: usize,
    include_hidden: bool,
    cancel: CancellationToken,
) -> Vec<SearchHit> {
    let adv = AdvancedSearchQuery {
        name_query: query.trim().to_owned(),
        include_hidden,
        ..Default::default()
    };
    advanced_search(root, adv, max_results, cancel).await
}

/// Executa busca avançada assíncrona com suporte a regex, metadados, tamanho, data e conteúdo.
pub async fn advanced_search(
    root: &Path,
    query: AdvancedSearchQuery,
    max_results: usize,
    cancel: CancellationToken,
) -> Vec<SearchHit> {
    if query.is_empty() || cancel.is_cancelled() {
        return Vec::new();
    }

    let root_buf = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        advanced_search_blocking(&root_buf, &query, max_results, &cancel)
    })
    .await
    .unwrap_or_default()
}

/// Abre arquivo para leitura compartilhada sem bloquear operações concorrentes no Windows.
fn open_shared_read(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ (1) | FILE_SHARE_WRITE (2) | FILE_SHARE_DELETE (4)
        opts.share_mode(0x7);
    }
    opts.open(path)
}

#[derive(Debug, Clone)]
struct ResolvedQuery {
    name_query: String,
    name_regex: bool,
    content_query: String,
    content_case_sensitive: bool,
    content_regex: bool,
    size_filter: SizeFilter,
    date_filter: DateFilter,
    type_filter: FileTypeFilter,
    extensions: Vec<String>,
    include_hidden: bool,
    max_file_size_bytes: u64,
}

fn resolve_inline_query(base: &AdvancedSearchQuery) -> ResolvedQuery {
    let mut resolved = ResolvedQuery {
        name_query: String::new(),
        name_regex: base.name_regex,
        content_query: base.content_query.trim().to_owned(),
        content_case_sensitive: base.content_case_sensitive,
        content_regex: base.content_regex,
        size_filter: base.size_filter,
        date_filter: base.date_filter,
        type_filter: base.type_filter,
        extensions: Vec::new(),
        include_hidden: base.include_hidden,
        max_file_size_bytes: if base.max_file_size_bytes == 0 {
            DEFAULT_MAX_CONTENT_FILE_BYTES
        } else {
            base.max_file_size_bytes
        },
    };

    if base.name_regex {
        resolved.name_query = base.name_query.trim().to_owned();
        return resolved;
    }

    let mut name_parts: Vec<String> = Vec::new();
    for token in split_query_tokens(base.name_query.trim()) {
        let lower = token.to_lowercase();
        if let Some(val) = token
            .strip_prefix("contendo:")
            .or_else(|| token.strip_prefix("content:"))
            .or_else(|| token.strip_prefix("CONTENDO:"))
            .or_else(|| token.strip_prefix("CONTENT:"))
        {
            if !val.is_empty() && resolved.content_query.is_empty() {
                resolved.content_query = val.to_owned();
            }
        } else if let Some(exts) = lower.strip_prefix("ext:") {
            for ext in exts.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                resolved.extensions.push(ext.trim_start_matches('.').to_owned());
            }
        } else if let Some(kind) = lower
            .strip_prefix("type:")
            .or_else(|| lower.strip_prefix("tipo:"))
        {
            if let Some(parsed) = parse_type_filter(kind) {
                if resolved.type_filter == FileTypeFilter::Any {
                    resolved.type_filter = parsed;
                }
            } else {
                name_parts.push(token);
            }
        } else if let Some(pat) = token
            .strip_prefix("regex:")
            .or_else(|| token.strip_prefix("REGEX:"))
        {
            if !pat.is_empty() {
                resolved.name_regex = true;
                name_parts.push(pat.to_owned());
            }
        } else if matches!(lower.as_str(), "hoje" | "today")
            && resolved.date_filter == DateFilter::Any
        {
            resolved.date_filter = DateFilter::Today;
        } else if matches!(lower.as_str(), "7d" | "7dias")
            && resolved.date_filter == DateFilter::Any
        {
            resolved.date_filter = DateFilter::Last7Days;
        } else if matches!(lower.as_str(), "30d" | "30dias")
            && resolved.date_filter == DateFilter::Any
        {
            resolved.date_filter = DateFilter::Last30Days;
        } else if resolved.size_filter == SizeFilter::Any
            && let Some(size) = parse_size_token(&lower)
        {
            resolved.size_filter = size;
        } else {
            name_parts.push(token);
        }
    }

    resolved.name_query = name_parts.join(" ");
    if resolved.name_query.starts_with('.') {
        resolved.include_hidden = true;
    }
    resolved
}

fn score_search_candidate(
    target: &str,
    query: &str,
    query_lower: &str,
    pattern: &Pattern,
    matcher: &mut Matcher,
    buf: &mut Vec<char>,
) -> Option<u32> {
    let target_lower = target.to_lowercase();
    if target_lower == query_lower {
        let exact_case_bonus = u32::from(target == query) * 500;
        return Some(100_000 + exact_case_bonus);
    }
    if target_lower.starts_with(query_lower) {
        let len_penalty = target_lower
            .len()
            .saturating_sub(query_lower.len())
            .min(2000) as u32;
        return Some(60_000_u32.saturating_sub(len_penalty));
    }
    if let Some(pos) = target_lower.find(query_lower) {
        let pos_penalty = (pos.min(1000) * 10) as u32;
        let len_penalty = target_lower
            .len()
            .saturating_sub(query_lower.len())
            .min(2000) as u32;
        return Some(
            40_000_u32
                .saturating_sub(pos_penalty)
                .saturating_sub(len_penalty),
        );
    }

    buf.clear();
    let haystack = Utf32Str::new(target, buf);
    pattern.score(haystack, matcher)
}

enum NameMatcher {
    Any,
    Regex(Regex),
    Fuzzy {
        query_normalized: String,
        query_lower: String,
        query_has_sep: bool,
        matcher: Matcher,
        pattern: Pattern,
        buf: Vec<char>,
    },
}

impl NameMatcher {
    fn build(query: &str, use_regex: bool) -> Option<Self> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Some(Self::Any);
        }
        if use_regex {
            let re = RegexBuilder::new(trimmed).case_insensitive(true).build().ok()?;
            return Some(Self::Regex(re));
        }

        let query_has_sep = trimmed.contains('/') || trimmed.contains('\\');
        let query_normalized = if query_has_sep {
            trimmed.replace('\\', "/")
        } else {
            trimmed.to_owned()
        };
        let query_lower = query_normalized.to_lowercase();
        let matcher = Matcher::new(if query_has_sep {
            Config::DEFAULT.match_paths()
        } else {
            Config::DEFAULT
        });
        let pattern = Pattern::parse(&query_normalized, CaseMatching::Smart, Normalization::Smart);
        Some(Self::Fuzzy {
            query_normalized,
            query_lower,
            query_has_sep,
            matcher,
            pattern,
            buf: Vec::new(),
        })
    }

    fn score(&mut self, name: &str, rel_str: &str) -> Option<u32> {
        match self {
            Self::Any => Some(10_000),
            Self::Regex(re) => {
                if re.is_match(name) {
                    Some(80_000)
                } else if re.is_match(rel_str) {
                    Some(60_000)
                } else {
                    None
                }
            }
            Self::Fuzzy {
                query_normalized,
                query_lower,
                query_has_sep,
                matcher,
                pattern,
                buf,
            } => {
                let target = if *query_has_sep {
                    rel_str.replace('\\', "/")
                } else {
                    name.to_owned()
                };
                score_search_candidate(
                    &target,
                    query_normalized,
                    query_lower,
                    pattern,
                    matcher,
                    buf,
                )
            }
        }
    }
}

enum ContentMatcher {
    Plain {
        needle: String,
        needle_lower: String,
        case_sensitive: bool,
    },
    Regex(Regex),
}

impl ContentMatcher {
    fn build(query: &str, case_sensitive: bool, use_regex: bool) -> Option<Self> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return None;
        }
        if use_regex {
            let re = RegexBuilder::new(trimmed)
                .case_insensitive(!case_sensitive)
                .build()
                .ok()?;
            Some(Self::Regex(re))
        } else {
            Some(Self::Plain {
                needle: trimmed.to_owned(),
                needle_lower: trimmed.to_lowercase(),
                case_sensitive,
            })
        }
    }

    fn matches_line(&self, line: &str) -> bool {
        match self {
            Self::Plain {
                needle,
                needle_lower,
                case_sensitive,
            } => {
                if *case_sensitive {
                    line.contains(needle)
                } else {
                    line.to_lowercase().contains(needle_lower)
                }
            }
            Self::Regex(re) => re.is_match(line),
        }
    }
}

fn truncate_snippet(line: &str) -> CompactString {
    let trimmed = line.trim();
    if trimmed.chars().count() <= MAX_SNIPPET_CHARS {
        CompactString::from(trimmed)
    } else {
        let mut out: String = trimmed.chars().take(MAX_SNIPPET_CHARS - 1).collect();
        out.push('…');
        CompactString::from(out)
    }
}

/// Lê um arquivo em streaming procurando ocorrências de texto/regex.
/// Pula arquivos binários (byte NUL nos primeiros 8 KiB) e aborta ao cancelar.
fn search_file_content(
    path: &Path,
    matcher: &ContentMatcher,
    cancel: &CancellationToken,
) -> Option<Vec<ContentMatchSnippet>> {
    let mut file = open_shared_read(path).ok()?;

    let mut probe = [0u8; BINARY_PROBE_BYTES];
    let probe_len = file.read(&mut probe).ok()?;
    if probe_len == 0 || probe[..probe_len].contains(&0) {
        return None;
    }

    // Reabre ou reposiciona para ler do início em streaming.
    let file = open_shared_read(path).ok()?;
    let mut reader = BufReader::with_capacity(32 * 1024, file);
    let mut raw_line = Vec::with_capacity(256);
    let mut line_number = 0usize;
    let mut snippets = Vec::new();

    loop {
        if line_number.is_multiple_of(64) && cancel.is_cancelled() {
            return None;
        }
        raw_line.clear();
        let bytes_read = reader.read_until(b'\n', &mut raw_line).ok()?;
        if bytes_read == 0 {
            break;
        }
        line_number += 1;

        let text = String::from_utf8_lossy(&raw_line);
        if matcher.matches_line(text.as_ref()) {
            snippets.push(ContentMatchSnippet {
                line_number,
                line_text: truncate_snippet(text.as_ref()),
            });
            if snippets.len() >= MAX_SNIPPETS_PER_FILE {
                break;
            }
        }
    }

    if snippets.is_empty() {
        None
    } else {
        Some(snippets)
    }
}

pub(crate) fn matches_type_ext(ext: &str, filter: FileTypeFilter) -> bool {
    let lower = ext.trim_start_matches('.').to_lowercase();
    match filter {
        FileTypeFilter::Any => true,
        FileTypeFilter::Code => matches!(
            lower.as_str(),
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "java"
                | "kt"
                | "cs"
                | "rb"
                | "php"
                | "sh"
                | "bash"
                | "zsh"
                | "ps1"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "xml"
                | "html"
                | "css"
                | "scss"
                | "sql"
                | "lua"
                | "zig"
                | "dart"
        ),
        FileTypeFilter::Documents => matches!(
            lower.as_str(),
            "txt"
                | "md"
                | "markdown"
                | "pdf"
                | "doc"
                | "docx"
                | "odt"
                | "rtf"
                | "csv"
                | "tsv"
                | "xls"
                | "xlsx"
                | "ppt"
                | "pptx"
                | "tex"
                | "log"
        ),
        FileTypeFilter::Images => matches!(
            lower.as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "gif"
                | "bmp"
                | "svg"
                | "ico"
                | "tif"
                | "tiff"
                | "avif"
                | "heic"
        ),
        FileTypeFilter::Audio => matches!(
            lower.as_str(),
            "mp3" | "wav" | "flac" | "ogg" | "opus" | "m4a" | "aac" | "wma"
        ),
        FileTypeFilter::Video => matches!(
            lower.as_str(),
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "flv" | "m4v"
        ),
        FileTypeFilter::Archives => matches!(
            lower.as_str(),
            "zip" | "tar" | "gz" | "tgz" | "xz" | "bz2" | "7z" | "rar" | "zst"
        ),
    }
}

fn matches_type_with_magic(path: &Path, ext: &str, filter: FileTypeFilter) -> bool {
    if filter == FileTypeFilter::Any {
        return true;
    }
    if !ext.is_empty() && matches_type_ext(ext, filter) {
        return true;
    }
    // Fallback por magic bytes para arquivos sem extensão ou com extensão incomum.
    let Ok(mut file) = open_shared_read(path) else {
        return false;
    };
    let mut head = [0u8; 32];
    let Ok(n) = file.read(&mut head) else {
        return false;
    };
    let bytes = &head[..n];
    match filter {
        FileTypeFilter::Images => {
            bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                || bytes.starts_with(b"GIF87a")
                || bytes.starts_with(b"GIF89a")
                || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
        }
        FileTypeFilter::Audio => {
            bytes.starts_with(b"ID3")
                || bytes.starts_with(b"fLaC")
                || bytes.starts_with(b"OggS")
                || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE")
        }
        FileTypeFilter::Video => {
            bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3])
                || (bytes.len() >= 8 && &bytes[4..8] == b"ftyp")
        }
        FileTypeFilter::Archives => {
            bytes.starts_with(b"PK\x03\x04")
                || bytes.starts_with(&[0x1F, 0x8B])
                || bytes.starts_with(b"7z\xBC\xAF\x27\x1C")
                || bytes.starts_with(b"Rar!")
        }
        FileTypeFilter::Documents => bytes.starts_with(b"%PDF-"),
        FileTypeFilter::Code | FileTypeFilter::Any => false,
    }
}

fn advanced_search_blocking(
    root: &Path,
    query: &AdvancedSearchQuery,
    max_results: usize,
    cancel: &CancellationToken,
) -> Vec<SearchHit> {
    const MAX_DEPTH: usize = 12;
    const MAX_VISITED: usize = 250_000;

    let resolved = resolve_inline_query(query);
    let Some(mut name_matcher) = NameMatcher::build(&resolved.name_query, resolved.name_regex)
    else {
        return Vec::new();
    };

    let has_content_query = !resolved.content_query.is_empty();
    let content_matcher = if has_content_query {
        let Some(cm) = ContentMatcher::build(
            &resolved.content_query,
            resolved.content_case_sensitive,
            resolved.content_regex,
        ) else {
            return Vec::new();
        };
        Some(cm)
    } else {
        None
    };

    let now = SystemTime::now();
    let mut hits: Vec<(SearchHit, usize)> = Vec::new();

    // Busca em largura (BFS) para alcançar pastas de usuário imediatamente antes de árvores profundas.
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0));
    let mut visited: usize = 0;

    while let Some((dir, depth)) = queue.pop_front() {
        if cancel.is_cancelled() {
            return Vec::new();
        }
        if visited >= MAX_VISITED {
            break;
        }
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };

        let mut visible_subdirs: Vec<PathBuf> = Vec::new();
        let mut hidden_subdirs: Vec<PathBuf> = Vec::new();

        for entry_res in read_dir {
            visited += 1;
            if visited.is_multiple_of(256) && cancel.is_cancelled() {
                return Vec::new();
            }
            if visited >= MAX_VISITED {
                break;
            }
            let Ok(entry) = entry_res else {
                continue;
            };
            let os_name = entry.file_name();
            let name = os_name.to_string_lossy();
            let is_hidden = name.starts_with('.');
            if is_hidden && !resolved.include_hidden {
                continue;
            }
            if name == ".git" {
                continue;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let is_dir = file_type.is_dir();
            let path = entry.path();

            if is_dir && depth + 1 < MAX_DEPTH {
                if is_hidden {
                    hidden_subdirs.push(path.clone());
                } else {
                    visible_subdirs.push(path.clone());
                }
            }

            // Busca de conteúdo / tamanho / tipo de arquivo aplica apenas a arquivos regulares.
            if is_dir
                && (has_content_query
                    || resolved.type_filter != FileTypeFilter::Any
                    || !resolved.extensions.is_empty()
                    || resolved.size_filter != SizeFilter::Any)
            {
                continue;
            }

            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_compact = CompactString::from(rel.to_string_lossy().as_ref());

            // 1. Nome / caminho
            let Some(mut score) = name_matcher.score(name.as_ref(), rel_compact.as_str()) else {
                continue;
            };

            // 2. Extensão inline (`ext:...`)
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            if !resolved.extensions.is_empty()
                && !resolved
                    .extensions
                    .iter()
                    .any(|wanted| ext.eq_ignore_ascii_case(wanted))
            {
                continue;
            }

            // 3. Metadados de sistema (tamanho e data)
            let meta = entry.metadata().ok();
            let size_bytes = meta.as_ref().map(|m| m.len());
            let modified_at = meta.as_ref().and_then(|m| m.modified().ok());

            if resolved.size_filter != SizeFilter::Any {
                let Some(sz) = size_bytes else {
                    continue;
                };
                if !matches_size_filter(sz, resolved.size_filter, is_dir) {
                    continue;
                }
            }

            if resolved.date_filter != DateFilter::Any
                && !matches_date_filter(modified_at, resolved.date_filter, now)
            {
                continue;
            }

            // 4. Tipo / MIME por extensão + magic bytes
            if resolved.type_filter != FileTypeFilter::Any
                && !matches_type_with_magic(&path, ext, resolved.type_filter)
            {
                continue;
            }

            // 5. Conteúdo ("Pesquisar contendo...")
            let content_matches = if let Some(cm) = &content_matcher {
                if let Some(sz) = size_bytes
                    && sz > resolved.max_file_size_bytes
                {
                    continue;
                }
                let Some(snippets) = search_file_content(&path, cm, cancel) else {
                    continue;
                };
                score = score.saturating_add((snippets.len() as u32) * 500);
                snippets
            } else {
                Vec::new()
            };

            hits.push((
                SearchHit {
                    path,
                    relative_path: rel_compact,
                    name: CompactString::from(name.as_ref()),
                    is_dir,
                    score,
                    size_bytes,
                    modified_at,
                    content_matches,
                },
                depth + 1,
            ));
        }

        visible_subdirs.sort();
        for sub in visible_subdirs {
            queue.push_back((sub, depth + 1));
        }
        if resolved.include_hidden {
            hidden_subdirs.sort();
            for sub in hidden_subdirs {
                queue.push_back((sub, depth + 1));
            }
        }
    }

    hits.sort_by(|(a, da), (b, db)| {
        b.score
            .cmp(&a.score)
            .then_with(|| da.cmp(db))
            .then_with(|| a.relative_path.len().cmp(&b.relative_path.len()))
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
    hits.truncate(max_results);
    hits.into_iter().map(|(hit, _)| hit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::{EntryId, EntryKind, PermissionsMeta};

    fn make_entry(id: u64, name: &str) -> FileEntry {
        FileEntry {
            id: EntryId(id),
            name: CompactString::new(name),
            path: PathBuf::from(name),
            extension: None,
            kind: EntryKind::File,
            size_bytes: 10,
            created_at: None,
            modified_at: None,
            is_hidden: false,
            permissions: PermissionsMeta::PosixMode(0o644),
            child_count: None,
            mime_badge: None,
        }
    }

    #[test]
    fn fuzzy_ranks_exact_and_subsequence_matches() {
        let entries = vec![
            make_entry(1, "kernel-config.json"),
            make_entry(2, "main.rs"),
            make_entry(3, "README.md"),
        ];
        let matched = fuzzy_rank_entries(&entries, "kcfg");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "kernel-config.json");
    }

    #[test]
    fn deep_search_finds_nested_files() {
        let dir = std::env::temp_dir().join(format!("fm_index_search_{}", std::process::id()));
        let nested = dir.join("crates/core");
        std::fs::create_dir_all(&nested).expect("mkdir");
        std::fs::write(nested.join("vfs_provider.rs"), b"").expect("write");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let hits = rt.block_on(deep_search(&dir, "vfsprov", 20, CancellationToken::new()));
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "vfs_provider.rs");
    }

    #[test]
    fn deep_search_finds_exact_file_in_subfolder_without_hidden_cache_false_positives() {
        let dir = std::env::temp_dir().join(format!("fm_index_search_home_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = dir
            .join(".pub-cache/hosted/pub.dev/flutter_gallery_assets-1.0.2/lib/crane/destinations");
        let images = dir.join("Imagens");
        std::fs::create_dir_all(&cache).expect("mkdir cache");
        std::fs::create_dir_all(&images).expect("mkdir images");
        std::fs::write(cache.join("fly_1.jpg"), b"").expect("write cache file");
        std::fs::write(images.join("cabello1.jpg"), b"").expect("write target file");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");

        let hits = rt.block_on(deep_search(
            &dir,
            "cabello1.jpg",
            20,
            CancellationToken::new(),
        ));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "cabello1.jpg");
        assert!(hits[0].relative_path.contains("Imagens"));

        let hits_hidden = rt.block_on(deep_search_with_hidden(
            &dir,
            "cabello1.jpg",
            20,
            true,
            CancellationToken::new(),
        ));
        assert_eq!(hits_hidden.len(), 1);
        assert_eq!(hits_hidden[0].name, "cabello1.jpg");

        let path_hits = rt.block_on(deep_search(
            &dir,
            "imagens/cabello",
            20,
            CancellationToken::new(),
        ));
        assert_eq!(path_hits.len(), 1);
        assert_eq!(path_hits[0].name, "cabello1.jpg");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn advanced_search_content_and_binary_skip_and_regex() {
        let dir = std::env::temp_dir().join(format!("fm_index_adv_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).expect("mkdir");
        std::fs::write(
            dir.join("src/lib.rs"),
            b"pub fn hello() {\n    // TODO: refactor CancellationToken\n}\n",
        )
        .expect("write rs");
        std::fs::write(
            dir.join("src/notes.txt"),
            b"nothing relevant here\ncancellationtoken lowercase\n",
        )
        .expect("write txt");
        // Arquivo binário com byte NUL contendo a palavra-alvo (deve ser ignorado).
        std::fs::write(dir.join("src/app.bin"), b"\x00\x01CancellationToken\x00").expect("write bin");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");

        // 1. Busca case-sensitive por conteúdo
        let hits = rt.block_on(advanced_search(
            &dir,
            AdvancedSearchQuery {
                content_query: "CancellationToken".to_owned(),
                content_case_sensitive: true,
                ..Default::default()
            },
            20,
            CancellationToken::new(),
        ));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "lib.rs");
        assert_eq!(hits[0].content_matches.len(), 1);
        assert_eq!(hits[0].content_matches[0].line_number, 2);

        // 2. Busca por regex no nome + filtro de tipo Code
        let regex_hits = rt.block_on(advanced_search(
            &dir,
            AdvancedSearchQuery {
                name_query: r"^lib\.rs$".to_owned(),
                name_regex: true,
                type_filter: FileTypeFilter::Code,
                ..Default::default()
            },
            20,
            CancellationToken::new(),
        ));
        assert_eq!(regex_hits.len(), 1);
        assert_eq!(regex_hits[0].name, "lib.rs");

        // 3. Sintaxe inline `contendo:`
        let inline_hits = rt.block_on(advanced_search(
            &dir,
            AdvancedSearchQuery {
                name_query: r#"contendo:"lowercase""#.to_owned(),
                ..Default::default()
            },
            20,
            CancellationToken::new(),
        ));
        assert_eq!(inline_hits.len(), 1);
        assert_eq!(inline_hits[0].name, "notes.txt");

        // 4. Cancelamento imediato retorna vazio
        let cancel = CancellationToken::new();
        cancel.cancel();
        let cancelled_hits = rt.block_on(advanced_search(
            &dir,
            AdvancedSearchQuery {
                content_query: "CancellationToken".to_owned(),
                ..Default::default()
            },
            20,
            cancel,
        ));
        assert!(cancelled_hits.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
