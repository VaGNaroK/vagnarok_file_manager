use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fm_core::RecursiveSize;
use tokio_util::sync::CancellationToken;

use crate::meta_cache::{DiskMetadataCache, mtime_nanos};

const MAX_COUNTED_PER_DIR: u32 = 10_000;
const MAX_RECURSIVE_ENTRIES: usize = 25_000;
const MAX_RECURSIVE_DEPTH: u32 = 32;
const MAX_RECURSIVE_DURATION: Duration = Duration::from_millis(1_500);
const MAX_PROPERTIES_RECURSIVE_ENTRIES: usize = 500_000;
const MAX_PROPERTIES_RECURSIVE_DEPTH: u32 = 64;
const MAX_PROPERTIES_RECURSIVE_DURATION: Duration = Duration::from_secs(12);

/// Conta itens diretos (1 nível) para uma lista limitada de subpastas de forma cancelável.
pub async fn count_direct_children(
    dirs: Vec<PathBuf>,
    cancel: CancellationToken,
) -> Vec<(PathBuf, u32)> {
    count_direct_children_cached(dirs, None, cancel).await
}

/// Conta itens diretos usando cache persistente `redb` quando `mtime` do diretório não mudou.
pub async fn count_direct_children_cached(
    dirs: Vec<PathBuf>,
    cache: Option<Arc<DiskMetadataCache>>,
    cancel: CancellationToken,
) -> Vec<(PathBuf, u32)> {
    if dirs.is_empty() || cancel.is_cancelled() {
        return Vec::new();
    }

    tokio::task::spawn_blocking(move || {
        let mut results = Vec::with_capacity(dirs.len());
        let mut to_cache: Vec<(PathBuf, u128, u32)> = Vec::new();
        for dir in dirs {
            if cancel.is_cancelled() {
                break;
            }
            let mtime = mtime_nanos(&dir);
            if let Some(c) = &cache
                && mtime > 0
                && let Some(cached) = c.get_dir_count(&dir, mtime)
            {
                results.push((dir, cached));
                continue;
            }
            let Ok(iter) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut count = 0u32;
            for entry in iter {
                if count >= MAX_COUNTED_PER_DIR {
                    break;
                }
                if entry.is_ok() {
                    count += 1;
                }
            }
            if cache.is_some() && mtime > 0 {
                to_cache.push((dir.clone(), mtime, count));
            }
            results.push((dir, count));
        }
        if let Some(c) = &cache
            && !to_cache.is_empty()
        {
            c.put_dir_counts_batch(&to_cache);
        }
        results
    })
    .await
    .unwrap_or_default()
}

pub(crate) fn is_pseudo_fs_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s == "/proc"
        || s.starts_with("/proc/")
        || s == "/sys"
        || s.starts_with("/sys/")
        || s == "/dev"
        || s.starts_with("/dev/")
        || s == "/run"
        || s.starts_with("/run/")
}

fn is_pseudo_or_root_path(path: &Path) -> bool {
    path == Path::new("/") || is_pseudo_fs_path(path)
}

/// Calcula tamanho recursivo da subárvore de `dir_path` em background com limite, cache e cancelamento.
pub async fn compute_folder_size_cached(
    dir_path: PathBuf,
    cache: Option<Arc<DiskMetadataCache>>,
    cancel: CancellationToken,
) -> Option<RecursiveSize> {
    if cancel.is_cancelled() || is_pseudo_or_root_path(&dir_path) {
        return None;
    }

    tokio::task::spawn_blocking(move || {
        let mtime = mtime_nanos(&dir_path);
        if let Some(c) = &cache
            && mtime > 0
            && let Some(cached) = c.get_dir_size(&dir_path, mtime)
        {
            return Some(cached);
        }

        let start = Instant::now();
        let mut total_bytes = 0u64;
        let mut item_count = 0u64;
        let mut hidden_count = 0u64;
        let mut visited = 0usize;
        let mut truncated = false;
        let mut stack = vec![(dir_path.clone(), 0u32)];

        while let Some((dir, depth)) = stack.pop() {
            if cancel.is_cancelled() {
                return None;
            }
            let Ok(iter) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in iter {
                visited += 1;
                if visited.is_multiple_of(256)
                    && (cancel.is_cancelled() || start.elapsed() >= MAX_RECURSIVE_DURATION)
                {
                    if cancel.is_cancelled() {
                        return None;
                    }
                    truncated = true;
                    break;
                }
                if visited >= MAX_RECURSIVE_ENTRIES {
                    truncated = true;
                    break;
                }
                let Ok(entry) = entry else {
                    continue;
                };
                let Ok(ft) = entry.file_type() else {
                    continue;
                };
                if ft.is_symlink() {
                    continue;
                }
                item_count += 1;
                if is_hidden_dir_entry(&entry) {
                    hidden_count += 1;
                }
                if ft.is_dir() {
                    if depth < MAX_RECURSIVE_DEPTH {
                        stack.push((entry.path(), depth + 1));
                    } else {
                        truncated = true;
                    }
                } else if ft.is_file()
                    && let Ok(meta) = entry.metadata()
                {
                    total_bytes = total_bytes.saturating_add(meta.len());
                }
            }
            if truncated {
                break;
            }
        }

        let res = RecursiveSize {
            total_bytes,
            item_count,
            hidden_count,
            truncated,
        };
        if let Some(c) = &cache
            && mtime > 0
        {
            c.put_dir_size(&dir_path, mtime, res);
        }
        Some(res)
    })
    .await
    .ok()
    .flatten()
}

/// Calcula tamanho total recursivo e contagem de itens para um ou mais caminhos selecionados
/// (arquivos, pastas ou combinação de ambos) com limites adequados à janela de Propriedades.
pub async fn compute_paths_size_cached(
    paths: Vec<PathBuf>,
    cache: Option<Arc<DiskMetadataCache>>,
    cancel: CancellationToken,
) -> Option<RecursiveSize> {
    if paths.is_empty() || cancel.is_cancelled() {
        return None;
    }
    if paths.len() == 1 && is_pseudo_or_root_path(&paths[0]) {
        return None;
    }

    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let single_root = paths.len() == 1;
        let mut total_bytes = 0u64;
        let mut item_count = 0u64;
        let mut hidden_count = 0u64;
        let mut visited = 0usize;
        let mut truncated = false;

        for root_path in paths {
            if cancel.is_cancelled() {
                return None;
            }
            if is_pseudo_or_root_path(&root_path) {
                continue;
            }
            let Ok(meta) = std::fs::symlink_metadata(&root_path) else {
                continue;
            };
            let ft = meta.file_type();
            if ft.is_symlink() {
                item_count = item_count.saturating_add(1);
                if is_hidden_path(&root_path, &meta) {
                    hidden_count = hidden_count.saturating_add(1);
                }
                continue;
            }
            if ft.is_file() {
                total_bytes = total_bytes.saturating_add(meta.len());
                item_count = item_count.saturating_add(1);
                if is_hidden_path(&root_path, &meta) {
                    hidden_count = hidden_count.saturating_add(1);
                }
                continue;
            }
            if !ft.is_dir() {
                continue;
            }

            let mtime = mtime_nanos(&root_path);
            if let Some(c) = &cache
                && mtime > 0
                && let Some(cached) = c.get_dir_size(&root_path, mtime)
                && !cached.truncated
            {
                total_bytes = total_bytes.saturating_add(cached.total_bytes);
                item_count = item_count.saturating_add(cached.item_count);
                hidden_count = hidden_count.saturating_add(cached.hidden_count);
                continue;
            }

            let mut dir_bytes = 0u64;
            let mut dir_items = 0u64;
            let mut dir_hidden = 0u64;
            let mut dir_truncated = false;
            let mut stack = vec![(root_path.clone(), 0u32)];

            while let Some((dir, depth)) = stack.pop() {
                if cancel.is_cancelled() {
                    return None;
                }
                let Ok(iter) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in iter {
                    visited += 1;
                    if visited.is_multiple_of(256)
                        && (cancel.is_cancelled()
                            || start.elapsed() >= MAX_PROPERTIES_RECURSIVE_DURATION)
                    {
                        if cancel.is_cancelled() {
                            return None;
                        }
                        dir_truncated = true;
                        break;
                    }
                    if visited >= MAX_PROPERTIES_RECURSIVE_ENTRIES {
                        dir_truncated = true;
                        break;
                    }
                    let Ok(entry) = entry else {
                        continue;
                    };
                    let Ok(entry_ft) = entry.file_type() else {
                        continue;
                    };
                    if entry_ft.is_symlink() {
                        continue;
                    }
                    let child_path = entry.path();
                    if is_pseudo_fs_path(&child_path) {
                        continue;
                    }
                    dir_items = dir_items.saturating_add(1);
                    if is_hidden_dir_entry(&entry) {
                        dir_hidden = dir_hidden.saturating_add(1);
                    }
                    if entry_ft.is_dir() {
                        if depth < MAX_PROPERTIES_RECURSIVE_DEPTH {
                            stack.push((child_path, depth + 1));
                        } else {
                            dir_truncated = true;
                        }
                    } else if entry_ft.is_file()
                        && let Ok(entry_meta) = entry.metadata()
                    {
                        dir_bytes = dir_bytes.saturating_add(entry_meta.len());
                    }
                }
                if dir_truncated {
                    break;
                }
            }

            if let Some(c) = &cache
                && mtime > 0
                && (!dir_truncated || single_root)
            {
                c.put_dir_size(
                    &root_path,
                    mtime,
                    RecursiveSize {
                        total_bytes: dir_bytes,
                        item_count: dir_items,
                        hidden_count: dir_hidden,
                        truncated: dir_truncated,
                    },
                );
            }

            total_bytes = total_bytes.saturating_add(dir_bytes);
            item_count = item_count.saturating_add(dir_items);
            hidden_count = hidden_count.saturating_add(dir_hidden);
            if dir_truncated {
                truncated = true;
                break;
            }
        }

        Some(RecursiveSize {
            total_bytes,
            item_count,
            hidden_count,
            truncated,
        })
    })
    .await
    .ok()
    .flatten()
}

fn is_hidden_path(path: &Path, meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        let _ = path;
        (meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = meta;
        path.file_name()
            .is_some_and(|n| n.as_encoded_bytes().starts_with(b"."))
    }
}

fn is_hidden_dir_entry(entry: &std::fs::DirEntry) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        entry
            .metadata()
            .is_ok_and(|m| (m.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0)
    }
    #[cfg(not(windows))]
    {
        entry.file_name().as_encoded_bytes().starts_with(b".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn computes_recursive_folder_size_and_respects_cancellation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("vagnarok_rsize_{nanos}"));
        let nested = root.join("sub");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(root.join("a.bin"), vec![1u8; 100]).expect("write a");
        std::fs::write(nested.join("b.bin"), vec![2u8; 250]).expect("write b");

        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let computed = rt
            .block_on(compute_folder_size_cached(
                root.clone(),
                None,
                CancellationToken::new(),
            ))
            .expect("recursive size computed");
        assert_eq!(computed.total_bytes, 350);
        assert_eq!(computed.item_count, 3);
        assert_eq!(computed.hidden_count, 0);
        assert!(!computed.truncated);

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert!(
            rt.block_on(compute_folder_size_cached(root.clone(), None, cancelled))
                .is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn computes_multi_paths_recursive_size_and_respects_cancellation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("vagnarok_multi_rsize_{nanos}"));
        let dir_a = root.join("dir_a");
        let dir_a_nested = dir_a.join("deep");
        let dir_b = root.join("dir_b_empty");
        let top_file = root.join("top.bin");
        std::fs::create_dir_all(&dir_a_nested).expect("create dir_a");
        std::fs::create_dir_all(&dir_b).expect("create dir_b");
        std::fs::write(dir_a.join("f1.bin"), vec![1u8; 120]).expect("write f1");
        std::fs::write(dir_a_nested.join("f2.bin"), vec![2u8; 280]).expect("write f2");
        std::fs::write(&top_file, vec![3u8; 50]).expect("write top");

        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let computed = rt
            .block_on(compute_paths_size_cached(
                vec![dir_a.clone(), dir_b.clone(), top_file.clone()],
                None,
                CancellationToken::new(),
            ))
            .expect("multi-path recursive size computed");
        assert_eq!(computed.total_bytes, 450);
        assert_eq!(computed.item_count, 4);
        assert!(!computed.truncated);

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert!(
            rt.block_on(compute_paths_size_cached(
                vec![dir_a, dir_b, top_file],
                None,
                cancelled
            ))
            .is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
