use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::meta_cache::{DiskMetadataCache, mtime_nanos};

const MAX_COUNTED_PER_DIR: u32 = 10_000;

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
