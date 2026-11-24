//! Worker assíncrono cancelável para análise de espaço em disco e descoberta de arquivos grandes.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use compact_str::CompactString;
use fm_core::{
    DiskCategory, DiskScanDirNode, DiskScanEvent, DiskScanFileEntry, DiskScanProgress,
    DiskScanResult,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const PROGRESS_INTERVAL: Duration = Duration::from_millis(80);
const MAX_TOP_FILES: usize = 600;
const MAX_DIRECT_FILES_PER_NODE: usize = 250;

/// Executa varredura recursiva nas raízes informadas em thread de background sem bloquear a UI.
pub async fn run_disk_scan(
    roots: Vec<PathBuf>,
    cancel: CancellationToken,
    tx: mpsc::UnboundedSender<DiskScanEvent>,
) {
    let _ = tokio::task::spawn_blocking(move || {
        scan_sync(roots, &cancel, &tx);
    })
    .await;
}

fn should_skip_virtual_linux_dir(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        matches!(
            path.to_str(),
            Some("/proc" | "/sys" | "/dev" | "/run" | "/proc/" | "/sys/" | "/dev/" | "/run/")
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        false
    }
}

fn display_node_name(path: &Path) -> CompactString {
    path.file_name()
        .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CompactString::from(path.to_string_lossy().as_ref()))
}

fn scan_sync(
    roots: Vec<PathBuf>,
    cancel: &CancellationToken,
    tx: &mpsc::UnboundedSender<DiskScanEvent>,
) {
    let started = Instant::now();
    let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
    let mut nodes: Vec<DiskScanDirNode> = Vec::new();
    let mut root_node_ids: Vec<usize> = Vec::new();
    let mut top_files: Vec<DiskScanFileEntry> = Vec::new();
    let mut cat_totals = [0u64; 6];
    let mut total_bytes: u64 = 0;
    let mut total_files: u64 = 0;
    let mut skipped_errors: usize = 0;

    // Pilha explícita (evita overflow em árvores muito profundas): (parent_node_id, dir_path)
    let mut stack: Vec<(Option<usize>, PathBuf)> = Vec::new();
    let mut clean_roots: Vec<PathBuf> = Vec::new();
    for r in roots {
        if !clean_roots.contains(&r) {
            clean_roots.push(r);
        }
    }
    for root in clean_roots.iter().rev() {
        stack.push((None, root.clone()));
    }

    while let Some((parent_id, dir_path)) = stack.pop() {
        if cancel.is_cancelled() {
            let _ = tx.send(DiskScanEvent::Cancelled);
            return;
        }
        if should_skip_virtual_linux_dir(&dir_path) {
            continue;
        }

        let node_id = nodes.len();
        nodes.push(DiskScanDirNode {
            id: node_id,
            parent_id,
            name: display_node_name(&dir_path),
            path: dir_path.clone(),
            total_bytes: 0,
            file_count: 0,
            children: Vec::new(),
            direct_files: Vec::new(),
        });
        if let Some(pid) = parent_id {
            nodes[pid].children.push(node_id);
        } else {
            root_node_ids.push(node_id);
        }

        let read_dir = match std::fs::read_dir(&dir_path) {
            Ok(rd) => rd,
            Err(_) => {
                skipped_errors += 1;
                continue;
            }
        };

        let mut subdirs: Vec<PathBuf> = Vec::new();
        for entry_res in read_dir {
            if cancel.is_cancelled() {
                let _ = tx.send(DiskScanEvent::Cancelled);
                return;
            }
            let Ok(entry) = entry_res else {
                skipped_errors += 1;
                continue;
            };
            let entry_path = entry.path();
            let Ok(ft) = entry.file_type() else {
                skipped_errors += 1;
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                if !should_skip_virtual_linux_dir(&entry_path) {
                    subdirs.push(entry_path);
                }
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                skipped_errors += 1;
                continue;
            };
            let size = meta.len();
            let category = DiskCategory::classify(&entry_path);
            let cat_idx = category_index(category);
            cat_totals[cat_idx] = cat_totals[cat_idx].saturating_add(size);
            total_bytes = total_bytes.saturating_add(size);
            total_files = total_files.saturating_add(1);

            nodes[node_id].total_bytes = nodes[node_id].total_bytes.saturating_add(size);
            nodes[node_id].file_count = nodes[node_id].file_count.saturating_add(1);

            let file_entry = DiskScanFileEntry {
                name: CompactString::from(entry.file_name().to_string_lossy().as_ref()),
                path: entry_path.clone(),
                size_bytes: size,
                category,
            };

            nodes[node_id].direct_files.push(file_entry.clone());
            if nodes[node_id].direct_files.len() > MAX_DIRECT_FILES_PER_NODE * 2 {
                nodes[node_id]
                    .direct_files
                    .sort_unstable_by_key(|a| std::cmp::Reverse(a.size_bytes));
                nodes[node_id]
                    .direct_files
                    .truncate(MAX_DIRECT_FILES_PER_NODE);
            }

            top_files.push(file_entry);
            if top_files.len() > MAX_TOP_FILES * 2 {
                top_files.sort_unstable_by_key(|a| std::cmp::Reverse(a.size_bytes));
                top_files.truncate(MAX_TOP_FILES);
            }

            if last_emit.elapsed() >= PROGRESS_INTERVAL {
                last_emit = Instant::now();
                let _ = tx.send(DiskScanEvent::Progress(DiskScanProgress {
                    current_path: entry_path,
                    files_scanned: total_files,
                    bytes_scanned: total_bytes,
                    elapsed_secs: started.elapsed().as_secs(),
                }));
            }
        }

        if last_emit.elapsed() >= PROGRESS_INTERVAL {
            last_emit = Instant::now();
            let _ = tx.send(DiskScanEvent::Progress(DiskScanProgress {
                current_path: dir_path,
                files_scanned: total_files,
                bytes_scanned: total_bytes,
                elapsed_secs: started.elapsed().as_secs(),
            }));
        }

        for sub in subdirs.into_iter().rev() {
            stack.push((Some(node_id), sub));
        }
    }

    if cancel.is_cancelled() {
        let _ = tx.send(DiskScanEvent::Cancelled);
        return;
    }

    // Agrega tamanhos bottom-up (os nós foram inseridos em pré-ordem, então iterar em reverso garante filhos antes dos pais).
    for idx in (0..nodes.len()).rev() {
        if let Some(pid) = nodes[idx].parent_id {
            let child_bytes = nodes[idx].total_bytes;
            let child_files = nodes[idx].file_count;
            nodes[pid].total_bytes = nodes[pid].total_bytes.saturating_add(child_bytes);
            nodes[pid].file_count = nodes[pid].file_count.saturating_add(child_files);
        }
    }

    // Ordena filhos e arquivos diretos de cada nó do maior para o menor.
    let child_sizes: Vec<u64> = nodes.iter().map(|n| n.total_bytes).collect();
    for node in &mut nodes {
        node.children
            .sort_unstable_by(|&a, &b| child_sizes[b].cmp(&child_sizes[a]));
        node.direct_files
            .sort_unstable_by_key(|a| std::cmp::Reverse(a.size_bytes));
        node.direct_files.truncate(MAX_DIRECT_FILES_PER_NODE);
    }
    root_node_ids.sort_unstable_by(|&a, &b| child_sizes[b].cmp(&child_sizes[a]));

    top_files.sort_unstable_by_key(|a| std::cmp::Reverse(a.size_bytes));
    top_files.truncate(MAX_TOP_FILES);

    let category_totals = [
        (DiskCategory::IsoAndDisks, cat_totals[0]),
        (DiskCategory::BuildAndCache, cat_totals[1]),
        (DiskCategory::Archives, cat_totals[2]),
        (DiskCategory::Media, cat_totals[3]),
        (DiskCategory::Documents, cat_totals[4]),
        (DiskCategory::Other, cat_totals[5]),
    ];

    let _ = tx.send(DiskScanEvent::Completed(DiskScanResult {
        roots: clean_roots,
        root_node_ids,
        nodes,
        top_files,
        category_totals,
        total_bytes,
        total_files,
        elapsed_ms: started.elapsed().as_millis() as u64,
        skipped_errors,
    }));
}

const fn category_index(cat: DiskCategory) -> usize {
    match cat {
        DiskCategory::IsoAndDisks => 0,
        DiskCategory::BuildAndCache => 1,
        DiskCategory::Archives => 2,
        DiskCategory::Media => 3,
        DiskCategory::Documents => 4,
        DiskCategory::Other => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_multiple_roots_aggregates_sizes_and_ignores_symlinks() {
        let base = std::env::temp_dir().join(format!("vagnarok_disk_scan_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root_a = base.join("folder_a");
        let sub_a = root_a.join("sub");
        let root_b = base.join("folder_b");
        std::fs::create_dir_all(&sub_a).expect("mkdir sub_a");
        std::fs::create_dir_all(&root_b).expect("mkdir root_b");

        std::fs::write(root_a.join("big.iso"), vec![1u8; 4096]).expect("write big");
        std::fs::write(sub_a.join("nested.bin"), vec![2u8; 2048]).expect("write nested");
        std::fs::write(root_b.join("small.txt"), vec![3u8; 512]).expect("write small");

        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(root_a.join("big.iso"), root_b.join("link.iso"));
        }

        let (tx, mut rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        scan_sync(vec![root_a.clone(), root_b.clone()], &cancel, &tx);

        let mut completed = None;
        while let Ok(ev) = rx.try_recv() {
            if let DiskScanEvent::Completed(res) = ev {
                completed = Some(res);
            }
        }
        let res = completed.expect("scan deve completar");
        assert_eq!(res.total_files, 3);
        assert_eq!(res.total_bytes, 4096 + 2048 + 512);
        assert_eq!(res.root_node_ids.len(), 2);
        assert_eq!(res.nodes[res.root_node_ids[0]].total_bytes, 4096 + 2048);
        assert_eq!(res.top_files[0].size_bytes, 4096);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn cancels_immediately_when_token_is_triggered() {
        let base =
            std::env::temp_dir().join(format!("vagnarok_disk_cancel_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("mkdir base");

        let (tx, mut rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        cancel.cancel();
        scan_sync(vec![base.clone()], &cancel, &tx);

        let ev = rx.try_recv().expect("evento de cancelamento");
        assert!(matches!(ev, DiskScanEvent::Cancelled));

        let _ = std::fs::remove_dir_all(&base);
    }
}
