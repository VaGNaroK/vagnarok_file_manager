use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

const MAX_COUNTED_PER_DIR: u32 = 10_000;

/// Conta itens diretos (1 nível) para uma lista limitada de subpastas de forma cancelável.
pub async fn count_direct_children(
    dirs: Vec<PathBuf>,
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
            results.push((dir, count));
        }
        results
    })
    .await
    .unwrap_or_default()
}
