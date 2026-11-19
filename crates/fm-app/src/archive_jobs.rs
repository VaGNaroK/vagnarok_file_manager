use std::path::PathBuf;

use fm_core::{
    ArchiveFormat, ArchiveJobKind, ArchiveJobProgress, CompressionLevel, ConflictPolicy,
    ExtractTargetMode, JobState, is_virtual_trash_path, suggest_archive_base_name,
};
use fm_fs::{run_compress_job, run_extract_job};
use iced::futures::SinkExt;
use iced::{Point, Size, Task, stream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::{App, Message};
use crate::bulk_rename::clamp_modal_pos;

pub const COMPRESS_MODAL_W: f32 = 540.0;
pub const COMPRESS_MODAL_EST_H: f32 = 430.0;

#[derive(Debug, Clone)]
pub enum ArchiveUiMsg {
    OpenCompressDialog,
    QuickAddSevenZ,
    Extract {
        archive_path: PathBuf,
        mode: ExtractTargetMode,
    },
    CloseModal,
    MinimizeModal,
    RestoreModal,
    RestoreJob(u64),
    DragStart,
    NameChanged(String),
    SetFormat(ArchiveFormat),
    SetLevel(CompressionLevel),
    SetConflict(ConflictPolicy),
    StartCompress,
    Progress(ArchiveJobProgress),
    CancelJob(u64),
}

#[derive(Debug, Clone)]
pub struct ArchiveJobRuntime {
    pub progress: ArchiveJobProgress,
    pub cancel: CancellationToken,
}

#[derive(Debug, Clone)]
pub struct CompressModalState {
    pub sources: Vec<PathBuf>,
    pub total_source_bytes: u64,
    pub dest_dir: PathBuf,
    pub archive_name: String,
    pub format: ArchiveFormat,
    pub level: CompressionLevel,
    pub conflict: ConflictPolicy,
    pub running_job_id: Option<u64>,
    pub minimized: bool,
    pub modal_offset: Option<(f32, f32)>,
    pub dragging_modal: Option<(Point, f32, f32)>,
}

impl CompressModalState {
    pub fn from_selection(sources: Vec<PathBuf>, total_source_bytes: u64, dest_dir: PathBuf) -> Self {
        let archive_name = suggest_archive_base_name(&sources, &dest_dir);
        Self {
            sources,
            total_source_bytes,
            dest_dir,
            archive_name,
            format: ArchiveFormat::SevenZ,
            level: CompressionLevel::Normal,
            conflict: ConflictPolicy::RenameAuto,
            running_job_id: None,
            minimized: false,
            modal_offset: None,
            dragging_modal: None,
        }
    }

    pub fn effective_level(&self) -> CompressionLevel {
        if self.format.supports_compression_level() {
            self.level
        } else {
            CompressionLevel::Store
        }
    }

    pub fn full_output_filename(&self) -> String {
        let trimmed = self.archive_name.trim();
        let base = if trimmed.is_empty() {
            "archive"
        } else {
            trimmed
        };
        let suffix = format!(".{}", self.format.extension());
        if base.to_ascii_lowercase().ends_with(&suffix) {
            base.to_owned()
        } else {
            format!("{base}{suffix}")
        }
    }
}

pub fn effective_compress_modal_pos(window_size: Size, modal: &CompressModalState) -> (f32, f32) {
    let w = COMPRESS_MODAL_W.min((window_size.width - 16.0).max(360.0));
    let h = COMPRESS_MODAL_EST_H.min((window_size.height - 16.0).max(320.0));
    let (raw_x, raw_y) = modal.modal_offset.unwrap_or_else(|| {
        (
            ((window_size.width - w) * 0.5).max(8.0),
            ((window_size.height - h) * 0.5).max(8.0),
        )
    });
    clamp_modal_pos(raw_x, raw_y, w, h, window_size)
}

pub fn clamp_compress_drag_pos(
    start: Point,
    start_x: f32,
    start_y: f32,
    current: Point,
    window_size: Size,
) -> (f32, f32) {
    let w = COMPRESS_MODAL_W.min((window_size.width - 16.0).max(360.0));
    let h = COMPRESS_MODAL_EST_H.min((window_size.height - 16.0).max(320.0));
    clamp_modal_pos(
        start_x + (current.x - start.x),
        start_y + (current.y - start.y),
        w,
        h,
        window_size,
    )
}

pub fn handle_archive_msg(app: &mut App, msg: ArchiveUiMsg) -> Task<Message> {
    match msg {
        ArchiveUiMsg::OpenCompressDialog => {
            app.context_menu = None;
            let Some((sources, total_bytes, dest_dir)) = collect_writable_selection(app) else {
                return Task::none();
            };
            app.compress_modal = Some(CompressModalState::from_selection(
                sources,
                total_bytes,
                dest_dir,
            ));
            Task::none()
        }
        ArchiveUiMsg::QuickAddSevenZ => {
            app.context_menu = None;
            let Some((sources, total_bytes, dest_dir)) = collect_writable_selection(app) else {
                return Task::none();
            };
            let base_name = suggest_archive_base_name(&sources, &dest_dir);
            let _ = total_bytes;
            dispatch_compress_job(
                app,
                sources,
                dest_dir,
                base_name,
                ArchiveFormat::SevenZ,
                CompressionLevel::Normal,
                ConflictPolicy::RenameAuto,
                false,
            )
        }
        ArchiveUiMsg::Extract { archive_path, mode } => {
            app.context_menu = None;
            let strings = app.strings();
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };
            if is_virtual_trash_path(&tab.current_path)
                || fm_fs::archive::split_archive_path(&tab.current_path).is_some()
            {
                app.status_notice = Some(strings.br_readonly_error.to_owned());
                return Task::none();
            }
            let dest_dir = archive_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| tab.current_path.clone());
            let title = archive_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            let job_id = app.next_job_id;
            app.next_job_id = app.next_job_id.wrapping_add(1).max(1);
            let cancel = CancellationToken::new();

            let initial = ArchiveJobProgress {
                job_id,
                kind: ArchiveJobKind::Extract { mode },
                title: title.into(),
                bytes_done: 0,
                bytes_total: 0,
                items_done: 0,
                items_total: 0,
                current_item: "...".into(),
                dest_dir: dest_dir.clone(),
                output_path: dest_dir,
                state: JobState::Running,
            };
            app.archive_jobs.insert(
                job_id,
                ArchiveJobRuntime {
                    progress: initial,
                    cancel: cancel.clone(),
                },
            );

            let worker_stream = stream::channel(32, async move |mut output| {
                let (tx, mut rx) = mpsc::unbounded_channel();
                tokio::spawn(async move {
                    let _ = run_extract_job(
                        job_id,
                        archive_path,
                        mode,
                        ConflictPolicy::RenameAuto,
                        cancel,
                        tx,
                    )
                    .await;
                });
                while let Some(progress) = rx.recv().await {
                    let _ = output
                        .send(Message::Archive(ArchiveUiMsg::Progress(progress)))
                        .await;
                }
            });
            Task::run(worker_stream, std::convert::identity)
        }
        ArchiveUiMsg::CloseModal => {
            if let Some(modal) = &mut app.compress_modal {
                if modal.running_job_id.is_some() {
                    // Se há compactação ativa, fechar pelo [x] apenas minimiza para evitar cancelamento acidental.
                    modal.minimized = true;
                    modal.dragging_modal = None;
                } else {
                    app.compress_modal = None;
                }
            }
            Task::none()
        }
        ArchiveUiMsg::MinimizeModal => {
            if let Some(modal) = &mut app.compress_modal {
                modal.minimized = true;
                modal.dragging_modal = None;
            }
            Task::none()
        }
        ArchiveUiMsg::RestoreModal => {
            if let Some(modal) = &mut app.compress_modal {
                modal.minimized = false;
            }
            Task::none()
        }
        ArchiveUiMsg::RestoreJob(job_id) => {
            if let Some(modal) = &mut app.compress_modal
                && modal.running_job_id == Some(job_id)
            {
                modal.minimized = false;
                return Task::none();
            }
            if let Some(runtime) = app.archive_jobs.get(&job_id) {
                let (format, level) = match runtime.progress.kind {
                    ArchiveJobKind::Compress { format, level } => (format, level),
                    ArchiveJobKind::Extract { .. } => {
                        (ArchiveFormat::Zip, CompressionLevel::Normal)
                    }
                };
                app.compress_modal = Some(CompressModalState {
                    sources: Vec::new(),
                    total_source_bytes: runtime.progress.bytes_total,
                    dest_dir: runtime.progress.dest_dir.clone(),
                    archive_name: runtime.progress.title.to_string(),
                    format,
                    level,
                    conflict: ConflictPolicy::RenameAuto,
                    running_job_id: Some(job_id),
                    minimized: false,
                    modal_offset: None,
                    dragging_modal: None,
                });
            }
            Task::none()
        }
        ArchiveUiMsg::DragStart => {
            let window_size = app.window_size;
            let cursor_pos = app.cursor_position;
            if let Some(modal) = &mut app.compress_modal {
                let (x, y) = effective_compress_modal_pos(window_size, modal);
                modal.modal_offset = Some((x, y));
                modal.dragging_modal = Some((cursor_pos, x, y));
            }
            Task::none()
        }
        ArchiveUiMsg::NameChanged(name) => {
            if let Some(modal) = &mut app.compress_modal
                && modal.running_job_id.is_none()
            {
                modal.archive_name = name;
            }
            Task::none()
        }
        ArchiveUiMsg::SetFormat(fmt) => {
            if let Some(modal) = &mut app.compress_modal
                && modal.running_job_id.is_none()
            {
                modal.format = fmt;
            }
            Task::none()
        }
        ArchiveUiMsg::SetLevel(lvl) => {
            if let Some(modal) = &mut app.compress_modal
                && modal.running_job_id.is_none()
            {
                modal.level = lvl;
            }
            Task::none()
        }
        ArchiveUiMsg::SetConflict(policy) => {
            if let Some(modal) = &mut app.compress_modal
                && modal.running_job_id.is_none()
            {
                modal.conflict = policy;
            }
            Task::none()
        }
        ArchiveUiMsg::StartCompress => {
            let Some((sources, dest_dir, archive_name, format, level, conflict)) = app
                .compress_modal
                .as_ref()
                .filter(|m| m.running_job_id.is_none() && !m.sources.is_empty())
                .map(|m| {
                    (
                        m.sources.clone(),
                        m.dest_dir.clone(),
                        m.archive_name.clone(),
                        m.format,
                        m.effective_level(),
                        m.conflict,
                    )
                })
            else {
                return Task::none();
            };

            let clean_name = if archive_name.trim().is_empty() {
                "archive".to_owned()
            } else {
                archive_name
            };
            dispatch_compress_job(
                app, sources, dest_dir, clean_name, format, level, conflict, true,
            )
        }
        ArchiveUiMsg::Progress(progress) => {
            let job_id = progress.job_id;
            match &progress.state {
                JobState::Running => {
                    if let Some(runtime) = app.archive_jobs.get_mut(&job_id) {
                        runtime.progress = progress;
                    }
                    Task::none()
                }
                JobState::Completed => {
                    app.archive_jobs.remove(&job_id);
                    if app
                        .compress_modal
                        .as_ref()
                        .is_some_and(|m| m.running_job_id == Some(job_id))
                    {
                        app.compress_modal = None;
                    }
                    let done_prefix = match progress.kind {
                        ArchiveJobKind::Compress { .. } => app.strings().arch_done_compress,
                        ArchiveJobKind::Extract { .. } => app.strings().arch_done_extract,
                    };
                    app.status_notice = Some(format!("{done_prefix}: {}", progress.title));
                    if app
                        .current_tab()
                        .is_some_and(|t| t.current_path == progress.dest_dir)
                    {
                        app.pending_select_path = Some(progress.output_path.clone());
                    }
                    app.reload_matching_dir_tabs(&progress.dest_dir)
                }
                JobState::Cancelled => {
                    app.archive_jobs.remove(&job_id);
                    if app
                        .compress_modal
                        .as_ref()
                        .is_some_and(|m| m.running_job_id == Some(job_id))
                    {
                        app.compress_modal = None;
                    }
                    app.status_notice = Some(app.strings().arch_cancelled.to_owned());
                    app.reload_matching_dir_tabs(&progress.dest_dir)
                }
                JobState::Failed(err_msg) => {
                    app.archive_jobs.remove(&job_id);
                    if app
                        .compress_modal
                        .as_ref()
                        .is_some_and(|m| m.running_job_id == Some(job_id))
                    {
                        app.compress_modal = None;
                    }
                    app.status_notice = Some(err_msg.clone());
                    app.reload_matching_dir_tabs(&progress.dest_dir)
                }
            }
        }
        ArchiveUiMsg::CancelJob(job_id) => {
            if let Some(runtime) = app.archive_jobs.get(&job_id) {
                runtime.cancel.cancel();
            }
            if app
                .compress_modal
                .as_ref()
                .is_some_and(|m| m.running_job_id == Some(job_id))
            {
                app.compress_modal = None;
            }
            Task::none()
        }
    }
}

fn collect_writable_selection(app: &mut App) -> Option<(Vec<PathBuf>, u64, PathBuf)> {
    let strings = app.strings();
    let tab = app.current_tab()?;
    if is_virtual_trash_path(&tab.current_path)
        || fm_fs::archive::split_archive_path(&tab.current_path).is_some()
    {
        app.status_notice = Some(strings.br_readonly_error.to_owned());
        return None;
    }
    let mut sources = Vec::new();
    let mut total_bytes = 0u64;
    for entry in &tab.visible_entries {
        if tab.selection.selected_ids.contains(&entry.id) {
            sources.push(entry.path.clone());
            if !entry.kind.is_dir_like() {
                total_bytes = total_bytes.saturating_add(entry.size_bytes);
            }
        }
    }
    if sources.is_empty() {
        return None;
    }
    Some((sources, total_bytes, tab.current_path.clone()))
}

#[allow(clippy::too_many_arguments)]
fn dispatch_compress_job(
    app: &mut App,
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    archive_name: String,
    format: ArchiveFormat,
    level: CompressionLevel,
    conflict: ConflictPolicy,
    attach_modal: bool,
) -> Task<Message> {
    let job_id = app.next_job_id;
    app.next_job_id = app.next_job_id.wrapping_add(1).max(1);
    let cancel = CancellationToken::new();

    let display_title = format!("{}.{}", archive_name.trim(), format.extension());
    let initial = ArchiveJobProgress {
        job_id,
        kind: ArchiveJobKind::Compress { format, level },
        title: display_title.into(),
        bytes_done: 0,
        bytes_total: 0,
        items_done: 0,
        items_total: sources.len(),
        current_item: "...".into(),
        dest_dir: dest_dir.clone(),
        output_path: dest_dir.clone(),
        state: JobState::Running,
    };

    app.archive_jobs.insert(
        job_id,
        ArchiveJobRuntime {
            progress: initial,
            cancel: cancel.clone(),
        },
    );

    if attach_modal && let Some(modal) = &mut app.compress_modal {
        modal.running_job_id = Some(job_id);
    }

    let worker_stream = stream::channel(32, async move |mut output| {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let _ = run_compress_job(
                job_id,
                sources,
                dest_dir,
                archive_name,
                format,
                level,
                conflict,
                cancel,
                tx,
            )
            .await;
        });
        while let Some(progress) = rx.recv().await {
            let _ = output
                .send(Message::Archive(ArchiveUiMsg::Progress(progress)))
                .await;
        }
    });

    Task::run(worker_stream, std::convert::identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_modal_tar_forces_store_and_preserves_selected_level_otherwise() {
        let mut modal = CompressModalState::from_selection(
            vec![PathBuf::from("/tmp/pasta")],
            1024,
            PathBuf::from("/tmp"),
        );
        assert_eq!(modal.archive_name, "pasta");
        assert_eq!(modal.format, ArchiveFormat::SevenZ);
        assert_eq!(modal.level, CompressionLevel::Normal);
        assert_eq!(modal.full_output_filename(), "pasta.7z");

        modal.level = CompressionLevel::Maximum;
        assert_eq!(modal.effective_level(), CompressionLevel::Maximum);

        modal.format = ArchiveFormat::Tar;
        assert_eq!(modal.effective_level(), CompressionLevel::Store);
        assert_eq!(modal.full_output_filename(), "pasta.tar");

        modal.format = ArchiveFormat::TarGz;
        assert_eq!(modal.effective_level(), CompressionLevel::Maximum);
        assert_eq!(modal.full_output_filename(), "pasta.tar.gz");
    }
}
