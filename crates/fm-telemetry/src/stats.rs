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
            if let Some(c) = &cache
                && mtime > 0
            {
                c.put_dir_count(&dir, mtime, count);
            }
            results.push((dir, count));
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
}
