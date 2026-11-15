use std::path::{Path, PathBuf};

use compact_str::CompactString;
use fm_core::FileEntry;
use ignore::WalkBuilder;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use tokio_util::sync::CancellationToken;

/// Resultado de busca profunda em subdiretórios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    pub relative_path: CompactString,
    pub name: CompactString,
    pub is_dir: bool,
    pub score: u32,
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

/// Executa busca fuzzy paralela/limitada em subpastas respeitando `.gitignore`.
pub async fn deep_search(
    root: &Path,
    query: &str,
    max_results: usize,
    cancel: CancellationToken,
) -> Vec<SearchHit> {
    let trimmed = query.trim().to_owned();
    if trimmed.is_empty() || cancel.is_cancelled() {
        return Vec::new();
    }

    let root_buf = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        deep_search_blocking(&root_buf, &trimmed, max_results, &cancel)
    })
    .await
    .unwrap_or_default()
}

fn deep_search_blocking(
    root: &Path,
    query: &str,
    max_results: usize,
    cancel: &CancellationToken,
) -> Vec<SearchHit> {
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let mut hits = Vec::new();

    let walker = WalkBuilder::new(root)
        .max_depth(Some(8))
        .hidden(false)
        .git_ignore(true)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();

    for (visited, item) in walker.enumerate() {
        if visited.is_multiple_of(256) && cancel.is_cancelled() {
            return Vec::new();
        }
        // Limite de segurança para pastas gigantes no protótipo.
        if visited > 40_000 {
            break;
        }

        let Ok(entry) = item else {
            continue;
        };
        if entry.depth() == 0 {
            continue;
        }

        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy();

        buf.clear();
        let haystack = Utf32Str::new(&rel_str, &mut buf);
        if let Some(score) = pattern.score(haystack, &mut matcher) {
            let name = entry.file_name().to_string_lossy();
            let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());
            hits.push(SearchHit {
                path: path.to_path_buf(),
                relative_path: CompactString::from(rel_str.as_ref()),
                name: CompactString::from(name.as_ref()),
                is_dir,
                score,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.relative_path.len().cmp(&b.relative_path.len()))
    });
    hits.truncate(max_results);
    hits
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
}
