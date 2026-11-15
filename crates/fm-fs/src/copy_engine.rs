//! Engine assíncrono e cancelável para cópia, movimentação, conflitos e progresso em streaming.

use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use compact_str::CompactString;
use fm_core::{ConflictPolicy, CopyMode, FsError, JobProgress, JobState};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::{map_io_err, platform};

const COPY_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const STREAM_BUF_BYTES: usize = 256 * 1024;

/// Executa transferência (cópia ou movimentação) de `sources` para `dest_dir` emitindo progresso.
pub async fn run_transfer_job(
    job_id: u64,
    mode: CopyMode,
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    policy: ConflictPolicy,
    cancel: CancellationToken,
    progress_tx: UnboundedSender<JobProgress>,
) -> Result<(), FsError> {
    tokio::task::spawn_blocking(move || {
        transfer_blocking(
            job_id,
            mode,
            &sources,
            &dest_dir,
            policy,
            &cancel,
            &progress_tx,
        )
    })
    .await
    .map_err(|err| FsError::Io {
        path: PathBuf::new(),
        message: format!("falha na tarefa de transferência: {err}"),
    })?
}

struct TransferContext<'a> {
    job_id: u64,
    mode: CopyMode,
    bytes_done: u64,
    bytes_total: u64,
    items_done: usize,
    items_total: usize,
    cancel: &'a CancellationToken,
    progress_tx: &'a UnboundedSender<JobProgress>,
}

impl TransferContext<'_> {
    fn emit(&self, current_item: &Path, state: JobState) {
        let name = current_item
            .file_name()
            .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
            .unwrap_or_default();
        let _ = self.progress_tx.send(JobProgress {
            job_id: self.job_id,
            mode: self.mode,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
            items_done: self.items_done,
            items_total: self.items_total,
            current_item: name,
            state,
        });
    }
}

fn transfer_blocking(
    job_id: u64,
    mode: CopyMode,
    sources: &[PathBuf],
    dest_dir: &Path,
    policy: ConflictPolicy,
    cancel: &CancellationToken,
    progress_tx: &UnboundedSender<JobProgress>,
) -> Result<(), FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let (bytes_total, items_total) = measure_sources(sources, cancel)?;
    let mut ctx = TransferContext {
        job_id,
        mode,
        bytes_done: 0,
        bytes_total,
        items_done: 0,
        items_total,
        cancel,
        progress_tx,
    };

    ctx.emit(dest_dir, JobState::Running);

    for src in sources {
        if ctx.cancel.is_cancelled() {
            ctx.emit(src, JobState::Cancelled);
            return Err(FsError::Cancelled);
        }

        if let Err(err) = transfer_top_level_item(src, dest_dir, policy, &mut ctx) {
            let state = if matches!(err, FsError::Cancelled) {
                JobState::Cancelled
            } else {
                JobState::Failed(err.to_string())
            };
            ctx.emit(src, state);
            return Err(err);
        }
    }

    ctx.emit(dest_dir, JobState::Completed);
    Ok(())
}

fn transfer_top_level_item(
    src: &Path,
    dest_dir: &Path,
    policy: ConflictPolicy,
    ctx: &mut TransferContext<'_>,
) -> Result<(), FsError> {
    let Some(file_name) = src.file_name() else {
        return Err(FsError::Io {
            path: src.to_path_buf(),
            message: "caminho de origem sem nome de arquivo".to_owned(),
        });
    };

    if src.is_dir() && dest_dir.starts_with(src) {
        return Err(FsError::Io {
            path: dest_dir.to_path_buf(),
            message: "não é possível copiar ou mover uma pasta para dentro dela mesma".to_owned(),
        });
    }

    // Caso o usuário cole no mesmo diretório no modo Recortar, evita apagar a própria origem.
    if ctx.mode == CopyMode::Move && src.parent() == Some(dest_dir) {
        let count = measure_single_tree(src, ctx.cancel)?;
        ctx.bytes_done = ctx.bytes_done.saturating_add(count.0);
        ctx.items_done += count.1;
        ctx.emit(src, JobState::Running);
        return Ok(());
    }

    let effective_policy = if ctx.mode == CopyMode::Copy && src.parent() == Some(dest_dir) {
        ConflictPolicy::RenameAuto
    } else {
        policy
    };

    let Some(target) = resolve_target_path(dest_dir, file_name, effective_policy)? else {
        return Ok(());
    };

    if ctx.mode == CopyMode::Move {
        match std::fs::rename(src, &target) {
            Ok(()) => {
                let (bytes, items) = measure_single_tree(&target, ctx.cancel)?;
                ctx.bytes_done = ctx.bytes_done.saturating_add(bytes);
                ctx.items_done += items;
                ctx.emit(&target, JobState::Running);
                return Ok(());
            }
            Err(err) if is_cross_device(&err) => {
                // Fallback para cópia entre volumes + remoção da origem.
                copy_tree(src, &target, effective_policy, ctx)?;
                remove_source_tree(src)?;
                return Ok(());
            }
            Err(err) => return Err(map_io_err(src, err)),
        }
    }

    copy_tree(src, &target, effective_policy, ctx)
}

fn copy_tree(
    src: &Path,
    dst: &Path,
    policy: ConflictPolicy,
    ctx: &mut TransferContext<'_>,
) -> Result<(), FsError> {
    if ctx.cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let meta = std::fs::symlink_metadata(src).map_err(|e| map_io_err(src, e))?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst).map_err(|e| map_io_err(dst, e))?;
        ctx.items_done += 1;
        ctx.emit(dst, JobState::Running);

        let entries = std::fs::read_dir(src).map_err(|e| map_io_err(src, e))?;
        for entry_res in entries {
            if ctx.cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            let entry = entry_res.map_err(|e| map_io_err(src, e))?;
            let child_src = entry.path();
            let child_name = entry.file_name();
            if let Some(child_dst) = resolve_target_path(dst, &child_name, policy)? {
                copy_tree(&child_src, &child_dst, policy, ctx)?;
            }
        }
        return Ok(());
    }

    copy_single_file(src, dst, ctx)?;
    ctx.items_done += 1;
    ctx.emit(dst, JobState::Running);
    Ok(())
}

fn copy_single_file(
    src: &Path,
    dst: &Path,
    ctx: &mut TransferContext<'_>,
) -> Result<(), FsError> {
    let mut in_opts = OpenOptions::new();
    in_opts.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        in_opts.share_mode(0x7);
    }
    let mut in_file = in_opts.open(src).map_err(|e| map_io_err(src, e))?;

    let mut out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)
        .map_err(|e| map_io_err(dst, e))?;

    #[cfg(target_os = "linux")]
    let mut use_zero_copy = true;
    #[cfg(not(target_os = "linux"))]
    let use_zero_copy = false;

    let mut scratch = Vec::new();
    let mut total_written = 0u64;

    loop {
        if ctx.cancel.is_cancelled() {
            drop(out_file);
            let _ = std::fs::remove_file(dst);
            return Err(FsError::Cancelled);
        }

        #[cfg(target_os = "linux")]
        if use_zero_copy {
            match platform::try_copy_file_range_chunk(&in_file, &out_file, COPY_CHUNK_BYTES) {
                Some(Ok(0)) => break,
                Some(Ok(n)) => {
                    total_written += n as u64;
                    ctx.bytes_done = ctx.bytes_done.saturating_add(n as u64);
                    ctx.emit(src, JobState::Running);
                    continue;
                }
                Some(Err(err)) => {
                    let _ = std::fs::remove_file(dst);
                    return Err(map_io_err(dst, err));
                }
                None => {
                    // Kernel não suportou copy_file_range neste par de FDs; continua em streaming.
                    use_zero_copy = false;
                }
            }
        }

        let _ = COPY_CHUNK_BYTES;
        if scratch.is_empty() {
            scratch.resize(STREAM_BUF_BYTES, 0);
        }
        let read_n = in_file.read(&mut scratch).map_err(|e| map_io_err(src, e))?;
        if read_n == 0 {
            break;
        }
        out_file
            .write_all(&scratch[..read_n])
            .map_err(|e| map_io_err(dst, e))?;
        total_written += read_n as u64;
        ctx.bytes_done = ctx.bytes_done.saturating_add(read_n as u64);
        if total_written.is_multiple_of((COPY_CHUNK_BYTES as u64).max(1)) {
            ctx.emit(src, JobState::Running);
        }
    }

    let _ = total_written;
    out_file.flush().map_err(|e| map_io_err(dst, e))
}

fn remove_source_tree(path: &Path) -> Result<(), FsError> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| map_io_err(path, e))?;
    if meta.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| map_io_err(path, e))
    } else {
        std::fs::remove_file(path).map_err(|e| map_io_err(path, e))
    }
}

fn resolve_target_path(
    dest_dir: &Path,
    file_name: &OsStr,
    policy: ConflictPolicy,
) -> Result<Option<PathBuf>, FsError> {
    let candidate = dest_dir.join(file_name);
    if !candidate.exists() {
        return Ok(Some(candidate));
    }

    match policy {
        ConflictPolicy::Overwrite => Ok(Some(candidate)),
        ConflictPolicy::Skip => Ok(None),
        ConflictPolicy::RenameAuto => {
            let path_name = Path::new(file_name);
            let stem = path_name
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| file_name.to_string_lossy().into_owned());
            let ext = path_name
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();

            for idx in 1..=999 {
                let mut os = OsString::new();
                os.push(format!("{stem} ({idx}){ext}"));
                let alt = dest_dir.join(os);
                if !alt.exists() {
                    return Ok(Some(alt));
                }
            }
            Err(FsError::Io {
                path: candidate,
                message: "muitos conflitos de nome no destino".to_owned(),
            })
        }
    }
}

fn measure_sources(sources: &[PathBuf], cancel: &CancellationToken) -> Result<(u64, usize), FsError> {
    let mut total_bytes = 0u64;
    let mut total_items = 0usize;
    for src in sources {
        let (b, i) = measure_single_tree(src, cancel)?;
        total_bytes = total_bytes.saturating_add(b);
        total_items += i;
    }
    Ok((total_bytes, total_items))
}

fn measure_single_tree(path: &Path, cancel: &CancellationToken) -> Result<(u64, usize), FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }
    let meta = std::fs::symlink_metadata(path).map_err(|e| map_io_err(path, e))?;
    if !meta.is_dir() {
        return Ok((meta.len(), 1));
    }

    let mut total_bytes = 0u64;
    let mut total_items = 1usize;
    let read_dir = std::fs::read_dir(path).map_err(|e| map_io_err(path, e))?;
    for entry in read_dir.flatten() {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let (b, i) = measure_single_tree(&entry.path(), cancel)?;
        total_bytes = total_bytes.saturating_add(b);
        total_items += i;
    }
    Ok((total_bytes, total_items))
}

fn is_cross_device(err: &std::io::Error) -> bool {
    err.kind() == ErrorKind::CrossesDevices || err.raw_os_error() == Some(18)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = std::env::temp_dir().join(format!("vagnarok_copy_{tag}_{nanos}"));
        std::fs::create_dir_all(&p).expect("create temp root");
        p
    }

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(fut)
    }

    #[test]
    fn copies_directory_tree_with_auto_rename_and_progress() {
        block_on(async {
            let root = temp_root("tree");
            let src_dir = root.join("src_folder");
            let dst_dir = root.join("dst");
            std::fs::create_dir_all(src_dir.join("nested")).expect("mkdir");
            std::fs::write(src_dir.join("a.txt"), b"hello").expect("write a");
            std::fs::write(src_dir.join("nested/b.txt"), b"world!").expect("write b");
            std::fs::create_dir_all(&dst_dir).expect("mkdir dst");
            std::fs::create_dir_all(dst_dir.join("src_folder")).expect("existing conflict");

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            run_transfer_job(
                1,
                CopyMode::Copy,
                vec![src_dir.clone()],
                dst_dir.clone(),
                ConflictPolicy::RenameAuto,
                CancellationToken::new(),
                tx,
            )
            .await
            .expect("copy succeeds");

            let copied = dst_dir.join("src_folder (1)");
            assert!(copied.join("a.txt").exists());
            assert_eq!(
                std::fs::read_to_string(copied.join("nested/b.txt")).expect("read b"),
                "world!"
            );

            let mut saw_completed = false;
            while let Ok(p) = rx.try_recv() {
                if p.state == JobState::Completed {
                    saw_completed = true;
                }
            }
            let _ = std::fs::remove_dir_all(&root);
            assert!(saw_completed);
        });
    }

    #[test]
    fn moves_file_cleanly_and_aborts_when_cancelled() {
        block_on(async {
            let root = temp_root("move_cancel");
            let src = root.join("report.txt");
            let dst_dir = root.join("target");
            std::fs::write(&src, b"data").expect("write");
            std::fs::create_dir_all(&dst_dir).expect("mkdir dst");

            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            run_transfer_job(
                2,
                CopyMode::Move,
                vec![src.clone()],
                dst_dir.clone(),
                ConflictPolicy::RenameAuto,
                CancellationToken::new(),
                tx,
            )
            .await
            .expect("move succeeds");

            assert!(!src.exists());
            assert!(dst_dir.join("report.txt").exists());

            let cancel = CancellationToken::new();
            cancel.cancel();
            let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
            let res = run_transfer_job(
                3,
                CopyMode::Copy,
                vec![dst_dir.join("report.txt")],
                root.clone(),
                ConflictPolicy::RenameAuto,
                cancel,
                tx2,
            )
            .await;

            let _ = std::fs::remove_dir_all(&root);
            assert!(matches!(res, Err(FsError::Cancelled)));
        });
    }
}
