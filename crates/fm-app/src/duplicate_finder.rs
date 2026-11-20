use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use fm_core::{
    DuplicateConfirmLevel, DuplicateGroup, DuplicateMinSizePreset, DuplicateScanEvent,
    DuplicateScanOptions, DuplicateScanProgress, KeepStrategy, VfsProvider, apply_keep_strategy,
    is_virtual_trash_path,
};
use fm_fs::LocalFs;
use fm_telemetry::run_duplicate_scan;
use iced::futures::SinkExt;
use iced::{Point, Size, Task, stream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::{App, Message};
use crate::bulk_rename::{clamp_modal_pos, clamp_modal_size, compute_dragged_modal_size};

pub const DUP_MODAL_DEFAULT_W: f32 = 860.0;
pub const DUP_MODAL_DEFAULT_H: f32 = 620.0;
pub const DUP_MODAL_MIN_W: f32 = 680.0;
pub const DUP_MODAL_MIN_H: f32 = 460.0;

#[derive(Debug, Clone)]
pub enum DuplicateFinderMsg {
    OpenForPath(PathBuf),
    OpenForCurrentDir,
    Close,
    Minimize,
    Restore,
    DragStart,
    ResizeStart,
    SetConfirmLevel(DuplicateConfirmLevel),
    SetMinSize(DuplicateMinSizePreset),
    ToggleSubfolders,
    ToggleHidden,
    ToggleIgnoreEmpty,
    ToggleIgnoreHardlinks,
    StartScan,
    CancelScan,
    CancelFromFooter,
    ScanEvent(u64, DuplicateScanEvent),
    ApplyKeepStrategy(KeepStrategy),
    ToggleGroupExpanded(u64),
    ToggleItemSelected(PathBuf),
    ListScrolled { offset_y: f32, viewport_height: f32 },
    RequestMoveToTrash,
    RequestPermanentDelete,
    CancelPermanentDeleteConfirm,
    ConfirmPermanentDelete,
    DeleteFinished(DuplicateDeleteOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateListRow {
    GroupHeader(usize),
    FileItem { group_idx: usize, item_idx: usize },
}

#[derive(Debug, Clone)]
pub struct DuplicateDeleteOutcome {
    pub deleted_paths: Vec<PathBuf>,
    pub stale_paths: Vec<PathBuf>,
    pub protected_groups: usize,
    pub error: Option<String>,
    pub affected_dirs: Vec<PathBuf>,
    pub was_trash: bool,
}

#[derive(Debug, Clone)]
pub struct DuplicateFinderState {
    pub root: PathBuf,
    pub options: DuplicateScanOptions,
    pub min_size_preset: DuplicateMinSizePreset,
    pub scanning: bool,
    pub has_completed_scan: bool,
    pub scan_id: u64,
    pub cancel: Option<CancellationToken>,
    pub progress: Option<DuplicateScanProgress>,
    pub groups: Vec<DuplicateGroup>,
    pub collapsed_groups: HashSet<u64>,
    pub flat_rows: Vec<DuplicateListRow>,
    pub scanned_files: usize,
    pub skipped_errors: usize,
    pub deleting: bool,
    pub confirm_perm_delete: bool,
    pub minimized: bool,
    pub scroll_y: f32,
    pub viewport_height: f32,
    pub modal_size: (f32, f32),
    pub modal_offset: Option<(f32, f32)>,
    pub dragging_modal: Option<(Point, f32, f32)>,
    pub resizing_modal: Option<(Point, f32, f32, f32, f32)>,
}

impl DuplicateFinderState {
    pub fn new(root: PathBuf, window_size: Size) -> Self {
        let size = clamp_modal_size(
            DUP_MODAL_DEFAULT_W,
            DUP_MODAL_DEFAULT_H,
            DUP_MODAL_MIN_W,
            DUP_MODAL_MIN_H,
            window_size,
        );
        Self {
            options: DuplicateScanOptions::for_root(root.clone()),
            root,
            min_size_preset: DuplicateMinSizePreset::OneKb,
            scanning: false,
            has_completed_scan: false,
            scan_id: 0,
            cancel: None,
            progress: None,
            groups: Vec::new(),
            collapsed_groups: HashSet::new(),
            flat_rows: Vec::new(),
            scanned_files: 0,
            skipped_errors: 0,
            deleting: false,
            confirm_perm_delete: false,
            minimized: false,
            scroll_y: 0.0,
            viewport_height: 300.0,
            modal_size: size,
            modal_offset: None,
            dragging_modal: None,
            resizing_modal: None,
        }
    }

    pub fn rebuild_flat_rows(&mut self) {
        self.flat_rows.clear();
        for (group_idx, group) in self.groups.iter().enumerate() {
            self.flat_rows
                .push(DuplicateListRow::GroupHeader(group_idx));
            if !self.collapsed_groups.contains(&group.group_id) {
                for item_idx in 0..group.items.len() {
                    self.flat_rows.push(DuplicateListRow::FileItem {
                        group_idx,
                        item_idx,
                    });
                }
            }
        }
    }

    pub fn total_selected_count(&self) -> usize {
        self.groups.iter().map(DuplicateGroup::selected_count).sum()
    }

    pub fn total_selected_reclaimable_bytes(&self) -> u64 {
        self.groups
            .iter()
            .map(DuplicateGroup::selected_reclaimable_bytes)
            .fold(0u64, u64::saturating_add)
    }

    pub fn total_reclaimable_bytes(&self) -> u64 {
        self.groups
            .iter()
            .map(DuplicateGroup::reclaimable_bytes)
            .fold(0u64, u64::saturating_add)
    }

    pub fn any_group_all_selected(&self) -> bool {
        self.groups.iter().any(DuplicateGroup::all_items_selected)
    }
}

pub fn effective_duplicate_modal_rect(
    window_size: Size,
    modal: &DuplicateFinderState,
) -> (f32, f32, f32, f32) {
    let (w, h) = clamp_modal_size(
        modal.modal_size.0,
        modal.modal_size.1,
        DUP_MODAL_MIN_W,
        DUP_MODAL_MIN_H,
        window_size,
    );
    let (raw_x, raw_y) = modal.modal_offset.unwrap_or_else(|| {
        (
            ((window_size.width - w) * 0.5).max(8.0),
            ((window_size.height - h) * 0.5).max(8.0),
        )
    });
    let (x, y) = clamp_modal_pos(raw_x, raw_y, w, h, window_size);
    (x, y, w, h)
}

pub fn clamp_duplicate_drag_pos(
    modal: &DuplicateFinderState,
    start: Point,
    start_x: f32,
    start_y: f32,
    current: Point,
    window_size: Size,
) -> (f32, f32) {
    let (w, h) = clamp_modal_size(
        modal.modal_size.0,
        modal.modal_size.1,
        DUP_MODAL_MIN_W,
        DUP_MODAL_MIN_H,
        window_size,
    );
    clamp_modal_pos(
        start_x + (current.x - start.x),
        start_y + (current.y - start.y),
        w,
        h,
        window_size,
    )
}

pub fn clamp_duplicate_resize_size(
    anchor_x: f32,
    anchor_y: f32,
    initial_w: f32,
    initial_h: f32,
    start: Point,
    current: Point,
    window_size: Size,
) -> (f32, f32) {
    compute_dragged_modal_size(
        (anchor_x, anchor_y),
        (initial_w, initial_h),
        (DUP_MODAL_MIN_W, DUP_MODAL_MIN_H),
        start,
        current,
        window_size,
    )
}

pub fn handle_duplicate_finder_msg(app: &mut App, msg: DuplicateFinderMsg) -> Task<Message> {
    match msg {
        DuplicateFinderMsg::OpenForCurrentDir => {
            app.context_menu = None;
            let Some(root) = app.current_tab().map(|t| t.current_path.clone()) else {
                return Task::none();
            };
            open_for_root(app, root)
        }
        DuplicateFinderMsg::OpenForPath(root) => {
            app.context_menu = None;
            open_for_root(app, root)
        }
        DuplicateFinderMsg::Close => {
            if let Some(modal) = &mut app.duplicate_finder {
                if modal.scanning {
                    // Se uma busca está rodando, fechar pelo [x] apenas minimiza para evitar perda de progresso.
                    modal.minimized = true;
                    modal.dragging_modal = None;
                    modal.resizing_modal = None;
                    return Task::none();
                }
                if let Some(cancel) = modal.cancel.take() {
                    cancel.cancel();
                }
            }
            app.duplicate_finder = None;
            Task::none()
        }
        DuplicateFinderMsg::Minimize => {
            if let Some(modal) = &mut app.duplicate_finder {
                modal.minimized = true;
                modal.dragging_modal = None;
                modal.resizing_modal = None;
            }
            Task::none()
        }
        DuplicateFinderMsg::Restore => {
            if let Some(modal) = &mut app.duplicate_finder {
                modal.minimized = false;
            }
            Task::none()
        }
        DuplicateFinderMsg::DragStart => {
            let window_size = app.window_size;
            let cursor_pos = app.cursor_position;
            if let Some(modal) = &mut app.duplicate_finder {
                let (x, y, _, _) = effective_duplicate_modal_rect(window_size, modal);
                modal.modal_offset = Some((x, y));
                modal.dragging_modal = Some((cursor_pos, x, y));
            }
            Task::none()
        }
        DuplicateFinderMsg::ResizeStart => {
            let window_size = app.window_size;
            let cursor_pos = app.cursor_position;
            if let Some(modal) = &mut app.duplicate_finder {
                let (x, y, w, h) = effective_duplicate_modal_rect(window_size, modal);
                modal.modal_offset = Some((x, y));
                modal.modal_size = (w, h);
                modal.resizing_modal = Some((cursor_pos, x, y, w, h));
            }
            Task::none()
        }
        DuplicateFinderMsg::SetConfirmLevel(level) => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.scanning
            {
                modal.options.confirm_level = level;
                modal.confirm_perm_delete = false;
            }
            Task::none()
        }
        DuplicateFinderMsg::SetMinSize(preset) => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.scanning
            {
                modal.min_size_preset = preset;
                modal.options.min_size_bytes = preset.bytes();
            }
            Task::none()
        }
        DuplicateFinderMsg::ToggleSubfolders => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.scanning
            {
                modal.options.recursive = !modal.options.recursive;
            }
            Task::none()
        }
        DuplicateFinderMsg::ToggleHidden => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.scanning
            {
                modal.options.include_hidden = !modal.options.include_hidden;
            }
            Task::none()
        }
        DuplicateFinderMsg::ToggleIgnoreEmpty => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.scanning
            {
                modal.options.ignore_empty_files = !modal.options.ignore_empty_files;
            }
            Task::none()
        }
        DuplicateFinderMsg::ToggleIgnoreHardlinks => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.scanning
            {
                modal.options.ignore_hardlinks = !modal.options.ignore_hardlinks;
            }
            Task::none()
        }
        DuplicateFinderMsg::StartScan => start_duplicate_scan(app),
        DuplicateFinderMsg::CancelScan => {
            if let Some(modal) = &mut app.duplicate_finder {
                if let Some(cancel) = modal.cancel.take() {
                    cancel.cancel();
                }
                modal.scanning = false;
                modal.progress = None;
            }
            app.status_notice = Some(app.strings().dup_cancelled.to_owned());
            Task::none()
        }
        DuplicateFinderMsg::CancelFromFooter => {
            if let Some(mut modal) = app.duplicate_finder.take() {
                let was_scanning = modal.scanning;
                if let Some(cancel) = modal.cancel.take() {
                    cancel.cancel();
                }
                if was_scanning {
                    app.status_notice = Some(app.strings().dup_cancelled.to_owned());
                }
            }
            Task::none()
        }
        DuplicateFinderMsg::ScanEvent(scan_id, event) => {
            let Some(modal) = &mut app.duplicate_finder else {
                return Task::none();
            };
            if modal.scan_id != scan_id {
                return Task::none();
            }
            match event {
                DuplicateScanEvent::Progress(prog) => {
                    modal.progress = Some(prog);
                    Task::none()
                }
                DuplicateScanEvent::Completed {
                    groups,
                    scanned_files,
                    skipped_errors,
                } => {
                    modal.scanning = false;
                    modal.has_completed_scan = true;
                    modal.cancel = None;
                    modal.progress = None;
                    modal.groups = groups;
                    modal.collapsed_groups.clear();
                    modal.scanned_files = scanned_files;
                    modal.skipped_errors = skipped_errors;
                    modal.confirm_perm_delete = false;
                    modal.scroll_y = 0.0;
                    modal.rebuild_flat_rows();
                    Task::none()
                }
                DuplicateScanEvent::Cancelled => {
                    modal.scanning = false;
                    modal.cancel = None;
                    modal.progress = None;
                    Task::none()
                }
                DuplicateScanEvent::Failed(err) => {
                    modal.scanning = false;
                    modal.cancel = None;
                    modal.progress = None;
                    app.status_notice = Some(err);
                    Task::none()
                }
            }
        }
        DuplicateFinderMsg::ApplyKeepStrategy(strategy) => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.deleting
            {
                apply_keep_strategy(&mut modal.groups, strategy);
                modal.confirm_perm_delete = false;
            }
            Task::none()
        }
        DuplicateFinderMsg::ToggleGroupExpanded(group_id) => {
            if let Some(modal) = &mut app.duplicate_finder {
                if !modal.collapsed_groups.remove(&group_id) {
                    modal.collapsed_groups.insert(group_id);
                }
                modal.rebuild_flat_rows();
            }
            Task::none()
        }
        DuplicateFinderMsg::ToggleItemSelected(target_path) => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.deleting
            {
                for group in &mut modal.groups {
                    for item in &mut group.items {
                        if item.path == target_path {
                            item.selected_for_delete = !item.selected_for_delete;
                            modal.confirm_perm_delete = false;
                            return Task::none();
                        }
                    }
                }
            }
            Task::none()
        }
        DuplicateFinderMsg::ListScrolled {
            offset_y,
            viewport_height,
        } => {
            if let Some(modal) = &mut app.duplicate_finder {
                modal.scroll_y = offset_y.max(0.0);
                if viewport_height > 0.0 {
                    modal.viewport_height = viewport_height;
                }
            }
            Task::none()
        }
        DuplicateFinderMsg::RequestMoveToTrash => dispatch_revalidated_delete(app, false),
        DuplicateFinderMsg::RequestPermanentDelete => {
            if let Some(modal) = &mut app.duplicate_finder
                && !modal.deleting
                && modal.options.confirm_level.allows_permanent_delete()
                && modal.total_selected_count() > 0
                && !modal.any_group_all_selected()
            {
                modal.confirm_perm_delete = true;
            }
            Task::none()
        }
        DuplicateFinderMsg::CancelPermanentDeleteConfirm => {
            if let Some(modal) = &mut app.duplicate_finder {
                modal.confirm_perm_delete = false;
            }
            Task::none()
        }
        DuplicateFinderMsg::ConfirmPermanentDelete => dispatch_revalidated_delete(app, true),
        DuplicateFinderMsg::DeleteFinished(outcome) => handle_delete_finished(app, outcome),
    }
}

fn open_for_root(app: &mut App, root: PathBuf) -> Task<Message> {
    let strings = app.strings();
    if is_virtual_trash_path(&root) || fm_fs::archive::split_archive_path(&root).is_some() {
        app.status_notice = Some(strings.dup_readonly_error.to_owned());
        return Task::none();
    }

    if let Some(existing) = &mut app.duplicate_finder
        && existing.root == root
    {
        existing.minimized = false;
        return Task::none();
    }

    if let Some(mut prev) = app.duplicate_finder.take()
        && let Some(cancel) = prev.cancel.take()
    {
        cancel.cancel();
    }

    app.duplicate_finder = Some(DuplicateFinderState::new(root, app.window_size));
    Task::none()
}

fn start_duplicate_scan(app: &mut App) -> Task<Message> {
    let Some(modal) = &mut app.duplicate_finder else {
        return Task::none();
    };
    if let Some(prev_cancel) = modal.cancel.take() {
        prev_cancel.cancel();
    }

    modal.scan_id = modal.scan_id.wrapping_add(1).max(1);
    let scan_id = modal.scan_id;
    let cancel = CancellationToken::new();
    modal.cancel = Some(cancel.clone());
    modal.scanning = true;
    modal.progress = None;
    modal.groups.clear();
    modal.collapsed_groups.clear();
    modal.flat_rows.clear();
    modal.confirm_perm_delete = false;
    modal.scroll_y = 0.0;

    let options = modal.options.clone();
    let cache = app.metadata_cache.clone();

    let worker_stream = stream::channel(32, async move |mut output| {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            run_duplicate_scan(options, cache, cancel, tx).await;
        });
        while let Some(ev) = rx.recv().await {
            let _ = output
                .send(Message::DuplicateFinder(DuplicateFinderMsg::ScanEvent(
                    scan_id, ev,
                )))
                .await;
        }
    });
    Task::run(worker_stream, std::convert::identity)
}

#[derive(Debug, Clone)]
struct GroupDeleteCandidate {
    selected: Vec<(PathBuf, u64, Option<i64>)>,
    unselected_paths: Vec<PathBuf>,
}

fn dispatch_revalidated_delete(app: &mut App, permanent: bool) -> Task<Message> {
    let Some(modal) = &mut app.duplicate_finder else {
        return Task::none();
    };
    if modal.deleting || modal.scanning {
        return Task::none();
    }
    if permanent && !modal.options.confirm_level.allows_permanent_delete() {
        return Task::none();
    }
    if modal.total_selected_count() == 0 || modal.any_group_all_selected() {
        return Task::none();
    }

    let plan: Vec<GroupDeleteCandidate> = modal
        .groups
        .iter()
        .filter(|g| g.selected_count() > 0)
        .map(|g| {
            let mut selected = Vec::new();
            let mut unselected_paths = Vec::new();
            for item in &g.items {
                if item.selected_for_delete {
                    selected.push((item.path.clone(), item.size_bytes, item.modified_secs));
                } else {
                    unselected_paths.push(item.path.clone());
                }
            }
            GroupDeleteCandidate {
                selected,
                unselected_paths,
            }
        })
        .collect();

    if plan.is_empty() {
        return Task::none();
    }

    modal.deleting = true;
    modal.confirm_perm_delete = false;
    let vfs = Arc::clone(&app.vfs);

    Task::perform(revalidate_and_delete(vfs, plan, permanent), |outcome| {
        Message::DuplicateFinder(DuplicateFinderMsg::DeleteFinished(outcome))
    })
}

/// Revalida cada `PathBuf` em disco antes da exclusão (Seção 5.3 do plano):
/// 1. Confirma que ao menos 1 cópia não marcada do mesmo grupo ainda existe como arquivo regular;
/// 2. Confirma que cada alvo marcado ainda existe, é arquivo regular (`!is_symlink()`),
///    mantém `size_bytes` e `modified_secs` compatíveis com o scan.
async fn revalidate_and_delete(
    vfs: Arc<LocalFs>,
    plan: Vec<GroupDeleteCandidate>,
    permanent: bool,
) -> DuplicateDeleteOutcome {
    let mut validated_paths: Vec<PathBuf> = Vec::new();
    let mut stale_paths: Vec<PathBuf> = Vec::new();
    let mut protected_groups = 0usize;
    let mut affected_dirs_set: HashSet<PathBuf> = HashSet::new();

    for group in plan {
        // Guarda da última cópia sobrevivente: pelo menos 1 cópia não marcada deve continuar viva.
        let mut has_live_keeper = false;
        for keep_path in &group.unselected_paths {
            if is_live_regular_file(keep_path).await {
                has_live_keeper = true;
                break;
            }
        }
        if !has_live_keeper {
            protected_groups += 1;
            continue;
        }

        for (path, expected_size, expected_mtime_secs) in group.selected {
            match tokio::fs::symlink_metadata(&path).await {
                Ok(meta) => {
                    let ft = meta.file_type();
                    let cur_mtime_secs = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64);
                    let mtime_ok = match (expected_mtime_secs, cur_mtime_secs) {
                        (Some(exp), Some(cur)) => exp == cur,
                        _ => true,
                    };
                    if ft.is_file() && !ft.is_symlink() && meta.len() == expected_size && mtime_ok {
                        if let Some(parent) = path.parent() {
                            affected_dirs_set.insert(parent.to_path_buf());
                        }
                        validated_paths.push(path);
                    } else {
                        stale_paths.push(path);
                    }
                }
                Err(_) => {
                    stale_paths.push(path);
                }
            }
        }
    }

    if validated_paths.is_empty() {
        return DuplicateDeleteOutcome {
            deleted_paths: Vec::new(),
            stale_paths,
            protected_groups,
            error: None,
            affected_dirs: Vec::new(),
            was_trash: !permanent,
        };
    }

    let op_res = if permanent {
        vfs.delete_permanent(&validated_paths).await
    } else {
        vfs.move_to_trash(&validated_paths).await
    };

    match op_res {
        Ok(()) => DuplicateDeleteOutcome {
            deleted_paths: validated_paths,
            stale_paths,
            protected_groups,
            error: None,
            affected_dirs: affected_dirs_set.into_iter().collect(),
            was_trash: !permanent,
        },
        Err(err) => DuplicateDeleteOutcome {
            deleted_paths: Vec::new(),
            stale_paths,
            protected_groups,
            error: Some(err.to_string()),
            affected_dirs: affected_dirs_set.into_iter().collect(),
            was_trash: !permanent,
        },
    }
}

async fn is_live_regular_file(path: &Path) -> bool {
    tokio::fs::symlink_metadata(path)
        .await
        .is_ok_and(|meta| meta.file_type().is_file() && !meta.file_type().is_symlink())
}

fn handle_delete_finished(app: &mut App, outcome: DuplicateDeleteOutcome) -> Task<Message> {
    let strings = app.strings();

    if let Some(modal) = &mut app.duplicate_finder {
        modal.deleting = false;
        modal.confirm_perm_delete = false;

        let removed: HashSet<&PathBuf> = outcome
            .deleted_paths
            .iter()
            .chain(outcome.stale_paths.iter())
            .collect();

        if !removed.is_empty() {
            for group in &mut modal.groups {
                group.items.retain(|item| !removed.contains(&item.path));
            }
            modal.groups.retain(|group| group.items.len() >= 2);
            modal.rebuild_flat_rows();
        }
    }

    if outcome.was_trash && !outcome.deleted_paths.is_empty() {
        app.push_undo(crate::app::UndoAction::Trash {
            paths: outcome.deleted_paths.clone(),
        });
    }

    if let Some(err) = outcome.error {
        app.status_notice = Some(err);
    } else if outcome.protected_groups > 0 {
        app.status_notice = Some(strings.dup_last_copy_protected.to_owned());
    } else if !outcome.stale_paths.is_empty() {
        app.status_notice = Some(strings.dup_stale_skipped.to_owned());
    } else if !outcome.deleted_paths.is_empty() {
        app.status_notice = Some(format!(
            "{} ({})",
            strings.dup_delete_done,
            outcome.deleted_paths.len()
        ));
    }

    let mut reload_tasks = Vec::new();
    for dir in &outcome.affected_dirs {
        reload_tasks.push(app.reload_matching_dir_tabs(dir));
    }
    if reload_tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(reload_tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn revalidation_blocks_group_when_unselected_keeper_disappeared_and_skips_modified_targets() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("vagnarok_dup_reval_{}_{nanos}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        // Grupo 1: keeper desapareceu externamente -> exclusão da cópia marcada deve ser bloqueada!
        let g1_missing_keeper = root.join("g1_keeper_deleted.bin");
        let g1_selected = root.join("g1_copy.bin");
        std::fs::write(&g1_selected, b"same-content").unwrap();

        // Grupo 2: keeper vivo, mas arquivo selecionado mudou de tamanho externamente -> stale!
        let g2_keeper = root.join("g2_keeper.bin");
        let g2_mutated = root.join("g2_mutated.bin");
        std::fs::write(&g2_keeper, b"12345").unwrap();
        std::fs::write(&g2_mutated, b"12345-changed").unwrap();

        let plan = vec![
            GroupDeleteCandidate {
                selected: vec![(g1_selected.clone(), 12, None)],
                unselected_paths: vec![g1_missing_keeper],
            },
            GroupDeleteCandidate {
                selected: vec![(g2_mutated.clone(), 5, None)],
                unselected_paths: vec![g2_keeper],
            },
        ];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let vfs = Arc::new(LocalFs::new());
        let outcome = rt.block_on(revalidate_and_delete(vfs, plan, true));

        assert_eq!(outcome.protected_groups, 1);
        assert_eq!(outcome.stale_paths, vec![g2_mutated.clone()]);
        assert!(outcome.deleted_paths.is_empty());
        // Nenhum arquivo foi apagado indevidamente!
        assert!(g1_selected.exists());
        assert!(g2_mutated.exists());

        let _ = std::fs::remove_dir_all(&root);
    }
}
