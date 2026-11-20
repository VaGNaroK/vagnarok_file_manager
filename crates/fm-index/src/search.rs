use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use compact_str::CompactString;
use fm_core::FileEntry;
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
    let trimmed = query.trim().to_owned();
    if trimmed.is_empty() || cancel.is_cancelled() {
        return Vec::new();
    }

    let root_buf = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        deep_search_blocking(&root_buf, &trimmed, max_results, include_hidden, &cancel)
    })
    .await
    .unwrap_or_default()
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

fn deep_search_blocking(
    root: &Path,
    query: &str,
    max_results: usize,
    include_hidden: bool,
    cancel: &CancellationToken,
) -> Vec<SearchHit> {
    const MAX_DEPTH: usize = 12;
    const MAX_VISITED: usize = 250_000;

    let query_has_sep = query.contains('/') || query.contains('\\');
    let query_normalized = if query_has_sep {
        query.replace('\\', "/")
    } else {
        query.to_owned()
    };
    let query_lower = query_normalized.to_lowercase();

    let mut matcher = Matcher::new(if query_has_sep {
        Config::DEFAULT.match_paths()
    } else {
        Config::DEFAULT
    });
    let pattern = Pattern::parse(&query_normalized, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let mut hits: Vec<(SearchHit, usize)> = Vec::new();

    // Busca em largura (BFS) para alcançar pastas de usuário (ex.: Imagens, Documentos)
    // imediatamente antes de mergulhar em árvores profundas de projetos ou caches.
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
            if is_hidden && !include_hidden {
                continue;
            }
            if name == ".git" {
                continue;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            // Evita loops infinitos de symlinks durante a busca recursiva.
            let is_dir = file_type.is_dir();
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy();

            let match_target = if query_has_sep {
                rel_str.replace('\\', "/")
            } else {
                name.as_ref().to_owned()
            };

            if let Some(score) = score_search_candidate(
                &match_target,
                &query_normalized,
                &query_lower,
                &pattern,
                &mut matcher,
                &mut buf,
            ) {
                hits.push((
                    SearchHit {
                        path: path.clone(),
                        relative_path: CompactString::from(rel_str.as_ref()),
                        name: CompactString::from(name.as_ref()),
                        is_dir,
                        score,
                    },
                    depth + 1,
                ));
            }

            if is_dir && depth + 1 < MAX_DEPTH {
                if is_hidden {
                    hidden_subdirs.push(path);
                } else {
                    visible_subdirs.push(path);
                }
            }
        }

        visible_subdirs.sort();
        for sub in visible_subdirs {
            queue.push_back((sub, depth + 1));
        }
        if include_hidden {
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

        // Padrão (ocultos desativados): encontra Imagens/cabello1.jpg sem ruído de .pub-cache
        let hits = rt.block_on(deep_search(
            &dir,
            "cabello1.jpg",
            20,
            CancellationToken::new(),
        ));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "cabello1.jpg");
        assert!(hits[0].relative_path.contains("Imagens"));

        // Mesmo com ocultos ativados, fly_1.jpg não gera falso positivo para cabello1.jpg
        let hits_hidden = rt.block_on(deep_search_with_hidden(
            &dir,
            "cabello1.jpg",
            20,
            true,
            CancellationToken::new(),
        ));
        assert_eq!(hits_hidden.len(), 1);
        assert_eq!(hits_hidden[0].name, "cabello1.jpg");

        // Consulta com separador de caminho casa contra o caminho relativo
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
}
