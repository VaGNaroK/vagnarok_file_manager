use std::collections::HashSet;
use std::path::PathBuf;

use fm_core::{
    DiskMinSizeFilter, DiskScanEvent, DiskScanProgress, DiskScanResult, VfsProvider,
    is_virtual_trash_path,
};
use fm_fs::LocalFs;
use fm_telemetry::run_disk_scan;
use iced::futures::SinkExt;
use iced::{Point, Size, Task, stream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::{App, Message};
use crate::bulk_rename::{clamp_modal_pos, clamp_modal_size, compute_dragged_modal_size};

pub const DISK_MODAL_DEFAULT_W: f32 = 960.0;
pub const DISK_MODAL_DEFAULT_H: f32 = 640.0;
pub const DISK_MODAL_MIN_W: f32 = 700.0;
pub const DISK_MODAL_MIN_H: f32 = 480.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiskListMode {
    #[default]
    LargestFiles,
    Folders,
}

#[derive(Debug, Clone)]
pub enum DiskAnalyzerMsg {
    OpenForPaths(Vec<PathBuf>),
    OpenForCurrentDir,
    Close,
    Minimize,
    Restore,
    DragStart,
    ResizeStart,
    StartScan,
    ScanEvent(u64, DiskScanEvent),
    DrillIntoNode(usize),
    DrillRoot,
    SetMinSize(DiskMinSizeFilter),
    SetListMode(DiskListMode),
    SetSearchQuery(String),
    ToggleItemSelected(PathBuf),
    ClearSelection,
    ListScrolled { offset_y: f32, viewport_height: f32 },
    OpenItemLocation(PathBuf),
    RequestMoveToTrash,
    RequestPermanentDelete,
    CancelPermanentDeleteConfirm,
    ConfirmPermanentDelete,
    DeleteFinished(DiskDeleteOutcome),
}

#[derive(Debug, Clone)]
pub struct DiskDeleteOutcome {
    pub deleted_paths: Vec<PathBuf>,
    pub error: Option<String>,
    pub affected_dirs: Vec<PathBuf>,
    pub was_trash: bool,
}

#[derive(Debug, Clone)]
pub struct DiskAnalyzerRow {
    pub path: PathBuf,
    pub name: String,
    pub parent_display: String,
    pub size_bytes: u64,
    pub node_id: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DiskAnalyzerState {
    pub roots: Vec<PathBuf>,
    pub scanning: bool,
    pub scan_id: u64,
    pub cancel: Option<CancellationToken>,
    pub progress: Option<DiskScanProgress>,
    pub result: Option<DiskScanResult>,
    pub current_node_id: Option<usize>,
    pub min_size: DiskMinSizeFilter,
    pub list_mode: DiskListMode,
    pub search_query: String,
    pub selected_paths: HashSet<PathBuf>,
    pub visible_rows: Vec<DiskAnalyzerRow>,
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

impl DiskAnalyzerState {
    pub fn new(roots: Vec<PathBuf>, window_size: Size) -> Self {
        let size = clamp_modal_size(
            DISK_MODAL_DEFAULT_W,
            DISK_MODAL_DEFAULT_H,
            DISK_MODAL_MIN_W,
            DISK_MODAL_MIN_H,
            window_size,
        );
        Self {
            roots,
            scanning: false,
            scan_id: 0,
            cancel: None,
            progress: None,
            result: None,
            current_node_id: None,
            min_size: DiskMinSizeFilter::All,
            list_mode: DiskListMode::LargestFiles,
            search_query: String::new(),
            selected_paths: HashSet::new(),
            visible_rows: Vec::new(),
            deleting: false,
            confirm_perm_delete: false,
            minimized: false,
            scroll_y: 0.0,
            viewport_height: 360.0,
            modal_size: size,
            modal_offset: None,
            dragging_modal: None,
            resizing_modal: None,
        }
    }

    pub fn breadcrumb_nodes(&self) -> Vec<(Option<usize>, String)> {
        let Some(res) = &self.result else {
            return Vec::new();
        };
        let mut chain = Vec::new();
        let mut cur = self.current_node_id;
        while let Some(nid) = cur {
            if let Some(node) = res.nodes.get(nid) {
                chain.push((Some(nid), node.name.to_string()));
                cur = node.parent_id;
            } else {
                break;
            }
        }
        chain.reverse();
        chain
    }

    pub fn current_scope_bytes(&self) -> u64 {
        let Some(res) = &self.result else {
            return 0;
        };
        if let Some(nid) = self.current_node_id
            && let Some(node) = res.nodes.get(nid)
        {
            return node.total_bytes;
        }
        res.total_bytes
    }

    pub fn selected_bytes(&self) -> u64 {
        let mut total = 0u64;
        for path in &self.selected_paths {
            if let Some(row) = self.visible_rows.iter().find(|r| &r.path == path) {
                total = total.saturating_add(row.size_bytes);
            } else if let Some(res) = &self.result {
                if let Some(f) = res.top_files.iter().find(|f| &f.path == path) {
                    total = total.saturating_add(f.size_bytes);
                } else if let Some(n) = res.nodes.iter().find(|n| &n.path == path) {
                    total = total.saturating_add(n.total_bytes);
                }
            }
        }
        total
    }

    pub fn rebuild_rows(&mut self) {
        self.visible_rows.clear();
        let Some(res) = &self.result else {
            return;
        };
        let min_bytes = self.min_size.min_bytes();
        let q = self.search_query.trim().to_ascii_lowercase();

        match self.list_mode {
            DiskListMode::LargestFiles => {
                let source: Vec<&fm_core::DiskScanFileEntry> =
                    if let Some(nid) = self.current_node_id {
                        let scope_path = res.nodes.get(nid).map(|n| &n.path);
                        let mut list: Vec<&fm_core::DiskScanFileEntry> = res
                            .top_files
                            .iter()
                            .filter(|f| scope_path.is_some_and(|sp| f.path.starts_with(sp)))
                            .collect();
                        if list.is_empty()
                            && let Some(node) = res.nodes.get(nid)
                        {
                            list = node.direct_files.iter().collect();
                        }
                        list
                    } else {
                        res.top_files.iter().collect()
                    };

                for f in source {
                    if f.size_bytes < min_bytes {
                        continue;
                    }
                    let parent_display = f
                        .path
                        .parent()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if !q.is_empty()
                        && !f.name.to_ascii_lowercase().contains(&q)
                        && !parent_display.to_ascii_lowercase().contains(&q)
                    {
                        continue;
                    }
                    self.visible_rows.push(DiskAnalyzerRow {
                        path: f.path.clone(),
                        name: f.name.to_string(),
                        parent_display,
                        size_bytes: f.size_bytes,
                        node_id: None,
                    });
                }
            }
            DiskListMode::Folders => {
                let ids: &[usize] = if let Some(nid) = self.current_node_id {
                    res.nodes
                        .get(nid)
                        .map(|n| n.children.as_slice())
                        .unwrap_or(&[])
                } else if res.root_node_ids.len() == 1 {
                    res.nodes
                        .get(res.root_node_ids[0])
                        .map(|n| n.children.as_slice())
                        .unwrap_or(&[])
                } else {
                    res.root_node_ids.as_slice()
                };

                for &cid in ids {
                    let Some(node) = res.nodes.get(cid) else {
                        continue;
                    };
                    if node.total_bytes < min_bytes {
                        continue;
                    }
                    let parent_display = node
                        .path
                        .parent()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if !q.is_empty()
                        && !node.name.to_ascii_lowercase().contains(&q)
                        && !parent_display.to_ascii_lowercase().contains(&q)
                    {
                        continue;
                    }
                    self.visible_rows.push(DiskAnalyzerRow {
                        path: node.path.clone(),
                        name: node.name.to_string(),
                        parent_display,
                        size_bytes: node.total_bytes,
                        node_id: Some(cid),
                    });
                }
            }
        }
    }

    pub fn remove_deleted_paths(&mut self, deleted: &[PathBuf]) {
        let deleted_set: HashSet<&PathBuf> = deleted.iter().collect();
        for p in deleted {
            self.selected_paths.remove(p);
        }
        if let Some(res) = &mut self.result {
            let mut removed_bytes_total = 0u64;
            let mut removed_files_total = 0u64;

            res.top_files.retain(|f| !deleted_set.contains(&f.path));

            for node in &mut res.nodes {
                node.direct_files.retain(|f| {
                    let drop = deleted
                        .iter()
                        .any(|d| f.path == *d || f.path.starts_with(d));
                    if drop {
                        removed_bytes_total = removed_bytes_total.saturating_add(f.size_bytes);
                        removed_files_total = removed_files_total.saturating_add(1);
                    }
                    !drop
                });
            }

            // Atualiza nós afetados diretamente ou ancestrais.
            for idx in 0..res.nodes.len() {
                if deleted_set.contains(&res.nodes[idx].path) {
                    let sub_bytes = res.nodes[idx].total_bytes;
                    let sub_files = res.nodes[idx].file_count;
                    res.nodes[idx].total_bytes = 0;
                    res.nodes[idx].file_count = 0;
                    let mut pid = res.nodes[idx].parent_id;
                    while let Some(p) = pid {
                        res.nodes[p].total_bytes =
                            res.nodes[p].total_bytes.saturating_sub(sub_bytes);
                        res.nodes[p].file_count = res.nodes[p].file_count.saturating_sub(sub_files);
                        pid = res.nodes[p].parent_id;
                    }
                    res.total_bytes = res.total_bytes.saturating_sub(sub_bytes);
                    res.total_files = res.total_files.saturating_sub(sub_files);
                }
            }
            if removed_bytes_total > 0 {
                res.total_bytes = res.total_bytes.saturating_sub(removed_bytes_total);
                res.total_files = res.total_files.saturating_sub(removed_files_total);
            }
        }
        self.rebuild_rows();
    }
}

pub fn clamp_disk_drag_pos(
    start_x: f32,
    start_y: f32,
    start: Point,
    pos: Point,
    modal_size: (f32, f32),
    window_size: Size,
) -> (f32, f32) {
    let (w, h) = clamp_modal_size(
        modal_size.0,
        modal_size.1,
        DISK_MODAL_MIN_W,
        DISK_MODAL_MIN_H,
        window_size,
    );
    clamp_modal_pos(
        start_x + (pos.x - start.x),
        start_y + (pos.y - start.y),
        w,
        h,
        window_size,
    )
}

pub fn clamp_disk_resize_size(
    anchor_x: f32,
    anchor_y: f32,
    initial_w: f32,
    initial_h: f32,
    start: Point,
    pos: Point,
    window_size: Size,
) -> (f32, f32) {
    compute_dragged_modal_size(
        (anchor_x, anchor_y),
        (initial_w, initial_h),
        (DISK_MODAL_MIN_W, DISK_MODAL_MIN_H),
        start,
        pos,
        window_size,
    )
}

pub fn handle_disk_analyzer_msg(app: &mut App, msg: DiskAnalyzerMsg) -> Task<Message> {
    match msg {
        DiskAnalyzerMsg::OpenForPaths(paths) => {
            let valid: Vec<PathBuf> = paths
                .into_iter()
                .filter(|p| {
                    !is_virtual_trash_path(p)
                        && !fm_core::is_virtual_recent_path(p)
                        && fm_fs::archive::split_archive_path(p).is_none()
                })
                .collect();
            if valid.is_empty() {
                app.status_notice = Some(
                    "Analisador de espaço disponível apenas para pastas e discos locais."
                        .to_owned(),
                );
                return Task::none();
            }
            open_and_start_scan(app, valid)
        }
        DiskAnalyzerMsg::OpenForCurrentDir => {
            let Some(tab) = app.current_tab() else {
                return Task::none();
            };
            let path = tab.current_path.clone();
            handle_disk_analyzer_msg(app, DiskAnalyzerMsg::OpenForPaths(vec![path]))
        }
        DiskAnalyzerMsg::Close => {
            if let Some(mut modal) = app.disk_analyzer.take()
                && let Some(cancel) = modal.cancel.take()
            {
                cancel.cancel();
            }
            Task::none()
        }
        DiskAnalyzerMsg::Minimize => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.minimized = true;
                modal.dragging_modal = None;
                modal.resizing_modal = None;
            }
            Task::none()
        }
        DiskAnalyzerMsg::Restore => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.minimized = false;
            }
            Task::none()
        }
        DiskAnalyzerMsg::DragStart => {
            if let Some(modal) = &mut app.disk_analyzer {
                let (x, y, _, _) = crate::bulk_rename::effective_modal_rect(
                    modal.modal_size,
                    modal.modal_offset,
                    DISK_MODAL_MIN_W,
                    DISK_MODAL_MIN_H,
                    app.window_size,
                );
                modal.modal_offset = Some((x, y));
                modal.dragging_modal = Some((app.cursor_position, x, y));
            }
            Task::none()
        }
        DiskAnalyzerMsg::ResizeStart => {
            if let Some(modal) = &mut app.disk_analyzer {
                let (x, y, w, h) = crate::bulk_rename::effective_modal_rect(
                    modal.modal_size,
                    modal.modal_offset,
                    DISK_MODAL_MIN_W,
                    DISK_MODAL_MIN_H,
                    app.window_size,
                );
                modal.modal_offset = Some((x, y));
                modal.resizing_modal = Some((app.cursor_position, x, y, w, h));
            }
            Task::none()
        }
        DiskAnalyzerMsg::StartScan => {
            let Some(roots) = app.disk_analyzer.as_ref().map(|m| m.roots.clone()) else {
                return Task::none();
            };
            open_and_start_scan(app, roots)
        }
        DiskAnalyzerMsg::ScanEvent(scan_id, event) => {
            let Some(modal) = &mut app.disk_analyzer else {
                return Task::none();
            };
            if modal.scan_id != scan_id {
                return Task::none();
            }
            match event {
                DiskScanEvent::Progress(prog) => {
                    modal.progress = Some(prog);
                }
                DiskScanEvent::Completed(res) => {
                    modal.scanning = false;
                    modal.cancel = None;
                    modal.current_node_id =
                        (res.root_node_ids.len() == 1).then(|| res.root_node_ids[0]);
                    modal.result = Some(res);
                    modal.selected_paths.clear();
                    modal.scroll_y = 0.0;
                    modal.rebuild_rows();
                }
                DiskScanEvent::Cancelled => {
                    modal.scanning = false;
                    modal.cancel = None;
                }
            }
            Task::none()
        }
        DiskAnalyzerMsg::DrillIntoNode(node_id) => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.current_node_id = Some(node_id);
                modal.scroll_y = 0.0;
                modal.rebuild_rows();
            }
            Task::none()
        }
        DiskAnalyzerMsg::DrillRoot => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.current_node_id = modal
                    .result
                    .as_ref()
                    .and_then(|r| (r.root_node_ids.len() == 1).then(|| r.root_node_ids[0]));
                modal.scroll_y = 0.0;
                modal.rebuild_rows();
            }
            Task::none()
        }
        DiskAnalyzerMsg::SetMinSize(min_size) => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.min_size = min_size;
                modal.scroll_y = 0.0;
                modal.rebuild_rows();
            }
            Task::none()
        }
        DiskAnalyzerMsg::SetListMode(mode) => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.list_mode = mode;
                modal.scroll_y = 0.0;
                modal.rebuild_rows();
            }
            Task::none()
        }
        DiskAnalyzerMsg::SetSearchQuery(q) => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.search_query = q;
                modal.scroll_y = 0.0;
                modal.rebuild_rows();
            }
            Task::none()
        }
        DiskAnalyzerMsg::ToggleItemSelected(path) => {
            if let Some(modal) = &mut app.disk_analyzer {
                if !modal.selected_paths.remove(&path) {
                    modal.selected_paths.insert(path);
                }
                modal.confirm_perm_delete = false;
            }
            Task::none()
        }
        DiskAnalyzerMsg::ClearSelection => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.selected_paths.clear();
                modal.confirm_perm_delete = false;
            }
            Task::none()
        }
        DiskAnalyzerMsg::ListScrolled {
            offset_y,
            viewport_height,
        } => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.scroll_y = offset_y.max(0.0);
                if viewport_height > 1.0 {
                    modal.viewport_height = viewport_height;
                }
            }
            Task::none()
        }
        DiskAnalyzerMsg::OpenItemLocation(path) => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.minimized = true;
            }
            app.update(Message::OpenEntryParentLocation(path))
        }
        DiskAnalyzerMsg::RequestMoveToTrash => spawn_delete(app, true),
        DiskAnalyzerMsg::RequestPermanentDelete => {
            if let Some(modal) = &mut app.disk_analyzer
                && !modal.selected_paths.is_empty()
                && !modal.deleting
            {
                modal.confirm_perm_delete = true;
            }
            Task::none()
        }
        DiskAnalyzerMsg::CancelPermanentDeleteConfirm => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.confirm_perm_delete = false;
            }
            Task::none()
        }
        DiskAnalyzerMsg::ConfirmPermanentDelete => spawn_delete(app, false),
        DiskAnalyzerMsg::DeleteFinished(outcome) => {
            if let Some(modal) = &mut app.disk_analyzer {
                modal.deleting = false;
                modal.confirm_perm_delete = false;
                if !outcome.deleted_paths.is_empty() {
                    modal.remove_deleted_paths(&outcome.deleted_paths);
                }
            }
            if let Some(err) = outcome.error {
                app.status_notice = Some(err);
            } else if !outcome.deleted_paths.is_empty() {
                if outcome.was_trash {
                    app.push_undo(crate::app::UndoAction::Trash {
                        paths: outcome.deleted_paths.clone(),
                    });
                }
                let action_word = if outcome.was_trash {
                    "movido(s) para a Lixeira"
                } else {
                    "excluído(s) permanentemente"
                };
                app.status_notice = Some(format!(
                    "{} item(ns) {action_word}.",
                    outcome.deleted_paths.len()
                ));
            }
            let mut tasks = Vec::new();
            for dir in outcome.affected_dirs {
                tasks.push(app.reload_matching_dir_tabs(&dir));
            }
            Task::batch(tasks)
        }
    }
}

fn open_and_start_scan(app: &mut App, roots: Vec<PathBuf>) -> Task<Message> {
    if let Some(mut prev) = app.disk_analyzer.take()
        && let Some(cancel) = prev.cancel.take()
    {
        cancel.cancel();
    }
    let mut state = DiskAnalyzerState::new(roots.clone(), app.window_size);
    let scan_id = app.next_request_id;
    app.next_request_id = app.next_request_id.wrapping_add(1);
    let cancel = CancellationToken::new();
    state.scanning = true;
    state.scan_id = scan_id;
    state.cancel = Some(cancel.clone());
    app.disk_analyzer = Some(state);

    let worker_stream = stream::channel(64, async move |mut output| {
        let (tx, mut rx) = mpsc::unbounded_channel::<DiskScanEvent>();
        tokio::spawn(async move {
            run_disk_scan(roots, cancel, tx).await;
        });
        while let Some(ev) = rx.recv().await {
            if output
                .send(Message::DiskAnalyzer(DiskAnalyzerMsg::ScanEvent(
                    scan_id, ev,
                )))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    Task::run(worker_stream, std::convert::identity)
}

fn spawn_delete(app: &mut App, to_trash: bool) -> Task<Message> {
    let Some(modal) = &mut app.disk_analyzer else {
        return Task::none();
    };
    if modal.deleting || modal.selected_paths.is_empty() {
        return Task::none();
    }
    modal.deleting = true;
    modal.confirm_perm_delete = false;
    let targets: Vec<PathBuf> = modal.selected_paths.iter().cloned().collect();
    let vfs: std::sync::Arc<LocalFs> = std::sync::Arc::clone(&app.vfs);

    Task::perform(
        async move {
            let mut deleted_paths = Vec::new();
            let mut affected_dirs = Vec::new();
            let mut last_err = None;
            for path in targets {
                let res = if to_trash {
                    vfs.move_to_trash(std::slice::from_ref(&path)).await
                } else {
                    vfs.delete_permanent(std::slice::from_ref(&path)).await
                };
                match res {
                    Ok(()) => {
                        if let Some(parent) = path.parent()
                            && !affected_dirs.iter().any(|d| d == parent)
                        {
                            affected_dirs.push(parent.to_path_buf());
                        }
                        deleted_paths.push(path);
                    }
                    Err(e) => {
                        last_err = Some(e.to_string());
                    }
                }
            }
            DiskDeleteOutcome {
                deleted_paths,
                error: last_err,
                affected_dirs,
                was_trash: to_trash,
            }
        },
        |outcome| Message::DiskAnalyzer(DiskAnalyzerMsg::DeleteFinished(outcome)),
    )
}
