use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use compact_str::CompactString;
use fm_core::{
    AppConfig, ClipboardTransfer, ConflictPolicy, CopyMode, DirectorySnapshot, DriveInfo, EntryId,
    FileEntry, FolderSummary, FsError, JobProgress, JobState, SelectedFileTelemetry,
    SelectionModifiers, SortField, Strings, TabId, TabState, UserPlaces, VfsProvider, ViewMode,
    VolumeSpace, WindowDecorationsMode,
};
use fm_fs::LocalFs;
use fm_index::SearchHit;
use fm_preview::{ThumbnailCache, ThumbnailKey, ThumbnailRgba};
use fm_theme::ThemePalette;
use iced::futures::SinkExt;
use iced::keyboard;
use iced::keyboard::key::Named;
use iced::widget::operation::RelativeOffset;
use iced::widget::{Id, column, container, image, operation, row, stack, text_editor};
use iced::{Element, Event, Length, Subscription, Task, event, stream, window};
use tokio_util::sync::CancellationToken;

use crate::views;

pub fn search_input_id() -> Id {
    Id::new("fm_search_input")
}

pub fn deep_search_input_id() -> Id {
    Id::new("fm_deep_search_input")
}

pub fn dialog_input_id() -> Id {
    Id::new("fm_dialog_input")
}

pub fn path_input_id() -> Id {
    Id::new("fm_path_input")
}

#[derive(Debug, Clone)]
pub enum ActiveDialog {
    CreateItem { is_dir: bool, name: String },
    Rename { src: PathBuf, name: String },
    ConfirmDelete { paths: Vec<PathBuf>, permanent: bool },
}

pub fn canvas_scroll_id() -> Id {
    Id::new("fm_canvas_scroll")
}

pub fn thumbnail_key_for_entry(entry: &FileEntry) -> ThumbnailKey {
    let modified_nanos = entry
        .modified_at
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    ThumbnailKey {
        path: entry.path.clone(),
        size_bytes: entry.size_bytes,
        modified_nanos,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTarget {
    Documents,
    Home,
    Desktop,
    Downloads,
    Pictures,
    CodeDev,
    Trash,
}

impl SidebarTarget {
    pub fn resolve_path(self, places: &UserPlaces) -> Option<PathBuf> {
        match self {
            Self::Documents => places.documents.clone(),
            Self::Home => Some(places.home.clone()),
            Self::Desktop => places.desktop.clone(),
            Self::Downloads => places.downloads.clone(),
            Self::Pictures => places.pictures.clone(),
            Self::CodeDev => std::env::current_dir().ok(),
            Self::Trash => places.trash.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabNew,
    TabSelect(TabId),
    TabClose(TabId),
    NavigateTo(PathBuf),
    NavigateBack,
    NavigateForward,
    NavigateUp,
    RefreshActiveTab,
    SelectSidebar(SidebarTarget),
    SelectDrive(PathBuf),
    SelectEntry {
        index: usize,
        modifiers: SelectionModifiers,
    },
    ActivateEntry(usize),
    SetViewMode(ViewMode),
    CycleSortField,
    ToggleSortDirection,
    ToggleHidden,
    FilterChanged(String),
    TogglePathEdit,
    PathDraftChanged(String),
    CommitPathDraft,
    ToggleTheme,
    ToggleLanguage,
    ToggleSidebar,
    ToggleInspector,
    ToggleHeroCard,
    ToggleQuickLook,
    CloseQuickLook,
    CanvasScrolled {
        offset_y: f32,
        viewport_height: f32,
    },
    DrivesLoaded(Result<Vec<DriveInfo>, FsError>),
    DirectoryLoaded {
        tab_id: TabId,
        request_id: u64,
        result: Result<DirectorySnapshot, FsError>,
        volume_space: Option<VolumeSpace>,
    },
    TextPreviewLoaded {
        request_id: u64,
        path: PathBuf,
        snippet: Option<String>,
    },
    QuickLookImageLoaded {
        request_id: u64,
        path: PathBuf,
        rgba: Option<Arc<ThumbnailRgba>>,
    },
    QuickLookEditorAction(text_editor::Action),
    DirectoryChangedOnDisk(PathBuf),
    GitBranchReady {
        tab_id: TabId,
        request_id: u64,
        branch: Option<CompactString>,
    },
    ChildCountsReady {
        tab_id: TabId,
        request_id: u64,
        counts: Vec<(PathBuf, u32)>,
    },
    MimeDetected {
        request_id: u64,
        path: PathBuf,
        mime: Option<CompactString>,
    },
    Sha256Computed {
        request_id: u64,
        path: PathBuf,
        sha: Option<(CompactString, CompactString)>,
    },
    OpenDeepSearch,
    CloseDeepSearch,
    DeepSearchQueryChanged(String),
    DeepSearchResults {
        request_id: u64,
        results: Vec<SearchHit>,
    },
    OpenSearchHit(PathBuf),
    ThumbnailReady {
        key: ThumbnailKey,
        rgba: Option<Arc<ThumbnailRgba>>,
    },
    OpenCreateDialog(bool),
    OpenRenameDialog,
    RequestDelete(bool),
    CloseDialog,
    DialogSetCreateIsDir(bool),
    DialogInputChanged(String),
    SubmitDialog,
    CopySelected,
    CutSelected,
    PasteClipboard,
    CancelTransfer,
    TransferProgress(JobProgress),
    FileOpFinished {
        result: Result<(), FsError>,
        select_after: Option<PathBuf>,
    },
    ColumnPreviewLoaded {
        request_id: u64,
        path: PathBuf,
        entries: Vec<FileEntry>,
    },
    ToggleFavoriteActive,
    OpenPreferences,
    ClosePreferences,
    SetDefaultViewMode(ViewMode),
    ToggleWindowDecorations,
    ConfigSaved(Option<String>),
    OpenDefaultFinished(Result<(), FsError>),
    WindowMinimize,
    WindowToggleMaximize,
    WindowClose,
    ModifiersChanged(keyboard::Modifiers),
    GlobalKeyPressed {
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    },
}

pub struct App {
    pub vfs: Arc<LocalFs>,
    pub places: UserPlaces,
    pub config: AppConfig,
    pub tabs: Vec<TabState>,
    pub active_tab: TabId,
    pub next_tab_id: u64,
    pub next_request_id: u64,
    pub tab_cancels: HashMap<TabId, CancellationToken>,
    pub preview_cancel: Option<CancellationToken>,
    pub preview_request_id: u64,
    pub selection_telemetry_cancel: Option<CancellationToken>,
    pub selection_telemetry_request_id: u64,
    pub column_preview_cancel: Option<CancellationToken>,
    pub column_preview_request_id: u64,
    pub column_preview: Option<(PathBuf, Vec<FileEntry>)>,
    pub deep_search_open: bool,
    pub deep_search_query: String,
    pub deep_search_loading: bool,
    pub deep_search_results: Vec<SearchHit>,
    pub deep_search_request_id: u64,
    pub deep_search_cancel: Option<CancellationToken>,
    pub active_dialog: Option<ActiveDialog>,
    pub prefs_open: bool,
    pub clipboard: Option<ClipboardTransfer>,
    pub next_job_id: u64,
    pub active_transfer: Option<JobProgress>,
    pub transfer_cancel: Option<CancellationToken>,
    pub pending_select_path: Option<PathBuf>,
    pub thumbnail_cache: ThumbnailCache,
    pub gpu_thumbnails: HashMap<ThumbnailKey, image::Handle>,
    pub in_flight_thumbs: HashSet<PathBuf>,
    pub quicklook_editor: text_editor::Content,
    pub quicklook_extension: String,
    pub quicklook_image: Option<(PathBuf, image::Handle)>,
    pub drives: Vec<DriveInfo>,
    pub active_sidebar: Option<SidebarTarget>,
    pub path_editing: bool,
    pub path_draft: String,
    pub quicklook_open: bool,
    pub current_modifiers: keyboard::Modifiers,
    pub canvas_scroll_y: f32,
    pub canvas_viewport_height: f32,
    pub status_notice: Option<String>,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let vfs = Arc::new(LocalFs::new());
        let places = fm_fs::user_places();
        let config = fm_fs::load_config();

        let tab1_path = places
            .documents
            .clone()
            .filter(|p| p.exists())
            .unwrap_or_else(|| places.home.clone());
        let tab2_path = std::env::current_dir()
            .ok()
            .filter(|p| p.exists())
            .unwrap_or_else(|| places.home.clone());

        let mut tab1 = TabState::new(TabId(1), tab1_path.clone());
        tab1.view_mode = config.default_view_mode;
        tab1.is_loading = true;
        tab1.active_request_id = 1;

        let mut tab2 = TabState::new(TabId(2), tab2_path.clone());
        tab2.view_mode = config.default_view_mode;
        tab2.is_loading = true;
        tab2.active_request_id = 2;

        let cancel1 = CancellationToken::new();
        let cancel2 = CancellationToken::new();

        let mut tab_cancels = HashMap::new();
        tab_cancels.insert(TabId(1), cancel1.clone());
        tab_cancels.insert(TabId(2), cancel2.clone());

        let app = Self {
            vfs: Arc::clone(&vfs),
            places,
            config,
            tabs: vec![tab1, tab2],
            active_tab: TabId(1),
            next_tab_id: 3,
            next_request_id: 3,
            tab_cancels,
            preview_cancel: None,
            preview_request_id: 0,
            selection_telemetry_cancel: None,
            selection_telemetry_request_id: 0,
            column_preview_cancel: None,
            column_preview_request_id: 0,
            column_preview: None,
            deep_search_open: false,
            deep_search_query: String::new(),
            deep_search_loading: false,
            deep_search_results: Vec::new(),
            deep_search_request_id: 0,
            deep_search_cancel: None,
            active_dialog: None,
            prefs_open: false,
            clipboard: None,
            next_job_id: 1,
            active_transfer: None,
            transfer_cancel: None,
            pending_select_path: None,
            thumbnail_cache: ThumbnailCache::default(),
            gpu_thumbnails: HashMap::new(),
            in_flight_thumbs: HashSet::new(),
            quicklook_editor: text_editor::Content::new(),
            quicklook_extension: String::new(),
            quicklook_image: None,
            drives: Vec::new(),
            active_sidebar: Some(SidebarTarget::Documents),
            path_editing: false,
            path_draft: String::new(),
            quicklook_open: false,
            current_modifiers: keyboard::Modifiers::empty(),
            canvas_scroll_y: 0.0,
            canvas_viewport_height: 700.0,
            status_notice: None,
        };

        let drives_task = {
            let vfs = Arc::clone(&vfs);
            Task::perform(async move { vfs.list_drives().await }, Message::DrivesLoaded)
        };
        let load1 = spawn_list_task(Arc::clone(&vfs), TabId(1), tab1_path, 1, cancel1);
        let load2 = spawn_list_task(vfs, TabId(2), tab2_path, 2, cancel2);

        (app, Task::batch([drives_task, load1, load2]))
    }

    pub fn title(&self) -> String {
        let tab_title = self
            .current_tab()
            .map(|t| t.title.as_str())
            .unwrap_or("Vagnarok");
        format!("Vagnarok File Manager — {tab_title}")
    }

    pub fn iced_theme(&self) -> iced::Theme {
        self.palette().to_iced_theme()
    }

    pub fn palette(&self) -> ThemePalette {
        ThemePalette::from_preset(self.config.theme)
    }

    pub fn strings(&self) -> Strings {
        Strings::for_language(self.config.language)
    }

    pub fn current_tab(&self) -> Option<&TabState> {
        self.tabs.iter().find(|t| t.id == self.active_tab)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut TabState> {
        let active = self.active_tab;
        self.tabs.iter_mut().find(|t| t.id == active)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabNew => {
                let id = TabId(self.next_tab_id);
                self.next_tab_id += 1;
                let req = self.alloc_request_id();
                let path = self
                    .current_tab()
                    .map(|t| t.current_path.clone())
                    .unwrap_or_else(|| self.places.home.clone());

                let mut tab = TabState::new(id, path.clone());
                tab.view_mode = self.config.default_view_mode;
                tab.active_request_id = req;
                tab.is_loading = true;
                self.tabs.push(tab);
                self.active_tab = id;
                self.path_editing = false;
                self.quicklook_open = false;

                let cancel = self.reset_tab_cancel(id);
                Task::batch([
                    spawn_list_task(Arc::clone(&self.vfs), id, path, req, cancel),
                    operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
                ])
            }
            Message::TabSelect(id) => {
                if self.tabs.iter().any(|t| t.id == id) {
                    self.active_tab = id;
                    self.path_editing = false;
                    self.quicklook_open = false;
                    self.canvas_scroll_y = 0.0;
                    let thumb_task = self.queue_visible_thumbnails();
                    return Task::batch([
                        operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
                        thumb_task,
                    ]);
                }
                Task::none()
            }
            Message::TabClose(id) => {
                if self.tabs.len() <= 1 {
                    return Task::none();
                }
                if let Some(cancel) = self.tab_cancels.remove(&id) {
                    cancel.cancel();
                }
                if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                    self.tabs.remove(pos);
                    if self.active_tab == id {
                        let fallback = pos.saturating_sub(1).min(self.tabs.len() - 1);
                        self.active_tab = self.tabs[fallback].id;
                        self.canvas_scroll_y = 0.0;
                    }
                }
                Task::none()
            }
            Message::NavigateTo(path) => self.navigate_active_tab(path),
            Message::NavigateBack => {
                let req = self.alloc_request_id();
                let mut target_path = None;
                let active = self.active_tab;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(prev) = tab.history_back.pop()
                {
                    let cur = std::mem::replace(&mut tab.current_path, prev.clone());
                    tab.history_forward.push(cur);
                    prepare_tab_for_load(tab, req);
                    target_path = Some(prev);
                }
                if let Some(path) = target_path {
                    self.finish_navigation_tasks(active, path, req)
                } else {
                    Task::none()
                }
            }
            Message::NavigateForward => {
                let req = self.alloc_request_id();
                let mut target_path = None;
                let active = self.active_tab;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(next) = tab.history_forward.pop()
                {
                    let cur = std::mem::replace(&mut tab.current_path, next.clone());
                    tab.history_back.push(cur);
                    prepare_tab_for_load(tab, req);
                    target_path = Some(next);
                }
                if let Some(path) = target_path {
                    self.finish_navigation_tasks(active, path, req)
                } else {
                    Task::none()
                }
            }
            Message::NavigateUp => {
                let parent = self
                    .current_tab()
                    .and_then(|t| t.current_path.parent().map(PathBuf::from));
                if let Some(p) = parent {
                    self.navigate_active_tab(p)
                } else {
                    Task::none()
                }
            }
            Message::RefreshActiveTab => {
                let req = self.alloc_request_id();
                let active = self.active_tab;
                let target_path = self.current_tab_mut().map(|tab| {
                    prepare_tab_for_load(tab, req);
                    tab.current_path.clone()
                });
                if let Some(path) = target_path {
                    let cancel = self.reset_tab_cancel(active);
                    spawn_list_task(Arc::clone(&self.vfs), active, path, req, cancel)
                } else {
                    Task::none()
                }
            }
            Message::SelectSidebar(target) => {
                if let Some(path) = target.resolve_path(&self.places) {
                    self.active_sidebar = Some(target);
                    self.navigate_active_tab(path)
                } else {
                    Task::none()
                }
            }
            Message::SelectDrive(path) => {
                self.active_sidebar = None;
                self.navigate_active_tab(path)
            }
            Message::SelectEntry { index, modifiers } => {
                let mut single_entry = None;
                if let Some(tab) = self.current_tab_mut() {
                    tab.selection
                        .apply_click(index, modifiers, &tab.visible_entries);
                    single_entry = tab
                        .selection
                        .single_selected(&tab.visible_entries)
                        .cloned();
                    tab.selected_telemetry = single_entry.as_ref().map(basic_file_telemetry);
                }
                self.dispatch_selection_side_effects(single_entry.as_ref())
            }
            Message::ActivateEntry(index) => {
                let target = self
                    .current_tab()
                    .and_then(|t| t.visible_entries.get(index).cloned());
                if let Some(entry) = target {
                    if entry.kind.is_dir_like() {
                        self.active_sidebar = None;
                        return self.navigate_active_tab(entry.path);
                    }
                    let vfs = Arc::clone(&self.vfs);
                    let path = entry.path;
                    return Task::perform(
                        async move { vfs.open_default(&path).await },
                        Message::OpenDefaultFinished,
                    );
                }
                Task::none()
            }
            Message::SetViewMode(mode) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.view_mode = mode;
                }
                self.canvas_scroll_y = 0.0;
                let single = self
                    .current_tab()
                    .and_then(|t| t.selection.single_selected(&t.visible_entries))
                    .cloned();
                let col_task = self.maybe_load_column_preview(single.as_ref());
                let thumb_task = self.queue_visible_thumbnails();
                Task::batch([
                    operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
                    thumb_task,
                    col_task,
                ])
            }
            Message::CycleSortField => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.sort.field = match tab.sort.field {
                        SortField::Name => SortField::Size,
                        SortField::Size => SortField::Modified,
                        SortField::Modified => SortField::Kind,
                        SortField::Kind => SortField::Name,
                    };
                    apply_tab_filter(tab);
                }
                self.queue_visible_thumbnails()
            }
            Message::ToggleSortDirection => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.sort.direction = tab.sort.direction.toggled();
                    apply_tab_filter(tab);
                }
                self.queue_visible_thumbnails()
            }
            Message::ToggleHidden => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.show_hidden = !tab.show_hidden;
                    apply_tab_filter(tab);
                }
                self.queue_visible_thumbnails()
            }
            Message::FilterChanged(query) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.filter_query = query;
                    apply_tab_filter(tab);
                }
                self.canvas_scroll_y = 0.0;
                self.queue_visible_thumbnails()
            }
            Message::TogglePathEdit => {
                self.path_editing = !self.path_editing;
                if self.path_editing {
                    self.path_draft = self
                        .current_tab()
                        .map(|t| t.current_path.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    return operation::focus(path_input_id());
                }
                Task::none()
            }
            Message::PathDraftChanged(value) => {
                self.path_draft = value;
                Task::none()
            }
            Message::CommitPathDraft => {
                let raw = self.path_draft.trim().to_owned();
                self.path_editing = false;
                if !raw.is_empty() {
                    self.active_sidebar = None;
                    return self.navigate_active_tab(PathBuf::from(raw));
                }
                Task::none()
            }
            Message::ToggleTheme => {
                self.config.theme = self.config.theme.toggled();
                self.persist_config_task()
            }
            Message::ToggleLanguage => {
                self.config.language = self.config.language.toggled();
                self.persist_config_task()
            }
            Message::ToggleSidebar => {
                self.config.sidebar_collapsed = !self.config.sidebar_collapsed;
                self.persist_config_task()
            }
            Message::ToggleInspector => {
                self.config.show_inspector = !self.config.show_inspector;
                let persist = self.persist_config_task();
                if self.config.show_inspector {
                    let single = self
                        .current_tab()
                        .and_then(|t| t.selection.single_selected(&t.visible_entries))
                        .cloned();
                    let side = self.dispatch_selection_side_effects(single.as_ref());
                    return Task::batch([persist, side]);
                }
                persist
            }
            Message::ToggleHeroCard => {
                self.config.show_hero_card = !self.config.show_hero_card;
                self.persist_config_task()
            }
            Message::ToggleQuickLook => {
                let single = self
                    .current_tab()
                    .and_then(|t| t.selection.single_selected(&t.visible_entries))
                    .cloned();
                if self.quicklook_open {
                    self.quicklook_open = false;
                    self.quicklook_image = None;
                    if let Some(cancel) = self.preview_cancel.take() {
                        cancel.cancel();
                    }
                    return Task::none();
                }
                if let Some(entry) = single {
                    self.quicklook_open = true;
                    return self.maybe_load_preview(Some(&entry));
                }
                Task::none()
            }
            Message::CloseQuickLook => {
                self.quicklook_open = false;
                self.quicklook_image = None;
                if let Some(cancel) = self.preview_cancel.take() {
                    cancel.cancel();
                }
                Task::none()
            }
            Message::CanvasScrolled {
                offset_y,
                viewport_height,
            } => {
                self.canvas_scroll_y = offset_y;
                if viewport_height > 0.0 {
                    self.canvas_viewport_height = viewport_height;
                }
                self.queue_visible_thumbnails()
            }
            Message::DrivesLoaded(res) => {
                if let Ok(drives) = res {
                    self.drives = drives;
                }
                Task::none()
            }
            Message::DirectoryLoaded {
                tab_id,
                request_id,
                result,
                volume_space,
            } => {
                let mut tasks = Vec::new();
                let mut pending_selected_entry = None;

                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    // Descarta respostas obsoletas de navegações canceladas.
                    if tab.active_request_id != request_id {
                        return Task::none();
                    }
                    tab.is_loading = false;
                    tab.telemetry.volume_space = volume_space;

                    match result {
                        Ok(snapshot) => {
                            tab.last_error = None;
                            tab.fs_type = snapshot.fs_type;
                            tab.raw_entries = snapshot.entries;
                            apply_tab_filter(tab);
                            tab.title = display_tab_title(&tab.current_path);

                            if tab_id == self.active_tab
                                && let Some(target_path) = self.pending_select_path.take()
                                && let Some(idx) = tab
                                    .visible_entries
                                    .iter()
                                    .position(|e| e.path == target_path)
                            {
                                tab.selection.select_single(idx, &tab.visible_entries);
                                let single = tab
                                    .selection
                                    .single_selected(&tab.visible_entries)
                                    .cloned();
                                tab.selected_telemetry =
                                    single.as_ref().map(basic_file_telemetry);
                                pending_selected_entry = single;
                            }

                            let cancel = self
                                .tab_cancels
                                .get(&tab_id)
                                .cloned()
                                .unwrap_or_default();
                            let dir_path = tab.current_path.clone();
                            tasks.push(Task::perform(
                                {
                                    let cancel = cancel.clone();
                                    async move {
                                        let branch =
                                            fm_telemetry::detect_git_branch(&dir_path, cancel)
                                                .await;
                                        (tab_id, request_id, branch)
                                    }
                                },
                                |(tab_id, request_id, branch)| Message::GitBranchReady {
                                    tab_id,
                                    request_id,
                                    branch,
                                },
                            ));

                            let child_dirs: Vec<PathBuf> = tab
                                .raw_entries
                                .iter()
                                .filter(|e| e.kind.is_dir_like())
                                .take(64)
                                .map(|e| e.path.clone())
                                .collect();
                            if !child_dirs.is_empty() {
                                tasks.push(Task::perform(
                                    async move {
                                        let counts =
                                            fm_telemetry::count_direct_children(child_dirs, cancel)
                                                .await;
                                        (tab_id, request_id, counts)
                                    },
                                    |(tab_id, request_id, counts)| Message::ChildCountsReady {
                                        tab_id,
                                        request_id,
                                        counts,
                                    },
                                ));
                            }
                        }
                        Err(FsError::Cancelled) => {}
                        Err(err) => {
                            tab.raw_entries.clear();
                            tab.visible_entries.clear();
                            tab.selection.clear();
                            tab.last_error = Some(err.to_string());
                        }
                    }
                }

                if pending_selected_entry.is_some() {
                    tasks.push(
                        self.dispatch_selection_side_effects(pending_selected_entry.as_ref()),
                    );
                } else if tab_id == self.active_tab {
                    tasks.push(self.queue_visible_thumbnails());
                }

                Task::batch(tasks)
            }
            Message::TextPreviewLoaded {
                request_id,
                path,
                snippet,
            } => {
                if request_id != self.preview_request_id {
                    return Task::none();
                }
                if let Some(code) = &snippet {
                    self.quicklook_editor = text_editor::Content::with_text(code);
                    self.quicklook_extension = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_lowercase())
                        .unwrap_or_else(|| "txt".to_owned());
                } else {
                    self.quicklook_editor = text_editor::Content::new();
                    self.quicklook_extension.clear();
                }
                if let Some(tab) = self.current_tab_mut()
                    && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                    && selected.path == path
                {
                    let mut tel = tab
                        .selected_telemetry
                        .clone()
                        .unwrap_or_else(|| basic_file_telemetry(selected));
                    tel.preview_snippet = snippet;
                    tab.selected_telemetry = Some(tel);
                }
                Task::none()
            }
            Message::QuickLookImageLoaded {
                request_id,
                path,
                rgba,
            } => {
                if request_id != self.preview_request_id {
                    return Task::none();
                }
                self.quicklook_image = rgba
                    .map(|thumb| {
                        (
                            path,
                            image::Handle::from_rgba(
                                thumb.width,
                                thumb.height,
                                thumb.pixels.clone(),
                            ),
                        )
                    });
                Task::none()
            }
            Message::QuickLookEditorAction(action) => {
                // Mantém QuickLook estritamente somente leitura, permitindo seleção e rolagem.
                if !matches!(action, text_editor::Action::Edit(_)) {
                    self.quicklook_editor.perform(action);
                }
                Task::none()
            }
            Message::DirectoryChangedOnDisk(watched_root) => {
                let is_active_same = self
                    .current_tab()
                    .is_some_and(|t| t.current_path == watched_root && !t.is_loading);
                if !is_active_same {
                    return Task::none();
                }
                let req = self.alloc_request_id();
                let active = self.active_tab;
                if let Some(tab) = self.current_tab_mut() {
                    tab.active_request_id = req;
                }
                let cancel = self.reset_tab_cancel(active);
                spawn_list_task(Arc::clone(&self.vfs), active, watched_root, req, cancel)
            }
            Message::GitBranchReady {
                tab_id,
                request_id,
                branch,
            } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id)
                    && tab.active_request_id == request_id
                {
                    tab.telemetry.git_branch = branch;
                }
                Task::none()
            }
            Message::ChildCountsReady {
                tab_id,
                request_id,
                counts,
            } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id)
                    && tab.active_request_id == request_id
                {
                    tab.apply_child_counts(&counts);
                }
                Task::none()
            }
            Message::MimeDetected {
                request_id,
                path,
                mime,
            } => {
                if request_id != self.selection_telemetry_request_id {
                    return Task::none();
                }
                if let Some(tab) = self.current_tab_mut()
                    && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                    && selected.path == path
                {
                    let mut tel = tab
                        .selected_telemetry
                        .clone()
                        .unwrap_or_else(|| basic_file_telemetry(selected));
                    if mime.is_some() {
                        tel.mime_type = mime;
                    }
                    tab.selected_telemetry = Some(tel);
                }
                Task::none()
            }
            Message::Sha256Computed {
                request_id,
                path,
                sha,
            } => {
                if request_id != self.selection_telemetry_request_id {
                    return Task::none();
                }
                if let Some(tab) = self.current_tab_mut()
                    && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                    && selected.path == path
                {
                    let mut tel = tab
                        .selected_telemetry
                        .clone()
                        .unwrap_or_else(|| basic_file_telemetry(selected));
                    if let Some((prefix, full)) = sha {
                        tel.sha256_prefix = Some(prefix);
                        tel.sha256_full = Some(full);
                    }
                    tab.selected_telemetry = Some(tel);
                }
                Task::none()
            }
            Message::OpenDeepSearch => {
                self.deep_search_open = true;
                if self.deep_search_query.is_empty()
                    && let Some(q) = self.current_tab().map(|t| t.filter_query.clone())
                    && !q.trim().is_empty()
                {
                    self.deep_search_query = q;
                }
                let search_task = self.spawn_deep_search_task();
                Task::batch([operation::focus(deep_search_input_id()), search_task])
            }
            Message::CloseDeepSearch => {
                self.deep_search_open = false;
                self.deep_search_loading = false;
                if let Some(cancel) = self.deep_search_cancel.take() {
                    cancel.cancel();
                }
                Task::none()
            }
            Message::DeepSearchQueryChanged(query) => {
                self.deep_search_query = query;
                self.spawn_deep_search_task()
            }
            Message::DeepSearchResults {
                request_id,
                results,
            } => {
                if request_id == self.deep_search_request_id {
                    self.deep_search_loading = false;
                    self.deep_search_results = results;
                }
                Task::none()
            }
            Message::OpenSearchHit(hit_path) => {
                self.deep_search_open = false;
                self.deep_search_loading = false;
                if let Some(cancel) = self.deep_search_cancel.take() {
                    cancel.cancel();
                }
                if hit_path.is_dir() {
                    self.pending_select_path = None;
                    return self.navigate_active_tab(hit_path);
                }
                if let Some(parent) = hit_path.parent().map(PathBuf::from) {
                    self.pending_select_path = Some(hit_path);
                    return self.navigate_active_tab(parent);
                }
                Task::none()
            }
            Message::ThumbnailReady { key, rgba } => {
                self.in_flight_thumbs.remove(&key.path);
                if let Some(thumb) = rgba {
                    self.thumbnail_cache.insert(key.clone(), Arc::clone(&thumb));
                    if self.gpu_thumbnails.len() >= fm_preview::DEFAULT_THUMBNAIL_CACHE_CAPACITY {
                        self.gpu_thumbnails.clear();
                    }
                    self.gpu_thumbnails.insert(
                        key,
                        image::Handle::from_rgba(thumb.width, thumb.height, thumb.pixels.clone()),
                    );
                }
                Task::none()
            }
            Message::OpenCreateDialog(is_dir) => {
                self.active_dialog = Some(ActiveDialog::CreateItem {
                    is_dir,
                    name: String::new(),
                });
                operation::focus(dialog_input_id())
            }
            Message::OpenRenameDialog => {
                let target = self
                    .current_tab()
                    .and_then(|t| t.selection.single_selected(&t.visible_entries))
                    .map(|e| (e.path.clone(), e.name.to_string()));
                if let Some((src, name)) = target {
                    self.active_dialog = Some(ActiveDialog::Rename { src, name });
                    return operation::focus(dialog_input_id());
                }
                Task::none()
            }
            Message::RequestDelete(permanent) => {
                let paths = self.selected_paths();
                if paths.is_empty() {
                    return Task::none();
                }
                self.active_dialog = Some(ActiveDialog::ConfirmDelete { paths, permanent });
                Task::none()
            }
            Message::CloseDialog => {
                self.active_dialog = None;
                Task::none()
            }
            Message::DialogSetCreateIsDir(is_dir) => {
                if let Some(ActiveDialog::CreateItem { is_dir: cur, .. }) =
                    &mut self.active_dialog
                {
                    *cur = is_dir;
                }
                operation::focus(dialog_input_id())
            }
            Message::DialogInputChanged(val) => {
                match &mut self.active_dialog {
                    Some(ActiveDialog::CreateItem { name, .. })
                    | Some(ActiveDialog::Rename { name, .. }) => *name = val,
                    _ => {}
                }
                Task::none()
            }
            Message::SubmitDialog => self.execute_active_dialog(),
            Message::CopySelected => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(ClipboardTransfer {
                        mode: CopyMode::Copy,
                        paths,
                    });
                }
                Task::none()
            }
            Message::CutSelected => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(ClipboardTransfer {
                        mode: CopyMode::Move,
                        paths,
                    });
                }
                Task::none()
            }
            Message::PasteClipboard => self.start_clipboard_paste(),
            Message::CancelTransfer => {
                if let Some(cancel) = self.transfer_cancel.take() {
                    cancel.cancel();
                }
                Task::none()
            }
            Message::TransferProgress(progress) => {
                let is_active_job = self
                    .active_transfer
                    .as_ref()
                    .is_none_or(|cur| cur.job_id == progress.job_id);
                if !is_active_job {
                    return Task::none();
                }
                match &progress.state {
                    JobState::Running => {
                        self.active_transfer = Some(progress);
                        Task::none()
                    }
                    JobState::Completed => {
                        self.active_transfer = None;
                        self.transfer_cancel = None;
                        self.status_notice = None;
                        self.update(Message::RefreshActiveTab)
                    }
                    JobState::Cancelled => {
                        self.active_transfer = None;
                        self.transfer_cancel = None;
                        self.update(Message::RefreshActiveTab)
                    }
                    JobState::Failed(msg) => {
                        self.active_transfer = None;
                        self.transfer_cancel = None;
                        self.status_notice = Some(msg.clone());
                        self.update(Message::RefreshActiveTab)
                    }
                }
            }
            Message::FileOpFinished {
                result,
                select_after,
            } => {
                match result {
                    Ok(()) => {
                        self.status_notice = None;
                        if let Some(target) = select_after {
                            self.pending_select_path = Some(target);
                        }
                        self.update(Message::RefreshActiveTab)
                    }
                    Err(err) => {
                        self.status_notice = Some(err.to_string());
                        Task::none()
                    }
                }
            }
            Message::ColumnPreviewLoaded {
                request_id,
                path,
                entries,
            } => {
                if request_id == self.column_preview_request_id {
                    self.column_preview = Some((path, entries));
                }
                Task::none()
            }
            Message::ToggleFavoriteActive => {
                if let Some(cur) = self.current_tab().map(|t| t.current_path.clone()) {
                    if let Some(pos) = self.config.favorites.iter().position(|p| p == &cur) {
                        self.config.favorites.remove(pos);
                    } else {
                        self.config.favorites.push(cur);
                    }
                    return self.persist_config_task();
                }
                Task::none()
            }
            Message::OpenPreferences => {
                self.prefs_open = true;
                Task::none()
            }
            Message::ClosePreferences => {
                self.prefs_open = false;
                Task::none()
            }
            Message::SetDefaultViewMode(mode) => {
                self.config.default_view_mode = mode;
                self.persist_config_task()
            }
            Message::ToggleWindowDecorations => {
                self.config.window_decorations = match self.config.window_decorations {
                    WindowDecorationsMode::NativeWm => WindowDecorationsMode::IntegratedCsd,
                    WindowDecorationsMode::IntegratedCsd => WindowDecorationsMode::NativeWm,
                };
                self.persist_config_task()
            }
            Message::ConfigSaved(err) => {
                if let Some(msg) = err {
                    self.status_notice = Some(msg);
                }
                Task::none()
            }
            Message::OpenDefaultFinished(res) => {
                self.status_notice = res.err().map(|e| e.to_string());
                Task::none()
            }
            Message::WindowMinimize => window::latest().and_then(|id| window::minimize(id, true)),
            Message::WindowToggleMaximize => window::latest().and_then(window::toggle_maximize),
            Message::WindowClose => window::latest().and_then(window::close),
            Message::ModifiersChanged(modifiers) => {
                self.current_modifiers = modifiers;
                Task::none()
            }
            Message::GlobalKeyPressed { key, modifiers } => {
                self.handle_global_key(key, modifiers)
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = event::listen_with(|event, status, _window| match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                Some(Message::ModifiersChanged(modifiers))
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let is_shortcut = modifiers.control()
                    || modifiers.alt()
                    || matches!(
                        key,
                        keyboard::Key::Named(Named::Escape | Named::F5)
                    );
                if status == event::Status::Ignored || is_shortcut {
                    Some(Message::GlobalKeyPressed { key, modifiers })
                } else {
                    None
                }
            }
            _ => None,
        });

        if let Some(active_path) = self.current_tab().map(|t| t.current_path.clone()) {
            let watch_sub = Subscription::run_with(active_path, watch_directory_stream);
            Subscription::batch([keyboard_sub, watch_sub])
        } else {
            keyboard_sub
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let palette = self.palette();
        let strings = self.strings();
        let tab = self.current_tab();

        let body_row = row![
            views::sidebar::view(self, palette, strings),
            container(iced::widget::Space::new().width(1).height(Length::Fill))
                .style(fm_theme::styles::divider_line(palette)),
            views::canvas::view(self, tab, palette, strings),
            views::inspector::drawer(self, tab, palette, strings),
        ]
        .height(Length::Fill)
        .width(Length::Fill);

        let main_shell = container(
            column![
                views::header::view(self, tab, palette, strings),
                container(iced::widget::Space::new().width(Length::Fill).height(1))
                    .style(fm_theme::styles::divider_line(palette)),
                body_row,
                container(iced::widget::Space::new().width(Length::Fill).height(1))
                    .style(fm_theme::styles::divider_line(palette)),
                views::footer::view(self, tab, palette, strings),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(fm_theme::styles::canvas_container(palette));

        if let Some(dialog) = &self.active_dialog {
            stack![
                main_shell,
                views::modals::dialog_modal(dialog, palette, strings),
            ]
            .into()
        } else if self.prefs_open {
            stack![
                main_shell,
                views::modals::preferences_modal(self, palette, strings),
            ]
            .into()
        } else if self.quicklook_open {
            stack![
                main_shell,
                views::inspector::quicklook_modal(self, tab, palette, strings),
            ]
            .into()
        } else if self.deep_search_open {
            stack![
                main_shell,
                views::inspector::deep_search_modal(self, palette, strings),
            ]
            .into()
        } else {
            main_shell.into()
        }
    }

    fn alloc_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    fn reset_tab_cancel(&mut self, tab_id: TabId) -> CancellationToken {
        if let Some(prev) = self.tab_cancels.remove(&tab_id) {
            prev.cancel();
        }
        let next = CancellationToken::new();
        self.tab_cancels.insert(tab_id, next.clone());
        next
    }

    fn navigate_active_tab(&mut self, path: PathBuf) -> Task<Message> {
        let req = self.alloc_request_id();
        let active = self.active_tab;
        if let Some(tab) = self.current_tab_mut() {
            tab.push_navigation(path.clone());
            prepare_tab_for_load(tab, req);
        }
        self.finish_navigation_tasks(active, path, req)
    }

    fn finish_navigation_tasks(
        &mut self,
        tab_id: TabId,
        path: PathBuf,
        request_id: u64,
    ) -> Task<Message> {
        self.path_editing = false;
        self.quicklook_open = false;
        self.quicklook_image = None;
        self.quicklook_editor = text_editor::Content::new();
        self.quicklook_extension.clear();
        if let Some(cancel) = self.selection_telemetry_cancel.take() {
            cancel.cancel();
        }
        if let Some(cancel) = self.column_preview_cancel.take() {
            cancel.cancel();
        }
        self.column_preview = None;
        self.status_notice = None;
        self.canvas_scroll_y = 0.0;
        let cancel = self.reset_tab_cancel(tab_id);
        Task::batch([
            spawn_list_task(Arc::clone(&self.vfs), tab_id, path, request_id, cancel),
            operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
        ])
    }

    fn dispatch_selection_side_effects(&mut self, entry: Option<&FileEntry>) -> Task<Message> {
        let tel_task = self.maybe_load_selection_telemetry(entry);
        let col_task = self.maybe_load_column_preview(entry);
        let preview_task = if self.quicklook_open || self.config.show_inspector {
            self.maybe_load_preview(entry)
        } else {
            Task::none()
        };
        let thumb_task = self.queue_visible_thumbnails();
        Task::batch([tel_task, col_task, preview_task, thumb_task])
    }

    fn persist_config_task(&self) -> Task<Message> {
        let cfg = self.config.clone();
        Task::perform(
            async move { fm_fs::save_config(cfg).await.err().map(|e| e.to_string()) },
            Message::ConfigSaved,
        )
    }

    fn selected_paths(&self) -> Vec<PathBuf> {
        let Some(tab) = self.current_tab() else {
            return Vec::new();
        };
        tab.visible_entries
            .iter()
            .filter(|e| tab.selection.selected_ids.contains(&e.id))
            .map(|e| e.path.clone())
            .collect()
    }

    fn execute_active_dialog(&mut self) -> Task<Message> {
        let Some(dialog) = self.active_dialog.take() else {
            return Task::none();
        };
        let vfs = Arc::clone(&self.vfs);
        match dialog {
            ActiveDialog::CreateItem { is_dir, name } => {
                let trimmed = name.trim();
                if trimmed.is_empty() || trimmed.contains(['/', '\\']) {
                    self.status_notice = Some("Nome inválido.".to_owned());
                    return Task::none();
                }
                let Some(dir) = self.current_tab().map(|t| t.current_path.clone()) else {
                    return Task::none();
                };
                let target = dir.join(trimmed);
                let select_target = target.clone();
                Task::perform(
                    async move {
                        if is_dir {
                            vfs.create_dir(&target).await
                        } else {
                            vfs.create_file(&target).await
                        }
                    },
                    move |result| Message::FileOpFinished {
                        result,
                        select_after: Some(select_target),
                    },
                )
            }
            ActiveDialog::Rename { src, name } => {
                let trimmed = name.trim();
                if trimmed.is_empty() || trimmed.contains(['/', '\\']) {
                    self.status_notice = Some("Nome inválido.".to_owned());
                    return Task::none();
                }
                let Some(parent) = src.parent() else {
                    return Task::none();
                };
                let dst = parent.join(trimmed);
                if src == dst {
                    return Task::none();
                }
                let select_target = dst.clone();
                Task::perform(
                    async move { vfs.rename(&src, &dst).await },
                    move |result| Message::FileOpFinished {
                        result,
                        select_after: Some(select_target),
                    },
                )
            }
            ActiveDialog::ConfirmDelete { paths, permanent } => Task::perform(
                async move {
                    if permanent {
                        vfs.delete_permanent(&paths).await
                    } else {
                        vfs.move_to_trash(&paths).await
                    }
                },
                |result| Message::FileOpFinished {
                    result,
                    select_after: None,
                },
            ),
        }
    }

    fn start_clipboard_paste(&mut self) -> Task<Message> {
        let Some(dest_dir) = self.current_tab().map(|t| t.current_path.clone()) else {
            return Task::none();
        };
        let Some(transfer) = self.clipboard.clone() else {
            return Task::none();
        };
        if transfer.paths.is_empty() {
            return Task::none();
        }
        if transfer.mode == CopyMode::Move {
            self.clipboard = None;
        }
        if let Some(prev) = self.transfer_cancel.take() {
            prev.cancel();
        }
        let job_id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancellationToken::new();
        self.transfer_cancel = Some(cancel.clone());
        let mode = transfer.mode;
        let sources = transfer.paths;

        self.active_transfer = Some(JobProgress {
            job_id,
            mode,
            bytes_done: 0,
            bytes_total: 0,
            items_done: 0,
            items_total: sources.len(),
            current_item: CompactString::new("..."),
            state: JobState::Running,
        });

        let transfer_stream = stream::channel(32, async move |mut output| {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let handle = tokio::spawn(async move {
                let _ = fm_fs::run_transfer_job(
                    job_id,
                    mode,
                    sources,
                    dest_dir,
                    ConflictPolicy::RenameAuto,
                    cancel,
                    tx,
                )
                .await;
            });
            while let Some(progress) = rx.recv().await {
                if output.send(Message::TransferProgress(progress)).await.is_err() {
                    break;
                }
            }
            let _ = handle.await;
        });

        Task::stream(transfer_stream)
    }

    fn maybe_load_column_preview(&mut self, entry: Option<&FileEntry>) -> Task<Message> {
        if let Some(prev) = self.column_preview_cancel.take() {
            prev.cancel();
        }
        let is_columns = self
            .current_tab()
            .is_some_and(|t| t.view_mode == ViewMode::Columns);
        let Some(entry) = entry else {
            self.column_preview = None;
            return Task::none();
        };
        if !is_columns || !entry.kind.is_dir_like() {
            self.column_preview = None;
            return Task::none();
        }
        if self
            .column_preview
            .as_ref()
            .is_some_and(|(p, _)| p == &entry.path)
        {
            return Task::none();
        }

        self.column_preview_request_id += 1;
        let req_id = self.column_preview_request_id;
        let cancel = CancellationToken::new();
        self.column_preview_cancel = Some(cancel.clone());
        let target = entry.path.clone();
        let show_hidden = self.current_tab().is_some_and(|t| t.show_hidden);
        let sort = self.current_tab().map(|t| t.sort).unwrap_or_default();
        let vfs = Arc::clone(&self.vfs);

        Task::perform(
            async move {
                let Ok(snap) = vfs.list_dir(&target, req_id, cancel).await else {
                    return (req_id, target, Vec::new());
                };
                let mut entries: Vec<FileEntry> = snap
                    .entries
                    .into_iter()
                    .filter(|e| show_hidden || !e.is_hidden)
                    .collect();
                entries.sort_by(|a, b| sort.compare(a, b));
                (req_id, target, entries)
            },
            |(request_id, path, entries)| Message::ColumnPreviewLoaded {
                request_id,
                path,
                entries,
            },
        )
    }

    fn maybe_load_selection_telemetry(&mut self, entry: Option<&FileEntry>) -> Task<Message> {
        if let Some(prev) = self.selection_telemetry_cancel.take() {
            prev.cancel();
        }
        let Some(entry) = entry else {
            return Task::none();
        };
        if entry.kind.is_dir_like() {
            return Task::none();
        }
        self.selection_telemetry_request_id += 1;
        let req_id = self.selection_telemetry_request_id;
        let cancel = CancellationToken::new();
        self.selection_telemetry_cancel = Some(cancel.clone());

        let path_mime = entry.path.clone();
        let path_sha = entry.path.clone();
        let cancel_mime = cancel.clone();

        let mime_task = Task::perform(
            async move {
                let mime = fm_telemetry::detect_mime_type(&path_mime, cancel_mime).await;
                (req_id, path_mime, mime)
            },
            |(request_id, path, mime)| Message::MimeDetected {
                request_id,
                path,
                mime,
            },
        );

        let sha_task = Task::perform(
            async move {
                let sha = fm_telemetry::compute_sha256(&path_sha, cancel).await;
                (req_id, path_sha, sha)
            },
            |(request_id, path, sha)| Message::Sha256Computed {
                request_id,
                path,
                sha,
            },
        );

        Task::batch([mime_task, sha_task])
    }

    fn maybe_load_preview(&mut self, entry: Option<&FileEntry>) -> Task<Message> {
        if let Some(prev) = self.preview_cancel.take() {
            prev.cancel();
        }
        self.quicklook_image = None;
        self.quicklook_editor = text_editor::Content::new();
        self.quicklook_extension.clear();

        let Some(entry) = entry else {
            return Task::none();
        };
        if entry.kind.is_dir_like() {
            return Task::none();
        }
        self.preview_request_id += 1;
        let req_id = self.preview_request_id;
        let cancel = CancellationToken::new();
        self.preview_cancel = Some(cancel.clone());
        let path = entry.path.clone();

        if fm_preview::is_image_extension(entry.extension.as_deref()) {
            let max_px = if self.quicklook_open { 560 } else { 160 };
            return Task::perform(
                async move {
                    let rgba = fm_preview::generate_thumbnail(&path, max_px, cancel).await;
                    (req_id, path, rgba)
                },
                |(request_id, path, rgba)| Message::QuickLookImageLoaded {
                    request_id,
                    path,
                    rgba,
                },
            );
        }

        Task::perform(
            async move {
                let snippet = fm_preview::read_text_preview(&path, 64 * 1024, cancel).await;
                (req_id, path, snippet)
            },
            |(request_id, path, snippet)| Message::TextPreviewLoaded {
                request_id,
                path,
                snippet,
            },
        )
    }

    fn spawn_deep_search_task(&mut self) -> Task<Message> {
        if let Some(prev) = self.deep_search_cancel.take() {
            prev.cancel();
        }
        let query = self.deep_search_query.trim().to_owned();
        if query.is_empty() {
            self.deep_search_loading = false;
            self.deep_search_results.clear();
            return Task::none();
        }
        let Some(root) = self.current_tab().map(|t| t.current_path.clone()) else {
            return Task::none();
        };

        self.deep_search_request_id += 1;
        let req_id = self.deep_search_request_id;
        let cancel = CancellationToken::new();
        self.deep_search_cancel = Some(cancel.clone());
        self.deep_search_loading = true;

        Task::perform(
            async move {
                let results = fm_index::deep_search(&root, &query, 100, cancel).await;
                (req_id, results)
            },
            |(request_id, results)| Message::DeepSearchResults {
                request_id,
                results,
            },
        )
    }

    fn queue_visible_thumbnails(&mut self) -> Task<Message> {
        let Some(tab) = self.current_tab() else {
            return Task::none();
        };

        let mut candidates: Vec<(ThumbnailKey, PathBuf)> = Vec::new();

        if tab.view_mode == ViewMode::Grid {
            let dirs_count = tab
                .visible_entries
                .iter()
                .filter(|e| e.kind.is_dir_like())
                .count();
            let files: Vec<&FileEntry> = tab
                .visible_entries
                .iter()
                .filter(|e| !e.kind.is_dir_like())
                .collect();

            if !files.is_empty() {
                let step = 148.0;
                let min_row = ((self.canvas_scroll_y - 32.0).max(0.0) / step).floor() as usize;
                let max_row = ((self.canvas_scroll_y + self.canvas_viewport_height + 280.0) / step)
                    .ceil() as usize;

                let start_idx = min_row.saturating_sub(dirs_count.div_ceil(2) + 2) * 2;
                let end_idx = (max_row.saturating_sub(dirs_count / 8) * 8 + 16).min(files.len());
                let clamped_start = start_idx.min(end_idx);

                for entry in &files[clamped_start..end_idx] {
                    if fm_preview::is_image_extension(entry.extension.as_deref()) {
                        candidates.push((thumbnail_key_for_entry(entry), entry.path.clone()));
                    }
                }
            }
        }

        if let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
            && !selected.kind.is_dir_like()
            && fm_preview::is_image_extension(selected.extension.as_deref())
        {
            candidates.push((thumbnail_key_for_entry(selected), selected.path.clone()));
        }

        let cancel = self
            .tab_cancels
            .get(&self.active_tab)
            .cloned()
            .unwrap_or_default();

        let mut tasks = Vec::new();
        for (key, path) in candidates {
            if tasks.len() >= 24 {
                break;
            }
            if self.gpu_thumbnails.contains_key(&key) {
                continue;
            }
            if let Some(cached) = self.thumbnail_cache.get(&key) {
                if self.gpu_thumbnails.len() >= fm_preview::DEFAULT_THUMBNAIL_CACHE_CAPACITY {
                    self.gpu_thumbnails.clear();
                }
                self.gpu_thumbnails.insert(
                    key,
                    image::Handle::from_rgba(cached.width, cached.height, cached.pixels.clone()),
                );
                continue;
            }
            if self.in_flight_thumbs.contains(&path) {
                continue;
            }
            self.in_flight_thumbs.insert(path.clone());
            let cancel = cancel.clone();
            tasks.push(Task::perform(
                async move {
                    let rgba = fm_preview::generate_thumbnail(&path, 160, cancel).await;
                    (key, rgba)
                },
                |(key, rgba)| Message::ThumbnailReady { key, rgba },
            ));
        }

        Task::batch(tasks)
    }

    fn handle_global_key(
        &mut self,
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        let text_input_modal_active = self.active_dialog.is_some()
            || self.deep_search_open
            || self.path_editing
            || self.prefs_open;
        if text_input_modal_active {
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Escape)) {
                if self.active_dialog.is_some() {
                    return self.update(Message::CloseDialog);
                } else if self.prefs_open {
                    return self.update(Message::ClosePreferences);
                } else if self.deep_search_open {
                    return self.update(Message::CloseDeepSearch);
                } else if self.path_editing {
                    self.path_editing = false;
                    return Task::none();
                }
            }
            return Task::none();
        }

        match key.as_ref() {
            keyboard::Key::Character("t") if modifiers.control() => self.update(Message::TabNew),
            keyboard::Key::Character("w") if modifiers.control() => {
                let id = self.active_tab;
                self.update(Message::TabClose(id))
            }
            keyboard::Key::Character("r") if modifiers.control() => {
                self.update(Message::RefreshActiveTab)
            }
            keyboard::Key::Named(Named::F5) => self.update(Message::RefreshActiveTab),
            keyboard::Key::Named(Named::F2) => self.update(Message::OpenRenameDialog),
            keyboard::Key::Named(Named::Delete) => {
                self.update(Message::RequestDelete(modifiers.shift()))
            }
            keyboard::Key::Character("c") if modifiers.control() => {
                self.update(Message::CopySelected)
            }
            keyboard::Key::Character("x") if modifiers.control() => {
                self.update(Message::CutSelected)
            }
            keyboard::Key::Character("v") if modifiers.control() => {
                self.update(Message::PasteClipboard)
            }
            keyboard::Key::Character("n") if modifiers.control() && modifiers.shift() => {
                self.update(Message::OpenCreateDialog(true))
            }
            keyboard::Key::Character("b") if modifiers.control() => {
                self.update(Message::ToggleSidebar)
            }
            keyboard::Key::Character("h") if modifiers.control() => {
                self.update(Message::ToggleHidden)
            }
            keyboard::Key::Character("i") if modifiers.alt() => {
                self.update(Message::ToggleInspector)
            }
            keyboard::Key::Character("k") if modifiers.control() => {
                operation::focus(search_input_id())
            }
            keyboard::Key::Character("f") if modifiers.control() && modifiers.shift() => {
                self.update(Message::OpenDeepSearch)
            }
            keyboard::Key::Character("l") if modifiers.control() => {
                self.update(Message::TogglePathEdit)
            }
            keyboard::Key::Named(Named::Space) if !modifiers.control() && !modifiers.alt() => {
                self.update(Message::ToggleQuickLook)
            }
            keyboard::Key::Named(Named::Escape) => {
                if self.quicklook_open {
                    self.update(Message::CloseQuickLook)
                } else {
                    self.update(Message::FilterChanged(String::new()))
                }
            }
            keyboard::Key::Named(Named::ArrowRight | Named::ArrowDown) => self.move_selection(1),
            keyboard::Key::Named(Named::ArrowLeft | Named::ArrowUp) => self.move_selection(-1),
            keyboard::Key::Named(Named::Enter) => {
                if let Some(focused) = self.current_tab().and_then(|t| t.selection.focused) {
                    return self.update(Message::ActivateEntry(focused));
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn move_selection(&mut self, delta: isize) -> Task<Message> {
        let mut selected_entry = None;
        if let Some(tab) = self.current_tab_mut() {
            if tab.visible_entries.is_empty() {
                return Task::none();
            }
            let len = tab.visible_entries.len() as isize;
            let current = tab.selection.focused.unwrap_or(0) as isize;
            let next = (current + delta).clamp(0, len - 1) as usize;
            tab.selection.select_single(next, &tab.visible_entries);
            let single = tab
                .selection
                .single_selected(&tab.visible_entries)
                .cloned();
            tab.selected_telemetry = single.as_ref().map(basic_file_telemetry);
            selected_entry = single;
        }
        self.dispatch_selection_side_effects(selected_entry.as_ref())
    }
}

#[allow(clippy::ptr_arg)] // Requerido por `Subscription::run_with::<PathBuf, _>` (`fn(&D) -> S`).
fn watch_directory_stream(root: &PathBuf) -> impl iced::futures::Stream<Item = Message> + use<> {
    let root = root.clone();
    stream::channel(16, async move |mut output| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let Ok(_debouncer) = fm_index::watcher::start_directory_watcher(root.clone(), tx) else {
            return;
        };
        while let Some(ev) = rx.recv().await {
            if output
                .send(Message::DirectoryChangedOnDisk(ev.watched_root))
                .await
                .is_err()
            {
                break;
            }
        }
    })
}

fn apply_tab_filter(tab: &mut TabState) {
    if tab.filter_query.trim().is_empty() {
        tab.recompute_visible();
        return;
    }

    let mut base: Vec<FileEntry> = tab
        .raw_entries
        .iter()
        .filter(|entry| tab.show_hidden || !entry.is_hidden)
        .cloned()
        .collect();
    base.sort_by(|a, b| tab.sort.compare(a, b));
    let ranked = fm_index::fuzzy_rank_entries(&base, &tab.filter_query);
    tab.telemetry.folder_stats = FolderSummary::from_entries(&ranked);
    tab.visible_entries = ranked;

    let valid_ids: HashSet<EntryId> = tab.visible_entries.iter().map(|e| e.id).collect();
    tab.selection
        .selected_ids
        .retain(|id| valid_ids.contains(id));
}

fn spawn_list_task(
    vfs: Arc<LocalFs>,
    tab_id: TabId,
    path: PathBuf,
    request_id: u64,
    cancel: CancellationToken,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = vfs.list_dir(&path, request_id, cancel).await;
            let volume_space = if result.is_ok() {
                vfs.volume_space(&path).await.ok().flatten()
            } else {
                None
            };
            (tab_id, request_id, result, volume_space)
        },
        |(tab_id, request_id, result, volume_space)| Message::DirectoryLoaded {
            tab_id,
            request_id,
            result,
            volume_space,
        },
    )
}

fn prepare_tab_for_load(tab: &mut TabState, request_id: u64) {
    tab.active_request_id = request_id;
    tab.is_loading = true;
    tab.last_error = None;
    tab.title = display_tab_title(&tab.current_path);
}

fn display_tab_title(path: &std::path::Path) -> CompactString {
    path.file_name()
        .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CompactString::from(path.to_string_lossy().as_ref()))
}

fn basic_file_telemetry(entry: &fm_core::FileEntry) -> SelectedFileTelemetry {
    let mime = if entry.kind.is_dir_like() {
        Some(CompactString::new("inode/directory"))
    } else {
        entry
            .extension
            .as_ref()
            .map(|e| CompactString::from(format!("ext/{e}")))
    };
    SelectedFileTelemetry {
        sha256_prefix: None,
        sha256_full: None,
        mime_type: mime,
        preview_snippet: None,
    }
}

pub fn selected_id_set_contains(tab: &TabState, id: EntryId) -> bool {
    tab.selection.selected_ids.contains(&id)
}
