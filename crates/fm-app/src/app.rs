use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use compact_str::CompactString;
use fm_core::{
    AppConfig, ClipboardTransfer, ConflictPolicy, CopyMode, DIR_FILTER_SENTINEL, DirectorySnapshot,
    DriveInfo, EntryExtraInfo, EntryId, FileEntry, FolderSummary, FsError, IconThemeMode,
    JobProgress, JobState, Language, PermissionsMeta, RecursiveSize, SelectedFileTelemetry,
    SelectionModifiers, SortDirection, SortField, Strings, SystemIconBucket, SystemIconKey,
    SystemPlaceKind, TabId, TabState, ThemePreset, UserPlaces, VfsProvider, ViewMode, VolumeSpace,
    WindowDecorationsMode,
};
use fm_fs::LocalFs;
use fm_index::SearchHit;
use fm_preview::{
    DiskThumbnailCache, SystemIconCache, ThumbnailCache, ThumbnailKey, ThumbnailRgba,
    rasterize_system_icon,
};
use fm_telemetry::{CachedFileTelemetry, DiskMetadataCache};
use fm_theme::{Spacing, ThemePalette};
use iced::futures::SinkExt;
use iced::keyboard;
use iced::keyboard::key::Named;
use iced::mouse;
use iced::widget::operation::{AbsoluteOffset, RelativeOffset};
use iced::widget::{Id, column, container, image, operation, row, stack, text_editor};
use iced::{
    Element, Event, Length, Point, Size, Subscription, Task, clipboard, event, stream, system,
    window,
};
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
pub enum ContextMenuKind {
    Entry {
        index: usize,
        path: PathBuf,
        is_dir: bool,
    },
    SidebarPlace {
        path: PathBuf,
        is_favorite: bool,
    },
    Drive {
        label: String,
        mount_path: PathBuf,
        device_path: Option<PathBuf>,
        is_mounted: bool,
        can_unmount: bool,
    },
    Background,
    NewMenu,
    FilterMenu,
    LanguageMenu,
    PreferencesMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreferencesTab {
    #[default]
    General,
    About,
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub kind: ContextMenuKind,
    pub position: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertiesTab {
    #[default]
    General,
    Security,
    History,
    Metadata,
    Permissions,
}

#[derive(Debug, Clone)]
pub struct PropertiesWindowState {
    pub entry: FileEntry,
    pub telemetry: Option<SelectedFileTelemetry>,
    pub recursive_size: Option<RecursiveSize>,
    pub volume_space: Option<VolumeSpace>,
    pub size_loading: bool,
    pub extra: Option<EntryExtraInfo>,
    pub active_tab: PropertiesTab,
    pub draft_name: String,
    pub readonly_draft: bool,
    pub initial_readonly: bool,
    pub hidden_draft: bool,
    pub initial_hidden: bool,
    pub posix_mode_draft: Option<u32>,
    pub initial_posix_mode: Option<u32>,
    pub applying: bool,
    pub cancel: Option<CancellationToken>,
}

impl PropertiesWindowState {
    pub fn is_archive(&self) -> bool {
        fm_fs::archive::split_archive_path(&self.entry.path).is_some()
    }

    pub fn is_dirty(&self) -> bool {
        if self.is_archive() {
            return false;
        }
        let trimmed = self.draft_name.trim();
        let name_changed = !trimmed.is_empty() && trimmed != self.entry.name.as_str();
        name_changed
            || self.readonly_draft != self.initial_readonly
            || self.hidden_draft != self.initial_hidden
            || self.posix_mode_draft != self.initial_posix_mode
    }
}

#[derive(Debug, Clone)]
pub enum ActiveDialog {
    CreateItem {
        is_dir: bool,
        name: String,
    },
    Rename {
        src: PathBuf,
        name: String,
    },
    ConfirmDelete {
        paths: Vec<PathBuf>,
        permanent: bool,
    },
    ConfirmEmptyTrash,
    Conflict {
        mode: CopyMode,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        conflicts: usize,
    },
}

pub fn canvas_scroll_id() -> Id {
    Id::new("fm_canvas_scroll")
}

pub fn entry_mtime_nanos(modified_at: Option<std::time::SystemTime>) -> u128 {
    modified_at
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

pub fn thumbnail_key_for_entry(entry: &FileEntry) -> ThumbnailKey {
    ThumbnailKey {
        path: entry.path.clone(),
        size_bytes: entry.size_bytes,
        modified_nanos: entry_mtime_nanos(entry.modified_at),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTarget {
    Home,
    Desktop,
    Documents,
    Downloads,
    Music,
    Pictures,
    Videos,
    PublicDir,
    Templates,
    Trash,
}

impl SidebarTarget {
    pub fn resolve_path(self, places: &UserPlaces) -> Option<PathBuf> {
        match self {
            Self::Home => Some(places.home.clone()),
            Self::Desktop => places.desktop.clone(),
            Self::Documents => places.documents.clone(),
            Self::Downloads => places.downloads.clone(),
            Self::Music => places.music.clone(),
            Self::Pictures => places.pictures.clone(),
            Self::Videos => places.videos.clone(),
            Self::PublicDir => places.public_dir.clone(),
            Self::Templates => places.templates.clone(),
            Self::Trash => places.trash.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabNew,
    TabSelect(TabId),
    TabNext(bool),
    TabClose(TabId),
    OpenInNewTab(PathBuf),
    NavigateTo(PathBuf),
    NavigateBack,
    NavigateForward,
    NavigateUp,
    RefreshActiveTab,
    SelectSidebar(SidebarTarget),
    ToggleSidebarHomeExpanded,
    HomeSubdirsLoaded(Vec<PathBuf>),
    SelectDrive(PathBuf),
    RefreshDrives,
    MountAndOpenDrive(PathBuf),
    DriveMounted {
        device_path: PathBuf,
        result: Result<PathBuf, FsError>,
    },
    UnmountDrive {
        device_path: Option<PathBuf>,
        mount_path: Option<PathBuf>,
        label: String,
    },
    DriveUnmounted {
        mount_path: Option<PathBuf>,
        label: String,
        result: Result<(), FsError>,
    },
    OpenDriveContextMenu {
        label: String,
        mount_path: PathBuf,
        device_path: Option<PathBuf>,
        is_mounted: bool,
        can_unmount: bool,
    },
    SelectEntry {
        index: usize,
        modifiers: SelectionModifiers,
    },
    ClearSelection,
    MarqueeSelect {
        rect: views::virtualized::ContentRect,
        viewport_width: f32,
        additive: bool,
        is_initial: bool,
    },
    MarqueeFinished,
    SelectAll,
    ActivateEntry(usize),
    SetViewMode(ViewMode),
    CycleSortField,
    SortByField(SortField),
    ToggleSortDirection,
    ToggleHidden,
    FilterChanged(String),
    TogglePathEdit,
    PathDraftChanged(String),
    PathAutocompleteLoaded {
        base_draft: String,
        parent: PathBuf,
        prefix: String,
        dir_names: Vec<String>,
    },
    CommitPathDraft,
    ToggleTheme,
    SystemThemeDetected(iced::theme::Mode),
    OpenLanguageMenu,
    SetLanguage(Language),
    UseSystemLanguage,
    ToggleSidebar,
    ToggleInspector,
    ToggleHeroCard,
    ToggleQuickLook,
    CloseQuickLook,
    CanvasScrolled {
        offset_y: f32,
        viewport_height: f32,
        viewport_width: f32,
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
    DirectoryChangedOnDisk {
        watched_root: PathBuf,
        has_visible_change: bool,
    },
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
    FolderSizeReady {
        tab_id: TabId,
        request_id: u64,
        size: Option<RecursiveSize>,
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
    GitStatusDetected {
        request_id: u64,
        path: PathBuf,
        status: Option<CompactString>,
    },
    DiskCacheReady(Option<Arc<DiskThumbnailCache>>),
    OpenTerminal,
    TerminalOpened(Result<(), FsError>),
    OpenAsRoot(PathBuf),
    OpenAsRootFinished(Result<(), FsError>),
    ToggleBreadcrumbMenu(PathBuf),
    CloseBreadcrumbMenu,
    BreadcrumbMenuLoaded {
        path: PathBuf,
        folders: Vec<FileEntry>,
    },
    ResolveConflict(ConflictPolicy),
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
    RemoveFavorite(PathBuf),
    OpenSidebarContextMenu {
        path: PathBuf,
        is_favorite: bool,
    },
    OpenPreferences,
    OpenAbout,
    SelectPreferencesTab(PreferencesTab),
    OpenPreferencesContextMenu,
    ClosePreferences,
    SetDefaultViewMode(ViewMode),
    ToggleGridGroupFolders,
    ToggleGridCenterItems,
    SetGridCenterItems(bool),
    ToggleSystemIcons,
    SystemIconReady {
        generation: u64,
        key: SystemIconKey,
        bucket: SystemIconBucket,
        rgba: Option<Arc<ThumbnailRgba>>,
    },
    ToggleDirsFirst,
    ToggleWindowDecorations,
    RequestEmptyTrash,
    RestoreFromTrash,
    TrashActionFinished {
        notice: String,
        result: Result<(), FsError>,
    },
    ConfigSaved(Option<String>),
    OpenDefaultFinished(Result<(), FsError>),
    OpenProperties(PathBuf),
    PropertiesEntryStatLoaded(Result<FileEntry, FsError>),
    PropertiesFolderSizeReady {
        window_id: window::Id,
        path: PathBuf,
        size: Option<RecursiveSize>,
    },
    PropertiesVolumeSpaceReady {
        window_id: window::Id,
        path: PathBuf,
        space: Option<VolumeSpace>,
    },
    PropertiesExtraInfoReady {
        window_id: window::Id,
        path: PathBuf,
        extra: EntryExtraInfo,
    },
    PropertiesTelemetryReady {
        window_id: window::Id,
        path: PathBuf,
        mime: Option<CompactString>,
        sha: Option<(CompactString, CompactString)>,
        git_status: Option<CompactString>,
    },
    PropertiesSelectTab(window::Id, PropertiesTab),
    PropertiesNameChanged(window::Id, String),
    PropertiesToggleReadonly(window::Id),
    PropertiesToggleHidden(window::Id),
    PropertiesTogglePosixBit(window::Id, u32),
    PropertiesApply {
        window_id: window::Id,
        close_after: bool,
    },
    PropertiesApplied {
        window_id: window::Id,
        original_path: PathBuf,
        close_after: bool,
        result: Result<(FileEntry, EntryExtraInfo), FsError>,
    },
    PropertiesOpenDefault(PathBuf),
    DragPropertiesWindow(window::Id),
    ClosePropertiesWindow(window::Id),
    OpenNewMenu,
    OpenFilterMenu,
    SetExtensionFilter(Option<CompactString>),
    OpenEntryContextMenu(usize),
    OpenCanvasContextMenu,
    CloseContextMenu,
    CopyPath(PathBuf),
    CopyText(String, String),
    StartColumnResize(usize),
    StartGridResize,
    StartColumnsPanelResize,
    StartSidebarResize,
    SidebarDividerHovered(bool),
    ResetSidebarWidth,
    EndColumnResize(window::Id),
    StepGridCardWidth(f32),
    ResetGridCardWidth,
    CtrlWheelZoom(mouse::ScrollDelta),
    FlushZoomConfig(u64),
    CursorMoved {
        window_id: window::Id,
        position: Point,
    },
    WindowResized {
        window_id: window::Id,
        size: Size,
    },
    MonitorSizeReady(Option<Size>),
    WindowClosed(window::Id),
    FileDropped {
        window_id: window::Id,
        path: PathBuf,
    },
    FlushDroppedFiles(u64),
    WindowMinimize,
    WindowToggleMaximize,
    WindowClose,
    ModifiersChanged(keyboard::Modifiers),
    WindowUnfocused(window::Id),
    GlobalKeyPressed {
        window_id: window::Id,
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    },
    MetaCacheReady(Option<Arc<DiskMetadataCache>>),
    ColumnAncestorsLoaded {
        request_id: u64,
        ancestors: Vec<(PathBuf, Vec<FileEntry>)>,
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
    pub column_ancestors: Vec<(PathBuf, Vec<FileEntry>)>,
    pub column_ancestors_request_id: u64,
    pub column_ancestors_cancel: Option<CancellationToken>,
    pub table_col_weights: [f32; 5],
    pub resizing_col: Option<(usize, f32, [f32; 5])>,
    pub resizing_grid: Option<(f32, f32)>,
    pub columns_panel_width: f32,
    pub resizing_columns_panel: Option<(f32, f32)>,
    pub resizing_sidebar: Option<(f32, f32)>,
    pub sidebar_divider_hovered: bool,
    pub deep_search_open: bool,
    pub deep_search_query: String,
    pub deep_search_loading: bool,
    pub deep_search_results: Vec<SearchHit>,
    pub deep_search_request_id: u64,
    pub deep_search_cancel: Option<CancellationToken>,
    pub active_dialog: Option<ActiveDialog>,
    pub prefs_open: bool,
    pub prefs_tab: PreferencesTab,
    pub clipboard: Option<ClipboardTransfer>,
    pub next_job_id: u64,
    pub active_transfer: Option<JobProgress>,
    pub transfer_cancel: Option<CancellationToken>,
    pub pending_select_path: Option<PathBuf>,
    pub marquee_base_selection: HashSet<EntryId>,
    pub metadata_cache: Option<Arc<DiskMetadataCache>>,
    pub disk_thumbnail_cache: Option<Arc<DiskThumbnailCache>>,
    pub thumbnail_cache: ThumbnailCache,
    pub gpu_thumbnails: HashMap<ThumbnailKey, image::Handle>,
    pub in_flight_thumbs: HashSet<PathBuf>,
    pub system_icon_cache: SystemIconCache,
    pub gpu_system_icons: HashMap<(SystemIconKey, SystemIconBucket), image::Handle>,
    pub in_flight_system_icons: HashSet<(SystemIconKey, SystemIconBucket)>,
    pub system_icon_generation: u64,
    pub breadcrumb_menu: Option<(PathBuf, Vec<FileEntry>)>,
    pub breadcrumb_menu_cancel: Option<CancellationToken>,
    pub quicklook_editor: text_editor::Content,
    pub quicklook_extension: String,
    pub quicklook_image: Option<(PathBuf, image::Handle)>,
    pub drives: Vec<DriveInfo>,
    pub home_subdirs: Vec<PathBuf>,
    pub sidebar_home_expanded: bool,
    pub is_elevated: bool,
    pub active_sidebar: Option<SidebarTarget>,
    pub path_editing: bool,
    pub path_draft: String,
    pub quicklook_open: bool,
    pub current_modifiers: keyboard::Modifiers,
    pub cursor_position: Point,
    pub window_size: Size,
    pub context_menu: Option<ContextMenuState>,
    pub pending_dropped_paths: Vec<PathBuf>,
    pub drop_batch_id: u64,
    pub canvas_scroll_y: f32,
    pub canvas_viewport_height: f32,
    pub canvas_viewport_width: Option<f32>,
    pub main_window_id: window::Id,
    pub properties_windows: HashMap<window::Id, PropertiesWindowState>,
    pub status_notice: Option<String>,
    pub last_entry_click: Option<(TabId, PathBuf, Instant)>,
    pub last_watcher_reload: Option<(PathBuf, Instant)>,
    pub grid_zoom_dirty: bool,
    pub zoom_save_seq: u64,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let vfs = Arc::new(LocalFs::new());
        let places = fm_fs::user_places();
        let mut config = fm_fs::load_config();
        if !config.language_customized {
            config.language = Language::detect_system();
        }

        let cli_initial = std::env::args_os()
            .nth(1)
            .and_then(|arg| resolve_cli_initial_dir(&arg));
        let tab1_path = cli_initial
            .or_else(|| places.documents.clone().filter(|p| p.exists()))
            .unwrap_or_else(|| places.home.clone());
        let tab2_path = std::env::current_dir()
            .ok()
            .filter(|p| p.exists())
            .unwrap_or_else(|| places.home.clone());

        let mut tab1 = TabState::new(TabId(1), tab1_path.clone());
        tab1.view_mode = config.default_view_mode;
        tab1.sort.dirs_first = config.dirs_first;
        tab1.is_loading = true;
        tab1.active_request_id = 1;

        let mut tab2 = TabState::new(TabId(2), tab2_path.clone());
        tab2.view_mode = config.default_view_mode;
        tab2.sort.dirs_first = config.dirs_first;
        tab2.is_loading = true;
        tab2.active_request_id = 2;

        let cancel1 = CancellationToken::new();
        let cancel2 = CancellationToken::new();

        let mut tab_cancels = HashMap::new();
        tab_cancels.insert(TabId(1), cancel1.clone());
        tab_cancels.insert(TabId(2), cancel2.clone());

        let initial_size = Size::new(1120.0, 660.0);
        let (main_window_id, open_main_task) = window::open(window::Settings {
            size: initial_size,
            min_size: Some(Size::new(760.0, 500.0)),
            position: window::Position::Centered,
            decorations: config.window_decorations == WindowDecorationsMode::NativeWm,
            ..window::Settings::default()
        });

        let mut app = Self {
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
            column_ancestors: Vec::new(),
            column_ancestors_request_id: 0,
            column_ancestors_cancel: None,
            table_col_weights: [48.0, 11.0, 9.0, 17.0, 15.0],
            resizing_col: None,
            resizing_grid: None,
            columns_panel_width: 280.0,
            resizing_columns_panel: None,
            resizing_sidebar: None,
            sidebar_divider_hovered: false,
            deep_search_open: false,
            deep_search_query: String::new(),
            deep_search_loading: false,
            deep_search_results: Vec::new(),
            deep_search_request_id: 0,
            deep_search_cancel: None,
            active_dialog: None,
            prefs_open: false,
            prefs_tab: PreferencesTab::General,
            clipboard: None,
            next_job_id: 1,
            active_transfer: None,
            transfer_cancel: None,
            pending_select_path: None,
            marquee_base_selection: HashSet::new(),
            metadata_cache: None,
            disk_thumbnail_cache: None,
            thumbnail_cache: ThumbnailCache::default(),
            gpu_thumbnails: HashMap::new(),
            in_flight_thumbs: HashSet::new(),
            system_icon_cache: SystemIconCache::default(),
            gpu_system_icons: HashMap::new(),
            in_flight_system_icons: HashSet::new(),
            system_icon_generation: 0,
            breadcrumb_menu: None,
            breadcrumb_menu_cancel: None,
            quicklook_editor: text_editor::Content::new(),
            quicklook_extension: String::new(),
            quicklook_image: None,
            drives: Vec::new(),
            home_subdirs: Vec::new(),
            sidebar_home_expanded: false,
            is_elevated: fm_fs::is_elevated_session(),
            active_sidebar: Some(SidebarTarget::Documents),
            path_editing: false,
            path_draft: String::new(),
            quicklook_open: false,
            current_modifiers: keyboard::Modifiers::empty(),
            cursor_position: Point::new(240.0, 140.0),
            window_size: initial_size,
            context_menu: None,
            pending_dropped_paths: Vec::new(),
            drop_batch_id: 0,
            canvas_scroll_y: 0.0,
            canvas_viewport_height: 700.0,
            canvas_viewport_width: None,
            main_window_id,
            properties_windows: HashMap::new(),
            status_notice: None,
            last_entry_click: None,
            last_watcher_reload: None,
            grid_zoom_dirty: false,
            zoom_save_seq: 0,
        };

        let auto_theme_task = if !app.config.theme_customized {
            system::theme().map(Message::SystemThemeDetected)
        } else {
            Task::none()
        };

        let drives_task = {
            let vfs = Arc::clone(&vfs);
            Task::perform(
                async move { vfs.list_drives().await },
                Message::DrivesLoaded,
            )
        };
        let home_subdirs_task = Task::perform(
            fm_fs::list_home_subdirs(app.places.home.clone()),
            Message::HomeSubdirsLoaded,
        );
        let disk_cache_task = Task::perform(
            async {
                tokio::task::spawn_blocking(DiskThumbnailCache::open_default)
                    .await
                    .ok()
                    .flatten()
                    .map(Arc::new)
            },
            Message::DiskCacheReady,
        );
        let meta_cache_task = Task::perform(
            async {
                tokio::task::spawn_blocking(DiskMetadataCache::open_default)
                    .await
                    .ok()
                    .flatten()
                    .map(Arc::new)
            },
            Message::MetaCacheReady,
        );
        let vibrancy_task = apply_window_vibrancy(main_window_id, app.config.theme);
        let load1 = spawn_list_task(Arc::clone(&vfs), TabId(1), tab1_path, 1, cancel1);
        let load2 = spawn_list_task(vfs, TabId(2), tab2_path, 2, cancel2);
        let sys_icons_task = app.queue_visible_system_icons();

        let monitor_fit_task = open_main_task
            .then(window::monitor_size)
            .map(Message::MonitorSizeReady);

        (
            app,
            Task::batch([
                monitor_fit_task,
                auto_theme_task,
                drives_task,
                home_subdirs_task,
                disk_cache_task,
                meta_cache_task,
                vibrancy_task,
                load1,
                load2,
                sys_icons_task,
            ]),
        )
    }

    pub fn title(&self, window_id: window::Id) -> String {
        if let Some(props) = self.properties_windows.get(&window_id) {
            return format!(
                "{}{}",
                self.strings().properties_header_prefix,
                views::sanitize_single_line(props.entry.name.as_str())
            );
        }
        let tab_title = self
            .current_tab()
            .map(|t| t.title.as_str())
            .unwrap_or("Vagnarok");
        format!("Vagnarok File Manager — {tab_title}")
    }

    pub fn iced_theme(&self, _window_id: window::Id) -> iced::Theme {
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
                self.context_menu = None;
                let path = self
                    .current_tab()
                    .map(|t| t.current_path.clone())
                    .unwrap_or_else(|| self.places.home.clone());
                self.open_tab_at(path)
            }
            Message::OpenInNewTab(path) => {
                self.context_menu = None;
                self.open_tab_at(path)
            }
            Message::TabNext(reverse) => {
                self.context_menu = None;
                if self.tabs.len() <= 1 {
                    return Task::none();
                }
                if let Some(pos) = self.tabs.iter().position(|t| t.id == self.active_tab) {
                    let next = if reverse {
                        if pos == 0 {
                            self.tabs.len() - 1
                        } else {
                            pos - 1
                        }
                    } else {
                        (pos + 1) % self.tabs.len()
                    };
                    let id = self.tabs[next].id;
                    return self.update(Message::TabSelect(id));
                }
                Task::none()
            }
            Message::TabSelect(id) => {
                self.context_menu = None;
                if self.tabs.iter().any(|t| t.id == id) {
                    self.active_tab = id;
                    self.path_editing = false;
                    self.quicklook_open = false;
                    self.canvas_scroll_y = 0.0;
                    let thumb_task = self.queue_visible_thumbnails();
                    let anc_task = self.maybe_load_column_ancestors();
                    return Task::batch([
                        operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
                        thumb_task,
                        anc_task,
                    ]);
                }
                Task::none()
            }
            Message::TabClose(id) => {
                self.context_menu = None;
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
            Message::NavigateTo(path) => {
                self.context_menu = None;
                self.navigate_active_tab(path)
            }
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
                    .filter(|t| !fm_core::is_virtual_trash_path(&t.current_path))
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
            Message::ToggleSidebarHomeExpanded => {
                self.sidebar_home_expanded = !self.sidebar_home_expanded;
                if self.sidebar_home_expanded {
                    let subdirs_task = Task::perform(
                        fm_fs::list_home_subdirs(self.places.home.clone()),
                        Message::HomeSubdirsLoaded,
                    );
                    Task::batch([subdirs_task, self.queue_visible_system_icons()])
                } else {
                    Task::none()
                }
            }
            Message::HomeSubdirsLoaded(dirs) => {
                self.home_subdirs = dirs;
                Task::none()
            }
            Message::SelectDrive(path) => {
                self.active_sidebar = None;
                self.navigate_active_tab(path)
            }
            Message::RefreshDrives => {
                let vfs = Arc::clone(&self.vfs);
                Task::perform(
                    async move { vfs.list_drives().await },
                    Message::DrivesLoaded,
                )
            }
            Message::MountAndOpenDrive(device_path) => {
                self.context_menu = None;
                let dev_for_cb = device_path.clone();
                Task::perform(fm_fs::mount_drive(device_path), move |result| {
                    Message::DriveMounted {
                        device_path: dev_for_cb,
                        result,
                    }
                })
            }
            Message::DriveMounted {
                device_path,
                result,
            } => {
                let refresh_task = self.update(Message::RefreshDrives);
                match result {
                    Ok(mount_path) => {
                        self.status_notice = Some(format!(
                            "{}: {}",
                            self.strings().mount_drive,
                            mount_path.to_string_lossy()
                        ));
                        let nav_task = self.navigate_active_tab(mount_path);
                        Task::batch([refresh_task, nav_task])
                    }
                    Err(err) => {
                        let _ = device_path;
                        self.status_notice = Some(err.to_string());
                        refresh_task
                    }
                }
            }
            Message::UnmountDrive {
                device_path,
                mount_path,
                label,
            } => {
                self.context_menu = None;
                let mp_for_cb = mount_path.clone();
                let label_for_cb = label.clone();
                Task::perform(
                    fm_fs::unmount_drive(device_path, mount_path),
                    move |result| Message::DriveUnmounted {
                        mount_path: mp_for_cb,
                        label: label_for_cb,
                        result,
                    },
                )
            }
            Message::DriveUnmounted {
                mount_path,
                label,
                result,
            } => {
                let refresh_task = self.update(Message::RefreshDrives);
                match result {
                    Ok(()) => {
                        self.status_notice =
                            Some(format!("{}: {label}", self.strings().unmount_drive));
                        let nav_task = if let Some(mp) = mount_path
                            && self
                                .current_tab()
                                .is_some_and(|t| t.current_path.starts_with(&mp))
                        {
                            self.navigate_active_tab(self.places.home.clone())
                        } else {
                            Task::none()
                        };
                        Task::batch([refresh_task, nav_task])
                    }
                    Err(err) => {
                        self.status_notice = Some(err.to_string());
                        refresh_task
                    }
                }
            }
            Message::OpenDriveContextMenu {
                label,
                mount_path,
                device_path,
                is_mounted,
                can_unmount,
            } => {
                self.breadcrumb_menu = None;
                self.context_menu = Some(ContextMenuState {
                    kind: ContextMenuKind::Drive {
                        label,
                        mount_path,
                        device_path,
                        is_mounted,
                        can_unmount,
                    },
                    position: self.cursor_position,
                });
                Task::none()
            }
            Message::SelectEntry { index, modifiers } => {
                self.context_menu = None;
                let clicked_entry = self
                    .current_tab()
                    .and_then(|t| t.visible_entries.get(index))
                    .map(|e| (self.active_tab, e.path.clone()));
                if !modifiers.ctrl
                    && !modifiers.shift
                    && let Some((tab_id, path)) = clicked_entry.as_ref()
                    && let Some((prev_tab, prev_path, prev_at)) = &self.last_entry_click
                    && prev_tab == tab_id
                    && prev_path == path
                    && prev_at.elapsed() <= Duration::from_millis(450)
                {
                    self.last_entry_click = None;
                    return self.update(Message::ActivateEntry(index));
                }
                self.last_entry_click =
                    clicked_entry.map(|(tab_id, path)| (tab_id, path, Instant::now()));

                let mut single_entry = None;
                if let Some(tab) = self.current_tab_mut() {
                    tab.selection
                        .apply_click(index, modifiers, &tab.visible_entries);
                    single_entry = tab.selection.single_selected(&tab.visible_entries).cloned();
                    tab.selected_telemetry = single_entry.as_ref().map(basic_file_telemetry);
                }
                self.dispatch_selection_side_effects(single_entry.as_ref())
            }
            Message::ClearSelection => {
                self.context_menu = None;
                self.last_entry_click = None;
                self.marquee_base_selection.clear();
                if let Some(tab) = self.current_tab_mut() {
                    tab.selection.clear();
                    tab.selected_telemetry = None;
                }
                Task::none()
            }
            Message::MarqueeSelect {
                rect,
                viewport_width,
                additive,
                is_initial,
            } => {
                self.context_menu = None;
                self.last_entry_click = None;
                let strings = self.strings();
                let card_width = self.config.grid_card_width;
                let group_folders = self.config.grid_group_folders;
                let center_items = self.config.grid_center_items;
                if is_initial {
                    self.marquee_base_selection = if additive {
                        self.current_tab()
                            .map(|t| t.selection.selected_ids.clone())
                            .unwrap_or_default()
                    } else {
                        HashSet::new()
                    };
                }
                let base_selection = self.marquee_base_selection.clone();
                if let Some(tab) = self.current_tab_mut() {
                    let hits = match tab.view_mode {
                        ViewMode::Grid => views::virtualized::grid_marquee_hits(
                            &tab.visible_entries,
                            viewport_width,
                            card_width,
                            group_folders,
                            center_items,
                            strings,
                            rect,
                        ),
                        ViewMode::DetailTable => views::virtualized::list_marquee_hits(
                            tab.visible_entries.len(),
                            viewport_width,
                            views::virtualized::DETAIL_LIST_SIDE_PADDING,
                            rect,
                        ),
                        ViewMode::Columns => views::virtualized::list_marquee_hits(
                            tab.visible_entries.len(),
                            viewport_width,
                            0.0,
                            rect,
                        ),
                    };

                    tab.selection.selected_ids = base_selection;
                    for &idx in &hits {
                        if let Some(entry) = tab.visible_entries.get(idx) {
                            tab.selection.selected_ids.insert(entry.id);
                        }
                    }
                    if let (Some(&first), Some(&last)) = (hits.first(), hits.last()) {
                        if tab.selection.anchor.is_none() || !additive {
                            tab.selection.anchor = Some(first);
                        }
                        tab.selection.focused = Some(last);
                    } else if !additive {
                        tab.selection.anchor = None;
                        tab.selection.focused = None;
                    }
                    let single = tab.selection.single_selected(&tab.visible_entries).cloned();
                    tab.selected_telemetry = single.as_ref().map(basic_file_telemetry);
                }
                Task::none()
            }
            Message::MarqueeFinished => {
                self.marquee_base_selection.clear();
                let single = self
                    .current_tab()
                    .and_then(|t| t.selection.single_selected(&t.visible_entries).cloned());
                self.dispatch_selection_side_effects(single.as_ref())
            }
            Message::SelectAll => {
                self.context_menu = None;
                let mut single_entry = None;
                if let Some(tab) = self.current_tab_mut() {
                    tab.selection.select_all(&tab.visible_entries);
                    single_entry = tab.selection.single_selected(&tab.visible_entries).cloned();
                    tab.selected_telemetry = single_entry.as_ref().map(basic_file_telemetry);
                }
                self.dispatch_selection_side_effects(single_entry.as_ref())
            }
            Message::ActivateEntry(index) => {
                self.context_menu = None;
                let target = self
                    .current_tab()
                    .and_then(|t| t.visible_entries.get(index).cloned());
                if let Some(entry) = target {
                    if entry.kind.is_dir_like() || fm_fs::is_archive_file(&entry.path) {
                        self.active_sidebar = None;
                        return self.navigate_active_tab(entry.path);
                    }
                    if fm_fs::archive::split_archive_path(&entry.path).is_some() {
                        return self.update(Message::ToggleQuickLook);
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
                let anc_task = self.maybe_load_column_ancestors();
                let thumb_task = self.queue_visible_thumbnails();
                Task::batch([
                    operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
                    thumb_task,
                    col_task,
                    anc_task,
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
            Message::SortByField(field) => {
                if let Some(tab) = self.current_tab_mut() {
                    if tab.sort.field == field {
                        tab.sort.direction = tab.sort.direction.toggled();
                    } else {
                        tab.sort.field = field;
                        tab.sort.direction = SortDirection::Ascending;
                    }
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
            Message::PathAutocompleteLoaded {
                base_draft,
                parent,
                prefix,
                dir_names,
            } => {
                if self.path_editing && self.path_draft == base_draft {
                    return self.apply_path_completion(&parent, &prefix, &dir_names);
                }
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
                self.config.theme_customized = true;
                self.config.theme = self.config.theme.toggled();
                let persist = self.persist_config_task();
                let vibrancy = apply_window_vibrancy(self.main_window_id, self.config.theme);
                Task::batch([persist, vibrancy])
            }
            Message::SystemThemeDetected(mode) => {
                if !self.config.theme_customized {
                    self.config.theme = match mode {
                        iced::theme::Mode::Light => ThemePreset::AetherLight,
                        iced::theme::Mode::Dark | iced::theme::Mode::None => {
                            ThemePreset::ObsidianFlow
                        }
                    };
                    return apply_window_vibrancy(self.main_window_id, self.config.theme);
                }
                Task::none()
            }
            Message::OpenLanguageMenu => {
                self.breadcrumb_menu = None;
                let is_open = matches!(
                    self.context_menu.as_ref().map(|m| &m.kind),
                    Some(ContextMenuKind::LanguageMenu)
                );
                if is_open {
                    self.context_menu = None;
                } else {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::LanguageMenu,
                        position: self.cursor_position,
                    });
                }
                Task::none()
            }
            Message::SetLanguage(lang) => {
                self.context_menu = None;
                self.config.language = lang;
                self.config.language_customized = true;
                self.persist_config_task()
            }
            Message::UseSystemLanguage => {
                self.context_menu = None;
                self.config.language_customized = false;
                self.config.language = Language::detect_system();
                self.persist_config_task()
            }
            Message::ToggleSidebar => {
                self.config.sidebar_collapsed = !self.config.sidebar_collapsed;
                self.canvas_viewport_width = None;
                self.persist_config_task()
            }
            Message::ToggleInspector => {
                self.config.show_inspector = !self.config.show_inspector;
                self.canvas_viewport_width = None;
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
                self.context_menu = None;
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
                viewport_width,
            } => {
                self.canvas_scroll_y = offset_y;
                if viewport_height > 0.0 {
                    self.canvas_viewport_height = viewport_height;
                }
                if viewport_width > 0.0 {
                    self.canvas_viewport_width = Some(viewport_width);
                }
                self.queue_visible_thumbnails()
            }
            Message::DrivesLoaded(res) => {
                if let Ok(drives) = res {
                    self.drives = drives;
                }
                self.queue_visible_system_icons()
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
                            let prev_counts: HashMap<PathBuf, u32> = tab
                                .raw_entries
                                .iter()
                                .filter_map(|e| e.child_count.map(|c| (e.path.clone(), c)))
                                .collect();
                            let mut entries = snapshot.entries;
                            if !prev_counts.is_empty() {
                                for entry in &mut entries {
                                    if let Some(&count) = prev_counts.get(&entry.path) {
                                        entry.child_count = Some(count);
                                    }
                                }
                            }
                            tab.raw_entries = entries;
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
                                let single =
                                    tab.selection.single_selected(&tab.visible_entries).cloned();
                                tab.selected_telemetry = single.as_ref().map(basic_file_telemetry);
                                pending_selected_entry = single;
                            }

                            let cancel = self.tab_cancels.get(&tab_id).cloned().unwrap_or_default();
                            let dir_path = tab.current_path.clone();
                            tasks.push(Task::perform(
                                {
                                    let cancel = cancel.clone();
                                    let dir_path = dir_path.clone();
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

                            if fm_fs::archive::split_archive_path(&dir_path).is_none()
                                && !fm_core::is_virtual_trash_path(&dir_path)
                            {
                                let meta_cache = self.metadata_cache.clone();
                                let cancel_size = cancel.clone();
                                let size_dir = dir_path.clone();
                                tasks.push(Task::perform(
                                    async move {
                                        let size = fm_telemetry::compute_folder_size_cached(
                                            size_dir,
                                            meta_cache,
                                            cancel_size,
                                        )
                                        .await;
                                        (tab_id, request_id, size)
                                    },
                                    |(tab_id, request_id, size)| Message::FolderSizeReady {
                                        tab_id,
                                        request_id,
                                        size,
                                    },
                                ));
                            }

                            let child_dirs: Vec<PathBuf> = tab
                                .raw_entries
                                .iter()
                                .filter(|e| e.kind.is_dir_like())
                                .take(64)
                                .map(|e| e.path.clone())
                                .collect();
                            if !child_dirs.is_empty() {
                                let meta_cache = self.metadata_cache.clone();
                                tasks.push(Task::perform(
                                    async move {
                                        let counts = fm_telemetry::count_direct_children_cached(
                                            child_dirs, meta_cache, cancel,
                                        )
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

                if tab_id == self.active_tab {
                    tasks.push(self.maybe_load_column_ancestors());
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
                self.quicklook_image = rgba.map(|thumb| {
                    (
                        path,
                        image::Handle::from_rgba(thumb.width, thumb.height, thumb.pixels.clone()),
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
            Message::DirectoryChangedOnDisk {
                watched_root,
                has_visible_change,
            } => {
                let Some(tab) = self.current_tab() else {
                    return Task::none();
                };
                if tab.current_path != watched_root || tab.is_loading {
                    return Task::none();
                }
                if !has_visible_change && !tab.show_hidden {
                    return Task::none();
                }
                let now = Instant::now();
                if let Some((prev_path, prev_at)) = &self.last_watcher_reload
                    && prev_path == &watched_root
                    && now.duration_since(*prev_at) < Duration::from_millis(600)
                {
                    return Task::none();
                }
                self.last_watcher_reload = Some((watched_root.clone(), now));
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
            Message::FolderSizeReady {
                tab_id,
                request_id,
                size,
            } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id)
                    && tab.active_request_id == request_id
                {
                    tab.telemetry.recursive_size = size;
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
                let mut persist_entry: Option<(FileEntry, SelectedFileTelemetry)> = None;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                    && selected.path == path
                {
                    let mut tel = tab
                        .selected_telemetry
                        .clone()
                        .unwrap_or_else(|| basic_file_telemetry(selected));
                    if mime.is_some() {
                        tel.mime_type = mime.clone();
                    }
                    persist_entry = Some((selected.clone(), tel.clone()));
                    tab.selected_telemetry = Some(tel);
                }
                for props in self.properties_windows.values_mut() {
                    if props.entry.path == path && mime.is_some() {
                        let mut tel = props
                            .telemetry
                            .take()
                            .unwrap_or_else(|| basic_file_telemetry(&props.entry));
                        tel.mime_type = mime.clone();
                        props.telemetry = Some(tel);
                    }
                }
                if let Some((entry, tel)) = persist_entry {
                    self.persist_file_telemetry(&entry, &tel);
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
                let mut persist_entry: Option<(FileEntry, SelectedFileTelemetry)> = None;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                    && selected.path == path
                {
                    let mut tel = tab
                        .selected_telemetry
                        .clone()
                        .unwrap_or_else(|| basic_file_telemetry(selected));
                    if let Some((ref prefix, ref full)) = sha {
                        tel.sha256_prefix = Some(prefix.clone());
                        tel.sha256_full = Some(full.clone());
                    }
                    persist_entry = Some((selected.clone(), tel.clone()));
                    tab.selected_telemetry = Some(tel);
                }
                for props in self.properties_windows.values_mut() {
                    if props.entry.path == path
                        && let Some((ref prefix, ref full)) = sha
                    {
                        let mut tel = props
                            .telemetry
                            .take()
                            .unwrap_or_else(|| basic_file_telemetry(&props.entry));
                        tel.sha256_prefix = Some(prefix.clone());
                        tel.sha256_full = Some(full.clone());
                        props.telemetry = Some(tel);
                    }
                }
                if let Some((entry, tel)) = persist_entry {
                    self.persist_file_telemetry(&entry, &tel);
                }
                Task::none()
            }
            Message::GitStatusDetected {
                request_id,
                path,
                status,
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
                    tel.git_status = status.clone();
                    tab.selected_telemetry = Some(tel);
                }
                for props in self.properties_windows.values_mut() {
                    if props.entry.path == path {
                        let mut tel = props
                            .telemetry
                            .take()
                            .unwrap_or_else(|| basic_file_telemetry(&props.entry));
                        tel.git_status = status.clone();
                        props.telemetry = Some(tel);
                    }
                }
                Task::none()
            }
            Message::DiskCacheReady(cache) => {
                self.disk_thumbnail_cache = cache;
                self.queue_visible_thumbnails()
            }
            Message::OpenTerminal => {
                self.context_menu = None;
                let Some(path) = self.current_tab().map(|t| t.current_path.clone()) else {
                    return Task::none();
                };
                let vfs = Arc::clone(&self.vfs);
                Task::perform(
                    async move { vfs.open_terminal(&path).await },
                    Message::TerminalOpened,
                )
            }
            Message::TerminalOpened(res) => {
                if let Err(err) = res {
                    self.status_notice = Some(err.to_string());
                }
                Task::none()
            }
            Message::OpenAsRoot(path) => {
                self.context_menu = None;
                Task::perform(fm_fs::open_as_root(path), Message::OpenAsRootFinished)
            }
            Message::OpenAsRootFinished(res) => {
                if let Err(err) = res {
                    self.status_notice = Some(err.to_string());
                }
                Task::none()
            }
            Message::ToggleBreadcrumbMenu(path) => {
                if let Some(cancel) = self.breadcrumb_menu_cancel.take() {
                    cancel.cancel();
                }
                if self
                    .breadcrumb_menu
                    .as_ref()
                    .is_some_and(|(cur, _)| cur == &path)
                {
                    self.breadcrumb_menu = None;
                    return Task::none();
                }
                let cancel = CancellationToken::new();
                self.breadcrumb_menu_cancel = Some(cancel.clone());
                self.breadcrumb_menu = Some((path.clone(), Vec::new()));
                let show_hidden = self.current_tab().is_some_and(|t| t.show_hidden);
                let sort = self.current_tab().map(|t| t.sort).unwrap_or_default();
                let vfs = Arc::clone(&self.vfs);
                Task::perform(
                    async move {
                        let Ok(snap) = vfs.list_dir(&path, 0, cancel).await else {
                            return (path, Vec::new());
                        };
                        let mut folders: Vec<FileEntry> = snap
                            .entries
                            .into_iter()
                            .filter(|e| e.kind.is_dir_like() && (show_hidden || !e.is_hidden))
                            .collect();
                        folders.sort_by(|a, b| sort.compare(a, b));
                        (path, folders)
                    },
                    |(path, folders)| Message::BreadcrumbMenuLoaded { path, folders },
                )
            }
            Message::CloseBreadcrumbMenu => {
                if let Some(cancel) = self.breadcrumb_menu_cancel.take() {
                    cancel.cancel();
                }
                self.breadcrumb_menu = None;
                Task::none()
            }
            Message::BreadcrumbMenuLoaded { path, folders } => {
                if self
                    .breadcrumb_menu
                    .as_ref()
                    .is_some_and(|(cur, _)| cur == &path)
                {
                    self.breadcrumb_menu = Some((path, folders));
                }
                Task::none()
            }
            Message::ResolveConflict(policy) => {
                let Some(ActiveDialog::Conflict {
                    mode,
                    sources,
                    dest_dir,
                    ..
                }) = self.active_dialog.take()
                else {
                    return Task::none();
                };
                if mode == CopyMode::Move {
                    self.clipboard = None;
                }
                self.dispatch_transfer(mode, sources, dest_dir, policy)
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
            Message::SystemIconReady {
                generation,
                key,
                bucket,
                rgba,
            } => {
                self.in_flight_system_icons.remove(&(key.clone(), bucket));
                if generation != self.system_icon_generation {
                    return Task::none();
                }
                self.system_icon_cache
                    .insert((key.clone(), bucket), rgba.clone());
                if let Some(thumb) = rgba {
                    if self.gpu_system_icons.len() >= fm_preview::DEFAULT_ICON_CACHE_CAPACITY {
                        self.gpu_system_icons.clear();
                    }
                    self.gpu_system_icons.insert(
                        (key, bucket),
                        image::Handle::from_rgba(thumb.width, thumb.height, thumb.pixels.clone()),
                    );
                }
                Task::none()
            }
            Message::OpenCreateDialog(is_dir) => {
                self.context_menu = None;
                self.active_dialog = Some(ActiveDialog::CreateItem {
                    is_dir,
                    name: String::new(),
                });
                operation::focus(dialog_input_id())
            }
            Message::OpenRenameDialog => {
                self.context_menu = None;
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
                self.context_menu = None;
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
                if let Some(ActiveDialog::CreateItem { is_dir: cur, .. }) = &mut self.active_dialog
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
                self.context_menu = None;
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
                self.context_menu = None;
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(ClipboardTransfer {
                        mode: CopyMode::Move,
                        paths,
                    });
                }
                Task::none()
            }
            Message::PasteClipboard => {
                self.context_menu = None;
                self.start_clipboard_paste()
            }
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
            } => match result {
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
            },
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
            Message::RemoveFavorite(path) => {
                self.context_menu = None;
                if let Some(pos) = self.config.favorites.iter().position(|p| p == &path) {
                    self.config.favorites.remove(pos);
                    return self.persist_config_task();
                }
                Task::none()
            }
            Message::OpenSidebarContextMenu { path, is_favorite } => {
                self.breadcrumb_menu = None;
                self.context_menu = Some(ContextMenuState {
                    kind: ContextMenuKind::SidebarPlace { path, is_favorite },
                    position: self.cursor_position,
                });
                Task::none()
            }
            Message::OpenPreferences => {
                self.context_menu = None;
                self.prefs_tab = PreferencesTab::General;
                self.prefs_open = true;
                Task::none()
            }
            Message::OpenAbout => {
                self.context_menu = None;
                self.prefs_tab = PreferencesTab::About;
                self.prefs_open = true;
                Task::none()
            }
            Message::SelectPreferencesTab(tab) => {
                self.prefs_tab = tab;
                Task::none()
            }
            Message::OpenPreferencesContextMenu => {
                self.breadcrumb_menu = None;
                self.context_menu = Some(ContextMenuState {
                    kind: ContextMenuKind::PreferencesMenu,
                    position: self.cursor_position,
                });
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
            Message::ToggleGridGroupFolders => {
                self.config.grid_group_folders = !self.config.grid_group_folders;
                self.persist_config_task()
            }
            Message::ToggleGridCenterItems => {
                self.config.grid_center_items = !self.config.grid_center_items;
                self.persist_config_task()
            }
            Message::SetGridCenterItems(center) => {
                if self.config.grid_center_items == center {
                    return Task::none();
                }
                self.config.grid_center_items = center;
                self.persist_config_task()
            }
            Message::ToggleSystemIcons => {
                self.config.icon_theme_mode = match self.config.icon_theme_mode {
                    IconThemeMode::Bundled => IconThemeMode::HostSystem,
                    IconThemeMode::HostSystem => IconThemeMode::Bundled,
                };
                let persist = self.persist_config_task();
                let queue = self.queue_visible_system_icons();
                Task::batch([persist, queue])
            }
            Message::ToggleDirsFirst => {
                self.config.dirs_first = !self.config.dirs_first;
                let dirs_first = self.config.dirs_first;
                for tab in &mut self.tabs {
                    tab.sort.dirs_first = dirs_first;
                    apply_tab_filter(tab);
                }
                let persist = self.persist_config_task();
                let thumbs = self.queue_visible_thumbnails();
                Task::batch([persist, thumbs])
            }
            Message::RequestEmptyTrash => {
                self.context_menu = None;
                self.active_dialog = Some(ActiveDialog::ConfirmEmptyTrash);
                Task::none()
            }
            Message::RestoreFromTrash => {
                self.context_menu = None;
                let paths = self.selected_paths();
                if paths.is_empty() {
                    return Task::none();
                }
                let vfs = Arc::clone(&self.vfs);
                let ok_msg = self.strings().trash_restored.to_owned();
                Task::perform(
                    async move { vfs.restore_from_trash(&paths).await.map(|_| ()) },
                    move |result| Message::TrashActionFinished {
                        notice: ok_msg.clone(),
                        result,
                    },
                )
            }
            Message::TrashActionFinished { notice, result } => {
                match result {
                    Ok(()) => {
                        self.status_notice = Some(notice);
                        return self.update(Message::RefreshActiveTab);
                    }
                    Err(err) => {
                        self.status_notice = Some(err.to_string());
                    }
                }
                Task::none()
            }
            Message::ToggleWindowDecorations => {
                self.config.window_decorations = match self.config.window_decorations {
                    WindowDecorationsMode::NativeWm => WindowDecorationsMode::IntegratedCsd,
                    WindowDecorationsMode::IntegratedCsd => WindowDecorationsMode::NativeWm,
                };
                let persist = self.persist_config_task();
                Task::batch([persist, window::toggle_decorations(self.main_window_id)])
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
            Message::OpenNewMenu => {
                self.breadcrumb_menu = None;
                let is_already_new = matches!(
                    self.context_menu.as_ref().map(|m| &m.kind),
                    Some(ContextMenuKind::NewMenu)
                );
                if is_already_new {
                    self.context_menu = None;
                } else {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::NewMenu,
                        position: self.cursor_position,
                    });
                }
                Task::none()
            }
            Message::OpenEntryContextMenu(index) => {
                self.breadcrumb_menu = None;
                let mut side_task = Task::none();
                let mut target_meta = None;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(entry) = tab.visible_entries.get(index)
                {
                    let already_selected = tab.selection.selected_ids.contains(&entry.id);
                    target_meta = Some((entry.path.clone(), entry.kind.is_dir_like()));
                    if !already_selected {
                        tab.selection.select_single(index, &tab.visible_entries);
                        let single = tab.selection.single_selected(&tab.visible_entries).cloned();
                        tab.selected_telemetry = single.as_ref().map(basic_file_telemetry);
                        side_task = self.dispatch_selection_side_effects(single.as_ref());
                    }
                }
                if let Some((path, is_dir)) = target_meta {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::Entry {
                            index,
                            path,
                            is_dir,
                        },
                        position: self.cursor_position,
                    });
                }
                side_task
            }
            Message::OpenCanvasContextMenu => {
                self.breadcrumb_menu = None;
                self.context_menu = Some(ContextMenuState {
                    kind: ContextMenuKind::Background,
                    position: self.cursor_position,
                });
                Task::none()
            }
            Message::CloseContextMenu => {
                self.context_menu = None;
                Task::none()
            }
            Message::CopyPath(path) => {
                self.context_menu = None;
                let text = path.to_string_lossy().into_owned();
                self.status_notice = Some(self.strings().path_copied.to_owned());
                clipboard::write(text)
            }
            Message::CopyText(text, notice) => {
                self.context_menu = None;
                self.status_notice = Some(notice);
                clipboard::write(text)
            }
            Message::OpenProperties(path) => {
                self.context_menu = None;
                if let Some((&existing_id, _)) = self
                    .properties_windows
                    .iter()
                    .find(|(_, props)| props.entry.path == path)
                {
                    if existing_id != self.main_window_id {
                        return window::gain_focus(existing_id);
                    }
                    return Task::none();
                }
                let entry = self.current_tab().and_then(|t| {
                    t.visible_entries
                        .iter()
                        .find(|e| e.path == path)
                        .or_else(|| t.raw_entries.iter().find(|e| e.path == path))
                        .cloned()
                });
                if let Some(entry) = entry {
                    return self.open_properties_window(entry);
                }
                Task::perform(fm_fs::stat_entry(path), Message::PropertiesEntryStatLoaded)
            }
            Message::PropertiesEntryStatLoaded(result) => match result {
                Ok(entry) => self.open_properties_window(entry),
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::PropertiesFolderSizeReady {
                window_id,
                path,
                size,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.entry.path == path
                {
                    props.recursive_size = size;
                    props.size_loading = false;
                }
                Task::none()
            }
            Message::PropertiesVolumeSpaceReady {
                window_id,
                path,
                space,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.entry.path == path
                {
                    props.volume_space = space;
                }
                Task::none()
            }
            Message::PropertiesTelemetryReady {
                window_id,
                path,
                mime,
                sha,
                git_status,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.entry.path == path
                {
                    let mut tel = props
                        .telemetry
                        .take()
                        .unwrap_or_else(|| basic_file_telemetry(&props.entry));
                    if mime.is_some() {
                        tel.mime_type = mime;
                    }
                    if let Some((prefix, full)) = sha {
                        tel.sha256_prefix = Some(prefix);
                        tel.sha256_full = Some(full);
                    }
                    if git_status.is_some() {
                        tel.git_status = git_status;
                    }
                    props.telemetry = Some(tel);
                }
                Task::none()
            }
            Message::PropertiesExtraInfoReady {
                window_id,
                path,
                extra,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.entry.path == path
                {
                    if props.readonly_draft == props.initial_readonly {
                        props.initial_readonly = extra.is_readonly;
                        props.readonly_draft = extra.is_readonly;
                    }
                    props.extra = Some(extra);
                }
                Task::none()
            }
            Message::PropertiesSelectTab(window_id, tab) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id) {
                    props.active_tab = tab;
                }
                Task::none()
            }
            Message::PropertiesNameChanged(window_id, val) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && !props.is_archive()
                    && !props.applying
                {
                    props.draft_name = val;
                    if props.posix_mode_draft.is_some() {
                        props.hidden_draft = props.draft_name.trim_start().starts_with('.');
                    }
                }
                Task::none()
            }
            Message::PropertiesToggleReadonly(window_id) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && !props.is_archive()
                    && !props.applying
                {
                    props.readonly_draft = !props.readonly_draft;
                    if let Some(mode) = &mut props.posix_mode_draft {
                        if props.readonly_draft {
                            *mode &= !0o222;
                        } else {
                            *mode |= 0o200;
                        }
                    }
                }
                Task::none()
            }
            Message::PropertiesToggleHidden(window_id) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && !props.is_archive()
                    && !props.applying
                {
                    props.hidden_draft = !props.hidden_draft;
                    if props.posix_mode_draft.is_some() {
                        if props.hidden_draft {
                            if !props.draft_name.starts_with('.') {
                                props.draft_name.insert(0, '.');
                            }
                        } else {
                            let stripped = props.draft_name.trim_start_matches('.').to_owned();
                            if !stripped.is_empty() {
                                props.draft_name = stripped;
                            } else {
                                props.hidden_draft = true;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::PropertiesTogglePosixBit(window_id, bit) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && !props.is_archive()
                    && !props.applying
                    && let Some(mode) = &mut props.posix_mode_draft
                {
                    *mode ^= bit;
                    props.readonly_draft = (*mode & 0o222) == 0;
                }
                Task::none()
            }
            Message::PropertiesApply {
                window_id,
                close_after,
            } => {
                let Some(props) = self.properties_windows.get_mut(&window_id) else {
                    return Task::none();
                };
                if props.applying {
                    return Task::none();
                }
                if !props.is_dirty() {
                    if close_after {
                        return self.update(Message::ClosePropertiesWindow(window_id));
                    }
                    return Task::none();
                }
                let trimmed = props.draft_name.trim().to_owned();
                if trimmed.is_empty() || trimmed.contains(['/', '\\']) {
                    self.status_notice = Some(self.strings().invalid_name.to_owned());
                    return Task::none();
                }
                let original_path = props.entry.path.clone();
                let original_name = props.entry.name.to_string();
                let readonly = (props.readonly_draft != props.initial_readonly)
                    .then_some(props.readonly_draft);
                let hidden =
                    (props.hidden_draft != props.initial_hidden).then_some(props.hidden_draft);
                let posix_mode = (props.posix_mode_draft != props.initial_posix_mode)
                    .then_some(props.posix_mode_draft)
                    .flatten();
                props.applying = true;

                let vfs = Arc::clone(&self.vfs);
                Task::perform(
                    async move {
                        let target_path = if trimmed != original_name {
                            let Some(parent) = original_path.parent() else {
                                return (
                                    original_path.clone(),
                                    Err(FsError::Io {
                                        path: original_path,
                                        message: "caminho pai ausente".to_owned(),
                                    }),
                                );
                            };
                            let dst = parent.join(&trimmed);
                            if let Err(err) = vfs.rename(&original_path, &dst).await {
                                return (original_path, Err(err));
                            }
                            dst
                        } else {
                            original_path.clone()
                        };

                        if readonly.is_some() || hidden.is_some() || posix_mode.is_some() {
                            let patch = fm_fs::PropertyPatch {
                                readonly,
                                hidden,
                                posix_mode,
                            };
                            if let Err(err) =
                                fm_fs::apply_entry_properties(target_path.clone(), patch).await
                            {
                                return (original_path, Err(err));
                            }
                        }

                        match fm_fs::stat_entry(target_path.clone()).await {
                            Ok(updated_entry) => {
                                let updated_extra = fm_fs::read_entry_extra_info(target_path).await;
                                (original_path, Ok((updated_entry, updated_extra)))
                            }
                            Err(err) => (original_path, Err(err)),
                        }
                    },
                    move |(original_path, result)| Message::PropertiesApplied {
                        window_id,
                        original_path,
                        close_after,
                        result,
                    },
                )
            }
            Message::PropertiesApplied {
                window_id,
                original_path,
                close_after,
                result,
            } => match result {
                Ok((updated_entry, updated_extra)) => {
                    let select_path = updated_entry.path.clone();
                    let mut favorite_config_task = Task::none();
                    if select_path != original_path
                        && let Some(fav) = self
                            .config
                            .favorites
                            .iter_mut()
                            .find(|p| **p == original_path)
                    {
                        *fav = select_path.clone();
                        favorite_config_task = self.persist_config_task();
                    }
                    self.status_notice = Some(self.strings().prop_applied_notice.to_owned());
                    let close_task = if close_after {
                        if let Some(mut props) = self.properties_windows.remove(&window_id)
                            && let Some(cancel) = props.cancel.take()
                        {
                            cancel.cancel();
                        }
                        window::close(window_id)
                    } else {
                        if let Some(props) = self.properties_windows.get_mut(&window_id)
                            && props.entry.path == original_path
                        {
                            let (pm, ro) = match updated_entry.permissions {
                                PermissionsMeta::PosixMode(mode) => {
                                    let perms = mode & 0o777;
                                    (Some(perms), updated_extra.is_readonly)
                                }
                                PermissionsMeta::WinAttributes(_) => {
                                    (None, updated_extra.is_readonly)
                                }
                            };
                            props.draft_name = updated_entry.name.to_string();
                            props.hidden_draft = updated_entry.is_hidden;
                            props.initial_hidden = updated_entry.is_hidden;
                            props.readonly_draft = ro;
                            props.initial_readonly = ro;
                            props.posix_mode_draft = pm;
                            props.initial_posix_mode = pm;
                            props.entry = updated_entry;
                            props.extra = Some(updated_extra);
                            props.applying = false;
                        }
                        Task::none()
                    };
                    self.pending_select_path = Some(select_path);
                    let refresh_task = self.update(Message::RefreshActiveTab);
                    Task::batch([close_task, refresh_task, favorite_config_task])
                }
                Err(err) => {
                    if let Some(props) = self.properties_windows.get_mut(&window_id) {
                        props.applying = false;
                    }
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::PropertiesOpenDefault(path) => {
                let vfs = Arc::clone(&self.vfs);
                Task::perform(
                    async move { vfs.open_default(&path).await },
                    Message::OpenDefaultFinished,
                )
            }
            Message::DragPropertiesWindow(window_id) => {
                if window_id == self.main_window_id {
                    Task::none()
                } else {
                    window::drag(window_id)
                }
            }
            Message::ClosePropertiesWindow(window_id) => {
                if let Some(mut props) = self.properties_windows.remove(&window_id)
                    && let Some(cancel) = props.cancel.take()
                {
                    cancel.cancel();
                }
                if window_id == self.main_window_id {
                    Task::none()
                } else {
                    window::close(window_id)
                }
            }
            Message::OpenFilterMenu => {
                self.breadcrumb_menu = None;
                let is_already_filter = matches!(
                    self.context_menu.as_ref().map(|m| &m.kind),
                    Some(ContextMenuKind::FilterMenu)
                );
                if is_already_filter {
                    self.context_menu = None;
                } else {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::FilterMenu,
                        position: self.cursor_position,
                    });
                }
                Task::none()
            }
            Message::SetExtensionFilter(ext) => {
                self.context_menu = None;
                if let Some(tab) = self.current_tab_mut() {
                    tab.extension_filter = ext;
                    apply_tab_filter(tab);
                }
                self.canvas_scroll_y = 0.0;
                self.queue_visible_thumbnails()
            }
            Message::StartColumnResize(idx) => {
                if idx < 4 {
                    self.resizing_col = Some((idx, self.cursor_position.x, self.table_col_weights));
                }
                Task::none()
            }
            Message::StartGridResize => {
                self.resizing_grid = Some((
                    self.cursor_position.x,
                    views::virtualized::clamp_grid_card_width(self.config.grid_card_width),
                ));
                Task::none()
            }
            Message::StartColumnsPanelResize => {
                self.resizing_columns_panel =
                    Some((self.cursor_position.x, self.columns_panel_width));
                Task::none()
            }
            Message::StartSidebarResize => {
                let current_w = views::sidebar::effective_sidebar_width(self, self.strings());
                let start_x = if self.cursor_position.x > 1.0 {
                    self.cursor_position.x
                } else {
                    current_w
                };
                self.resizing_sidebar = Some((start_x, current_w));
                Task::none()
            }
            Message::SidebarDividerHovered(hovered) => {
                self.sidebar_divider_hovered = hovered;
                Task::none()
            }
            Message::ResetSidebarWidth => {
                self.resizing_sidebar = None;
                self.config.sidebar_width = None;
                self.persist_config_task()
            }
            Message::EndColumnResize(window_id) => {
                if window_id != self.main_window_id {
                    return Task::none();
                }
                self.resizing_col = None;
                self.resizing_columns_panel = None;
                let mut should_persist = self.resizing_grid.take().is_some();
                if self.resizing_sidebar.take().is_some() {
                    should_persist = true;
                }
                if should_persist {
                    return self.persist_config_task();
                }
                Task::none()
            }
            Message::StepGridCardWidth(delta) => {
                self.grid_zoom_dirty = false;
                let next =
                    views::virtualized::clamp_grid_card_width(self.config.grid_card_width + delta);
                self.config.grid_card_width = next;
                let persist = self.persist_config_task();
                let thumbs = self.queue_visible_thumbnails();
                Task::batch([persist, thumbs])
            }
            Message::ResetGridCardWidth => {
                self.grid_zoom_dirty = false;
                self.config.grid_card_width = views::virtualized::GRID_CARD_WIDTH_DEFAULT;
                let persist = self.persist_config_task();
                let thumbs = self.queue_visible_thumbnails();
                Task::batch([persist, thumbs])
            }
            Message::CtrlWheelZoom(delta) => {
                let px_delta = wheel_zoom_delta(delta);
                if px_delta.abs() <= f32::EPSILON {
                    return Task::none();
                }
                let next = views::virtualized::clamp_grid_card_width(
                    self.config.grid_card_width + px_delta,
                );
                if (next - self.config.grid_card_width).abs() <= f32::EPSILON {
                    return Task::none();
                }
                self.config.grid_card_width = next;
                self.grid_zoom_dirty = true;
                self.zoom_save_seq = self.zoom_save_seq.wrapping_add(1);
                let seq = self.zoom_save_seq;
                let thumbs = self.queue_visible_thumbnails();
                let save_debounce = Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
                        seq
                    },
                    Message::FlushZoomConfig,
                );
                Task::batch([thumbs, save_debounce])
            }
            Message::FlushZoomConfig(seq) => {
                if seq == self.zoom_save_seq && self.grid_zoom_dirty {
                    self.grid_zoom_dirty = false;
                    return self.persist_config_task();
                }
                Task::none()
            }
            Message::CursorMoved {
                window_id,
                position: pos,
            } => {
                if window_id != self.main_window_id {
                    return Task::none();
                }
                self.cursor_position = pos;
                if let Some((idx, start_x, initial_weights)) = self.resizing_col
                    && idx < 4
                {
                    let content_width = (self.window_size.width - 280.0).max(480.0);
                    let delta_weight = ((pos.x - start_x) / content_width) * 100.0;
                    let pair_sum = initial_weights[idx] + initial_weights[idx + 1];
                    let min_w = 6.0_f32;
                    let max_w = (pair_sum - min_w).max(min_w);
                    let new_left = (initial_weights[idx] + delta_weight).clamp(min_w, max_w);
                    self.table_col_weights[idx] = new_left;
                    self.table_col_weights[idx + 1] = pair_sum - new_left;
                }
                if let Some((start_x, initial_w)) = self.resizing_grid {
                    self.config.grid_card_width =
                        views::virtualized::clamp_grid_card_width(initial_w + (pos.x - start_x));
                }
                if let Some((start_x, initial_w)) = self.resizing_columns_panel {
                    self.columns_panel_width = (initial_w + (pos.x - start_x)).clamp(200.0, 480.0);
                }
                if let Some((start_x, initial_w)) = self.resizing_sidebar {
                    self.config.sidebar_width = Some(compute_dragged_sidebar_width(
                        initial_w,
                        start_x,
                        pos.x,
                        self.window_size.width,
                    ));
                }
                Task::none()
            }
            Message::MetaCacheReady(cache) => {
                self.metadata_cache = cache;
                Task::none()
            }
            Message::ColumnAncestorsLoaded {
                request_id,
                ancestors,
            } => {
                if request_id == self.column_ancestors_request_id {
                    self.column_ancestors = ancestors;
                }
                Task::none()
            }
            Message::WindowResized { window_id, size } => {
                if window_id == self.main_window_id {
                    self.window_size = size;
                    self.canvas_viewport_width = None;
                }
                Task::none()
            }
            Message::MonitorSizeReady(Some(mon)) => {
                let fitted = fit_window_size_to_monitor(mon);
                self.window_size = fitted;
                self.canvas_viewport_width = None;
                let x = ((mon.width - fitted.width) * 0.5).max(8.0);
                let y = ((mon.height - 48.0 - fitted.height) * 0.5).max(16.0);
                Task::batch([
                    window::resize(self.main_window_id, fitted),
                    window::move_to(self.main_window_id, iced::Point::new(x, y)),
                ])
            }
            Message::MonitorSizeReady(None) => Task::none(),
            Message::WindowClosed(window_id) => {
                if window_id == self.main_window_id {
                    for (_, mut props) in self.properties_windows.drain() {
                        if let Some(cancel) = props.cancel.take() {
                            cancel.cancel();
                        }
                    }
                    return iced::exit();
                }
                if let Some(mut props) = self.properties_windows.remove(&window_id)
                    && let Some(cancel) = props.cancel.take()
                {
                    cancel.cancel();
                }
                Task::none()
            }
            Message::FileDropped { window_id, path } => {
                if window_id != self.main_window_id {
                    return Task::none();
                }
                if !self.pending_dropped_paths.contains(&path) {
                    self.pending_dropped_paths.push(path);
                }
                self.drop_batch_id += 1;
                let batch_id = self.drop_batch_id;
                Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_millis(45)).await;
                        batch_id
                    },
                    Message::FlushDroppedFiles,
                )
            }
            Message::FlushDroppedFiles(batch_id) => {
                if batch_id == self.drop_batch_id {
                    return self.flush_dropped_files();
                }
                Task::none()
            }
            Message::WindowMinimize => window::minimize(self.main_window_id, true),
            Message::WindowToggleMaximize => window::toggle_maximize(self.main_window_id),
            Message::WindowClose => window::close(self.main_window_id),
            Message::ModifiersChanged(modifiers) => {
                self.current_modifiers = modifiers;
                if !modifiers.control() && self.grid_zoom_dirty {
                    self.grid_zoom_dirty = false;
                    return self.persist_config_task();
                }
                Task::none()
            }
            Message::WindowUnfocused(window_id) => {
                if window_id == self.main_window_id {
                    self.current_modifiers = keyboard::Modifiers::empty();
                    if self.grid_zoom_dirty {
                        self.grid_zoom_dirty = false;
                        return self.persist_config_task();
                    }
                }
                Task::none()
            }
            Message::GlobalKeyPressed {
                window_id,
                key,
                modifiers,
            } => {
                if window_id == self.main_window_id {
                    self.current_modifiers = modifiers;
                }
                self.handle_global_key(window_id, key, modifiers)
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let events_sub = event::listen_with(|event, status, window_id| match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                Some(Message::ModifiersChanged(modifiers))
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let effective = apply_modifier_key_press(&key, modifiers);
                let is_modifier_key = effective != modifiers;
                let is_shortcut = effective.control()
                    || effective.alt()
                    || matches!(
                        key,
                        keyboard::Key::Named(Named::Escape | Named::F5 | Named::Tab)
                    );
                if is_modifier_key {
                    Some(Message::ModifiersChanged(effective))
                } else if status == event::Status::Ignored || is_shortcut {
                    Some(Message::GlobalKeyPressed {
                        window_id,
                        key,
                        modifiers: effective,
                    })
                } else {
                    None
                }
            }
            Event::Keyboard(keyboard::Event::KeyReleased { key, modifiers, .. }) => {
                let effective = apply_modifier_key_release(&key, modifiers);
                if effective != modifiers
                    || matches!(
                        key,
                        keyboard::Key::Named(
                            Named::Control | Named::Shift | Named::Alt | Named::Super
                        )
                    )
                {
                    Some(Message::ModifiersChanged(effective))
                } else {
                    None
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Message::CursorMoved {
                window_id,
                position,
            }),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::EndColumnResize(window_id))
            }
            Event::Window(window::Event::Resized(size)) => {
                Some(Message::WindowResized { window_id, size })
            }
            Event::Window(window::Event::Unfocused) => Some(Message::WindowUnfocused(window_id)),
            Event::Window(window::Event::FileDropped(path)) => {
                Some(Message::FileDropped { window_id, path })
            }
            _ => None,
        });

        let mut subs = vec![
            events_sub,
            window::close_events().map(Message::WindowClosed),
        ];
        if !self.config.theme_customized {
            subs.push(system::theme_changes().map(Message::SystemThemeDetected));
        }
        if let Some(active_path) = self.current_tab().map(|t| t.current_path.clone()) {
            subs.push(Subscription::run_with(active_path, watch_directory_stream));
        }
        Subscription::batch(subs)
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        let palette = self.palette();
        let strings = self.strings();
        if window_id != self.main_window_id
            && let Some(props) = self.properties_windows.get(&window_id)
        {
            return views::modals::properties_window_view(window_id, props, palette, strings);
        }
        let tab = self.current_tab();

        let sidebar_divider: Element<'_, Message> = if self.config.sidebar_collapsed {
            container(iced::widget::Space::new().width(1).height(Length::Fill))
                .style(fm_theme::styles::divider_line(palette))
                .into()
        } else {
            let active = self.resizing_sidebar.is_some();
            let highlighted = active || self.sidebar_divider_hovered;
            let line_color = if highlighted {
                palette.accent_primary
            } else {
                palette.border
            };
            let line_width = if highlighted { 2.0 } else { 1.0 };
            let full_line = container(
                container(
                    iced::widget::Space::new()
                        .width(line_width)
                        .height(Length::Fill),
                )
                .style(move |_| {
                    fm_theme::styles::panel_surface(line_color, iced::Color::TRANSPARENT, 0.0)
                }),
            )
            .width(Length::Fixed(10.0))
            .height(Length::Fill)
            .align_x(iced::Alignment::Center);

            let content: Element<'_, Message> = if highlighted {
                let grip = container(
                    container(iced::widget::Space::new().width(4.0).height(36.0)).style(
                        move |_| {
                            fm_theme::styles::panel_surface(
                                palette.accent_primary,
                                iced::Color::TRANSPARENT,
                                9999.0,
                            )
                        },
                    ),
                )
                .width(Length::Fixed(10.0))
                .height(Length::Fill)
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center);
                stack![full_line, grip].into()
            } else {
                full_line.into()
            };

            iced::widget::mouse_area(content)
                .interaction(mouse::Interaction::ResizingHorizontally)
                .on_enter(Message::SidebarDividerHovered(true))
                .on_exit(Message::SidebarDividerHovered(false))
                .on_press(Message::StartSidebarResize)
                .on_right_press(Message::ResetSidebarWidth)
                .into()
        };

        let body_row = row![
            views::sidebar::view(self, palette, strings),
            sidebar_divider,
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

        // Mantém `Stack` sempre como raiz para que abrir/fechar menus e modais não descarte o estado da árvore (ex.: scroll do canvas).
        let overlay: Option<Element<'_, Message>> = if let Some(dialog) = &self.active_dialog {
            Some(views::modals::dialog_modal(dialog, palette, strings))
        } else if let Some(props) = self.properties_windows.get(&self.main_window_id) {
            Some(views::modals::properties_modal_overlay(
                self.main_window_id,
                props,
                palette,
                strings,
            ))
        } else if self.prefs_open {
            Some(views::modals::preferences_modal(self, palette, strings))
        } else if self.quicklook_open {
            Some(views::inspector::quicklook_modal(
                self, tab, palette, strings,
            ))
        } else if self.deep_search_open {
            Some(views::inspector::deep_search_modal(self, palette, strings))
        } else if let Some((root, folders)) = &self.breadcrumb_menu {
            Some(views::modals::breadcrumb_menu_overlay(
                root, folders, palette, strings,
            ))
        } else {
            self.context_menu
                .as_ref()
                .map(|menu| views::modals::context_menu_overlay(self, menu, palette, strings))
        };

        if let Some(layer) = overlay {
            stack![main_shell, layer].into()
        } else {
            stack![main_shell].into()
        }
    }

    fn alloc_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    /// Opens (or focuses) a floating centered Properties window for `entry`.
    fn open_properties_window(&mut self, entry: FileEntry) -> Task<Message> {
        if let Some((&existing_id, _)) = self
            .properties_windows
            .iter()
            .find(|(_, props)| props.entry.path == entry.path)
        {
            return window::gain_focus(existing_id);
        }
        let existing_telemetry = self.current_tab().and_then(|t| {
            let selected = t.selection.single_selected(&t.visible_entries)?;
            (selected.path == entry.path)
                .then_some(t.selected_telemetry.as_ref())
                .flatten()
                .cloned()
        });
        let is_archive = fm_fs::archive::split_archive_path(&entry.path).is_some();
        let needs_file_telemetry = !entry.kind.is_dir_like()
            && !is_archive
            && existing_telemetry
                .as_ref()
                .is_none_or(|tel| tel.mime_type.is_none() || tel.sha256_full.is_none());
        let telemetry = Some(existing_telemetry.unwrap_or_else(|| basic_file_telemetry(&entry)));
        let size_loading = entry.kind.is_dir_like() && !is_archive;

        let initial_volume_space = self.current_tab().and_then(|t| {
            (t.current_path == entry.path)
                .then_some(t.telemetry.volume_space.clone())
                .flatten()
        });

        #[cfg(target_os = "windows")]
        let (win_id, open_task) = (self.main_window_id, Task::<window::Id>::none());

        #[cfg(not(target_os = "windows"))]
        let (win_id, open_task) = window::open(window::Settings {
            size: Size::new(720.0, 520.0),
            min_size: Some(Size::new(640.0, 460.0)),
            position: window::Position::Centered,
            decorations: true,
            resizable: true,
            ..window::Settings::default()
        });

        let win_cancel = CancellationToken::new();
        let mut tasks = vec![open_task.discard()];

        if !is_archive && initial_volume_space.is_none() {
            let vfs = Arc::clone(&self.vfs);
            let vpath = entry.path.clone();
            tasks.push(Task::perform(
                async move {
                    let space = vfs.volume_space(&vpath).await.ok().flatten();
                    (vpath, space)
                },
                move |(path, space)| Message::PropertiesVolumeSpaceReady {
                    window_id: win_id,
                    path,
                    space,
                },
            ));
        }

        if size_loading {
            let cancel = win_cancel.clone();
            let size_dir = entry.path.clone();
            let meta_cache = self.metadata_cache.clone();
            tasks.push(Task::perform(
                async move {
                    let size = fm_telemetry::compute_folder_size_cached(
                        size_dir.clone(),
                        meta_cache,
                        cancel,
                    )
                    .await;
                    (size_dir, size)
                },
                move |(path, size)| Message::PropertiesFolderSizeReady {
                    window_id: win_id,
                    path,
                    size,
                },
            ));
        } else if needs_file_telemetry {
            let tel_path = entry.path.clone();
            let cancel_mime = win_cancel.clone();
            let cancel_sha = win_cancel.clone();
            let cancel_git = win_cancel.clone();
            let size_bytes = entry.size_bytes;
            let modified_nanos = entry_mtime_nanos(entry.modified_at);
            let meta_cache = self.metadata_cache.clone();
            tasks.push(Task::perform(
                async move {
                    let cached = if let Some(ref cache) = meta_cache {
                        let p = tel_path.clone();
                        let c = Arc::clone(cache);
                        tokio::task::spawn_blocking(move || {
                            c.get_file(&p, size_bytes, modified_nanos)
                        })
                        .await
                        .ok()
                        .flatten()
                    } else {
                        None
                    };
                    let mime = match cached.as_ref().and_then(|c| c.mime.clone()) {
                        Some(m) => Some(m),
                        None => fm_telemetry::detect_mime_type(&tel_path, cancel_mime).await,
                    };
                    let sha = match cached
                        .as_ref()
                        .and_then(|c| c.sha_prefix.clone().zip(c.sha_full.clone()))
                    {
                        Some(pair) => Some(pair),
                        None => fm_telemetry::compute_sha256(&tel_path, cancel_sha).await,
                    };
                    let git_status =
                        fm_telemetry::detect_file_git_status(&tel_path, cancel_git).await;
                    if let Some(cache) = meta_cache
                        && (mime.is_some() || sha.is_some())
                    {
                        let p = tel_path.clone();
                        let val = CachedFileTelemetry {
                            mime: mime.clone(),
                            sha_prefix: sha.as_ref().map(|(pre, _)| pre.clone()),
                            sha_full: sha.as_ref().map(|(_, full)| full.clone()),
                        };
                        drop(tokio::task::spawn_blocking(move || {
                            cache.put_file(&p, size_bytes, modified_nanos, &val);
                        }));
                    }
                    (tel_path, mime, sha, git_status)
                },
                move |(path, mime, sha, git_status)| Message::PropertiesTelemetryReady {
                    window_id: win_id,
                    path,
                    mime,
                    sha,
                    git_status,
                },
            ));
        }

        if !is_archive {
            let extra_path = entry.path.clone();
            tasks.push(Task::perform(
                async move {
                    let extra = fm_fs::read_entry_extra_info(extra_path.clone()).await;
                    (extra_path, extra)
                },
                move |(path, extra)| Message::PropertiesExtraInfoReady {
                    window_id: win_id,
                    path,
                    extra,
                },
            ));
        }

        let (initial_posix_mode, initial_readonly) = match entry.permissions {
            PermissionsMeta::PosixMode(mode) => {
                let perms = mode & 0o777;
                (Some(perms), (perms & 0o222) == 0)
            }
            PermissionsMeta::WinAttributes(attrs) => (None, (attrs & 0x1) != 0),
        };
        let draft_name = entry.name.to_string();
        let initial_hidden = entry.is_hidden;

        self.properties_windows.insert(
            win_id,
            PropertiesWindowState {
                entry,
                telemetry,
                recursive_size: None,
                volume_space: initial_volume_space,
                size_loading,
                extra: None,
                active_tab: PropertiesTab::General,
                draft_name,
                readonly_draft: initial_readonly,
                initial_readonly,
                hidden_draft: initial_hidden,
                initial_hidden,
                posix_mode_draft: initial_posix_mode,
                initial_posix_mode,
                applying: false,
                cancel: Some(win_cancel),
            },
        );
        Task::batch(tasks)
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
        self.context_menu = None;
        self.quicklook_image = None;
        self.quicklook_editor = text_editor::Content::new();
        self.quicklook_extension.clear();
        if let Some(cancel) = self.breadcrumb_menu_cancel.take() {
            cancel.cancel();
        }
        self.breadcrumb_menu = None;
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
                    self.status_notice = Some(self.strings().invalid_name.to_owned());
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
                    self.status_notice = Some(self.strings().invalid_name.to_owned());
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
                Task::perform(async move { vfs.rename(&src, &dst).await }, move |result| {
                    Message::FileOpFinished {
                        result,
                        select_after: Some(select_target),
                    }
                })
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
            ActiveDialog::ConfirmEmptyTrash => {
                let notice = self.strings().trash_emptied.to_owned();
                Task::perform(async move { vfs.empty_trash().await }, move |result| {
                    Message::TrashActionFinished {
                        notice: notice.clone(),
                        result,
                    }
                })
            }
            ActiveDialog::Conflict { .. } => Task::none(),
        }
    }

    fn start_clipboard_paste(&mut self) -> Task<Message> {
        let Some(transfer) = self.clipboard.clone() else {
            return Task::none();
        };
        if transfer.mode == CopyMode::Move {
            // Limpeza definitiva ocorre quando transferência for despachada sem diálogo de conflito.
            let task = self.start_transfer_with_conflict_check(transfer.mode, transfer.paths);
            if !matches!(self.active_dialog, Some(ActiveDialog::Conflict { .. })) {
                self.clipboard = None;
            }
            return task;
        }
        self.start_transfer_with_conflict_check(transfer.mode, transfer.paths)
    }

    fn flush_dropped_files(&mut self) -> Task<Message> {
        if self.pending_dropped_paths.is_empty() {
            return Task::none();
        }
        let sources = std::mem::take(&mut self.pending_dropped_paths);
        self.start_transfer_with_conflict_check(CopyMode::Copy, sources)
    }

    fn start_transfer_with_conflict_check(
        &mut self,
        mode: CopyMode,
        sources: Vec<PathBuf>,
    ) -> Task<Message> {
        let Some(tab) = self.current_tab() else {
            return Task::none();
        };
        if sources.is_empty() {
            return Task::none();
        }
        let dest_dir = tab.current_path.clone();

        let conflicts = sources
            .iter()
            .filter(|src| {
                src.parent() != Some(&dest_dir)
                    && src.file_name().is_some_and(|n| {
                        let target_name = n.to_string_lossy();
                        tab.raw_entries
                            .iter()
                            .any(|e| e.name.as_str() == target_name.as_ref())
                    })
            })
            .count();

        if conflicts > 0 {
            self.active_dialog = Some(ActiveDialog::Conflict {
                mode,
                sources,
                dest_dir,
                conflicts,
            });
            return Task::none();
        }

        self.dispatch_transfer(mode, sources, dest_dir, ConflictPolicy::RenameAuto)
    }

    fn dispatch_transfer(
        &mut self,
        mode: CopyMode,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        policy: ConflictPolicy,
    ) -> Task<Message> {
        if let Some(prev) = self.transfer_cancel.take() {
            prev.cancel();
        }
        let job_id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancellationToken::new();
        self.transfer_cancel = Some(cancel.clone());

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
                let _ =
                    fm_fs::run_transfer_job(job_id, mode, sources, dest_dir, policy, cancel, tx)
                        .await;
            });
            while let Some(progress) = rx.recv().await {
                if output
                    .send(Message::TransferProgress(progress))
                    .await
                    .is_err()
                {
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

    fn maybe_load_column_ancestors(&mut self) -> Task<Message> {
        if let Some(prev) = self.column_ancestors_cancel.take() {
            prev.cancel();
        }
        let Some(tab) = self.current_tab() else {
            self.column_ancestors.clear();
            return Task::none();
        };
        if tab.view_mode != ViewMode::Columns {
            return Task::none();
        }
        let show_hidden = tab.show_hidden;
        let sort = tab.sort;
        let mut chain: Vec<PathBuf> = tab
            .current_path
            .ancestors()
            .skip(1)
            .take(3)
            .map(PathBuf::from)
            .collect();
        chain.reverse();
        if chain.is_empty() {
            self.column_ancestors.clear();
            return Task::none();
        }

        self.column_ancestors_request_id += 1;
        let req_id = self.column_ancestors_request_id;
        let cancel = CancellationToken::new();
        self.column_ancestors_cancel = Some(cancel.clone());
        let vfs = Arc::clone(&self.vfs);

        Task::perform(
            async move {
                let mut ancestors = Vec::with_capacity(chain.len());
                for dir in chain {
                    if cancel.is_cancelled() {
                        break;
                    }
                    if let Ok(snap) = vfs.list_dir(&dir, req_id, cancel.clone()).await {
                        let mut entries: Vec<FileEntry> = snap
                            .entries
                            .into_iter()
                            .filter(|e| show_hidden || !e.is_hidden)
                            .collect();
                        entries.sort_by(|a, b| sort.compare(a, b));
                        ancestors.push((dir, entries));
                    }
                }
                (req_id, ancestors)
            },
            |(request_id, ancestors)| Message::ColumnAncestorsLoaded {
                request_id,
                ancestors,
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
        if entry.kind.is_dir_like() || fm_fs::archive::split_archive_path(&entry.path).is_some() {
            return Task::none();
        }
        self.selection_telemetry_request_id += 1;
        let req_id = self.selection_telemetry_request_id;
        let cancel = CancellationToken::new();
        self.selection_telemetry_cancel = Some(cancel.clone());

        let path_mime = entry.path.clone();
        let path_sha = entry.path.clone();
        let path_git = entry.path.clone();
        let size_bytes = entry.size_bytes;
        let modified_nanos = entry_mtime_nanos(entry.modified_at);
        let meta_cache_mime = self.metadata_cache.clone();
        let meta_cache_sha = self.metadata_cache.clone();
        let cancel_mime = cancel.clone();
        let cancel_git = cancel.clone();

        let mime_task = Task::perform(
            async move {
                if let Some(cache) = meta_cache_mime {
                    let p = path_mime.clone();
                    if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                        cache.get_file(&p, size_bytes, modified_nanos)
                    })
                    .await
                        && cached.mime.is_some()
                    {
                        return (req_id, path_mime, cached.mime);
                    }
                }
                let mime = fm_telemetry::detect_mime_type(&path_mime, cancel_mime).await;
                (req_id, path_mime, mime)
            },
            |(request_id, path, mime)| Message::MimeDetected {
                request_id,
                path,
                mime,
            },
        );

        let git_task = Task::perform(
            async move {
                let status = fm_telemetry::detect_file_git_status(&path_git, cancel_git).await;
                (req_id, path_git, status)
            },
            |(request_id, path, status)| Message::GitStatusDetected {
                request_id,
                path,
                status,
            },
        );

        let sha_task = Task::perform(
            async move {
                if let Some(cache) = meta_cache_sha {
                    let p = path_sha.clone();
                    if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                        cache.get_file(&p, size_bytes, modified_nanos)
                    })
                    .await
                        && let (Some(prefix), Some(full)) = (cached.sha_prefix, cached.sha_full)
                    {
                        return (req_id, path_sha, Some((prefix, full)));
                    }
                }
                let sha = fm_telemetry::compute_sha256(&path_sha, cancel).await;
                (req_id, path_sha, sha)
            },
            |(request_id, path, sha)| Message::Sha256Computed {
                request_id,
                path,
                sha,
            },
        );

        Task::batch([mime_task, git_task, sha_task])
    }

    fn persist_file_telemetry(&self, entry: &FileEntry, tel: &SelectedFileTelemetry) {
        let Some(cache) = self.metadata_cache.clone() else {
            return;
        };
        let path = entry.path.clone();
        let size_bytes = entry.size_bytes;
        let mod_nanos = entry_mtime_nanos(entry.modified_at);
        let val = CachedFileTelemetry {
            mime: tel.mime_type.clone(),
            sha_prefix: tel.sha256_prefix.clone(),
            sha_full: tel.sha256_full.clone(),
        };
        drop(tokio::task::spawn_blocking(move || {
            cache.put_file(&path, size_bytes, mod_nanos, &val);
        }));
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
        let is_in_archive = fm_fs::archive::split_archive_path(&path).is_some();

        if fm_preview::is_image_extension(entry.extension.as_deref()) {
            let max_px = if self.quicklook_open { 560 } else { 160 };
            if is_in_archive {
                let vfs = Arc::clone(&self.vfs);
                let is_svg = entry
                    .extension
                    .as_deref()
                    .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
                return Task::perform(
                    async move {
                        let rgba =
                            match vfs.read_bytes(&path, 8 * 1024 * 1024, cancel.clone()).await {
                                Ok(bytes) => fm_preview::decode_thumbnail_from_bytes(
                                    &bytes, is_svg, max_px, &cancel,
                                ),
                                Err(_) => None,
                            };
                        (req_id, path, rgba)
                    },
                    |(request_id, path, rgba)| Message::QuickLookImageLoaded {
                        request_id,
                        path,
                        rgba,
                    },
                );
            }
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

        if is_in_archive {
            let vfs = Arc::clone(&self.vfs);
            return Task::perform(
                async move {
                    let snippet = match vfs.read_bytes(&path, 512 * 1024, cancel).await {
                        Ok(bytes) if !bytes.contains(&0) => {
                            Some(String::from_utf8_lossy(&bytes).into_owned())
                        }
                        _ => None,
                    };
                    (req_id, path, snippet)
                },
                |(request_id, path, snippet)| Message::TextPreviewLoaded {
                    request_id,
                    path,
                    snippet,
                },
            );
        }

        Task::perform(
            async move {
                let snippet = fm_preview::read_text_preview(&path, 512 * 1024, cancel).await;
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
            let disk_cache = self.disk_thumbnail_cache.clone();
            tasks.push(Task::perform(
                async move {
                    let key_for_disk = key.clone();
                    let rgba = fm_preview::generate_or_load_thumbnail(
                        &path,
                        Some(key_for_disk),
                        160,
                        disk_cache,
                        cancel,
                    )
                    .await;
                    (key, rgba)
                },
                |(key, rgba)| Message::ThumbnailReady { key, rgba },
            ));
        }

        Task::batch([Task::batch(tasks), self.queue_visible_system_icons()])
    }

    fn queue_visible_system_icons(&mut self) -> Task<Message> {
        if self.config.icon_theme_mode != IconThemeMode::HostSystem {
            return Task::none();
        }

        let mut needed: Vec<(SystemIconKey, SystemIconBucket)> = Vec::new();
        let mut seen: HashSet<(SystemIconKey, SystemIconBucket)> = HashSet::new();
        let mut push_icon = |key: SystemIconKey, bucket: SystemIconBucket| {
            let pair = (key, bucket);
            if seen.insert(pair.clone()) {
                needed.push(pair);
            }
        };

        for place in [
            SystemPlaceKind::Home,
            SystemPlaceKind::Desktop,
            SystemPlaceKind::Documents,
            SystemPlaceKind::Downloads,
            SystemPlaceKind::Music,
            SystemPlaceKind::Pictures,
            SystemPlaceKind::Videos,
            SystemPlaceKind::Public,
            SystemPlaceKind::Templates,
            SystemPlaceKind::Trash,
        ] {
            push_icon(
                SystemIconKey::SpecialPlace(place),
                SystemIconBucket::Small24,
            );
        }
        if self.sidebar_home_expanded {
            push_icon(SystemIconKey::Directory, SystemIconBucket::Small24);
        }
        for drive in &self.drives {
            push_icon(SystemIconKey::Drive(drive.kind), SystemIconBucket::Small24);
        }

        if let Some(tab) = self.current_tab() {
            let strings = self.strings();
            let entry_bucket = if tab.view_mode == ViewMode::Grid {
                SystemIconBucket::Large64
            } else {
                SystemIconBucket::Small24
            };
            match tab.view_mode {
                ViewMode::Grid => {
                    let card_w =
                        views::virtualized::clamp_grid_card_width(self.config.grid_card_width);
                    let card_h = views::virtualized::grid_card_height(card_w);
                    let fit_cols = views::virtualized::grid_columns_for_width(
                        self.effective_canvas_width(),
                        card_w,
                    );
                    let bands = views::virtualized::build_tab_grid_bands(
                        &tab.visible_entries,
                        fit_cols,
                        self.config.grid_group_folders,
                        strings,
                    );
                    let win = views::virtualized::compute_band_window_with_row_height(
                        &bands,
                        card_h,
                        self.canvas_scroll_y,
                        self.canvas_viewport_height,
                        280.0,
                    );
                    for band in &bands[win.range] {
                        if let views::virtualized::GridBand::Items(chunk) = band {
                            for &(_, entry) in chunk {
                                push_icon(
                                    SystemIconKey::for_entry(
                                        entry,
                                        fm_fs::is_archive_file(&entry.path),
                                    ),
                                    entry_bucket,
                                );
                            }
                        }
                    }
                }
                ViewMode::DetailTable | ViewMode::Columns => {
                    let step = if tab.view_mode == ViewMode::DetailTable {
                        views::virtualized::DETAIL_ROW_STEP
                    } else {
                        32.0
                    };
                    let win = views::virtualized::compute_fixed_window(
                        tab.visible_entries.len(),
                        step,
                        self.canvas_scroll_y,
                        self.canvas_viewport_height,
                        6,
                    );
                    for entry in &tab.visible_entries[win.range] {
                        push_icon(
                            SystemIconKey::for_entry(entry, fm_fs::is_archive_file(&entry.path)),
                            entry_bucket,
                        );
                    }
                    if tab.view_mode == ViewMode::Columns {
                        for (_, anc_entries) in &self.column_ancestors {
                            for child in anc_entries.iter().take(60) {
                                push_icon(
                                    SystemIconKey::for_entry(
                                        child,
                                        fm_fs::is_archive_file(&child.path),
                                    ),
                                    SystemIconBucket::Small24,
                                );
                            }
                        }
                        if let Some((_, children)) = &self.column_preview {
                            for child in children.iter().take(60) {
                                push_icon(
                                    SystemIconKey::for_entry(
                                        child,
                                        fm_fs::is_archive_file(&child.path),
                                    ),
                                    SystemIconBucket::Small24,
                                );
                            }
                        }
                    }
                }
            }

            if (self.config.show_inspector || self.quicklook_open)
                && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
            {
                push_icon(
                    SystemIconKey::for_entry(selected, fm_fs::is_archive_file(&selected.path)),
                    SystemIconBucket::Large64,
                );
            }
        }

        let generation = self.system_icon_generation;
        let cancel = self
            .tab_cancels
            .get(&self.active_tab)
            .cloned()
            .unwrap_or_default();

        let mut tasks = Vec::new();
        for (key, bucket) in needed {
            let pair = (key.clone(), bucket);
            if self.gpu_system_icons.contains_key(&pair) {
                continue;
            }
            if let Some(cached_opt) = self.system_icon_cache.get(&pair) {
                if let Some(thumb) = cached_opt {
                    if self.gpu_system_icons.len() >= fm_preview::DEFAULT_ICON_CACHE_CAPACITY {
                        self.gpu_system_icons.clear();
                    }
                    self.gpu_system_icons.insert(
                        pair,
                        image::Handle::from_rgba(thumb.width, thumb.height, thumb.pixels.clone()),
                    );
                }
                continue;
            }
            if self.in_flight_system_icons.contains(&pair) {
                continue;
            }
            self.in_flight_system_icons.insert(pair);
            let cancel = cancel.clone();
            tasks.push(Task::perform(
                async move {
                    let k_bg = key.clone();
                    let cancel_bg = cancel.clone();
                    let rgba = tokio::task::spawn_blocking(move || {
                        if cancel_bg.is_cancelled() {
                            return None;
                        }
                        let payload = fm_fs::resolve_system_icon(&k_bg, bucket)?;
                        rasterize_system_icon(payload, bucket, &cancel_bg)
                    })
                    .await
                    .ok()
                    .flatten();
                    (generation, key, bucket, rgba)
                },
                |(generation, key, bucket, rgba)| Message::SystemIconReady {
                    generation,
                    key,
                    bucket,
                    rgba,
                },
            ));
        }

        Task::batch(tasks)
    }

    fn handle_global_key(
        &mut self,
        window_id: window::Id,
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        let is_ctrl_q = matches!(
            key.as_ref(),
            keyboard::Key::Character(c) if c.eq_ignore_ascii_case("q") && modifiers.control()
        );
        if is_ctrl_q {
            if self.properties_windows.contains_key(&window_id) {
                return self.update(Message::ClosePropertiesWindow(window_id));
            }
            if self.prefs_open {
                return self.update(Message::ClosePreferences);
            }
            if self.deep_search_open {
                return self.update(Message::CloseDeepSearch);
            }
            if self.active_dialog.is_some() {
                return self.update(Message::CloseDialog);
            }
            if self.quicklook_open {
                return self.update(Message::CloseQuickLook);
            }
            return self.update(Message::WindowClose);
        }

        if self.properties_windows.contains_key(&window_id) {
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Escape)) {
                return self.update(Message::ClosePropertiesWindow(window_id));
            }
            return Task::none();
        }
        let text_input_modal_active = self.active_dialog.is_some()
            || self.deep_search_open
            || self.path_editing
            || self.prefs_open;
        if text_input_modal_active {
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Tab)) && self.path_editing {
                return self.autocomplete_path_draft();
            }
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
            keyboard::Key::Character("t") if modifiers.control() && modifiers.shift() => {
                self.update(Message::OpenTerminal)
            }
            keyboard::Key::Character("t") if modifiers.control() => self.update(Message::TabNew),
            keyboard::Key::Character("w") if modifiers.control() => {
                let id = self.active_tab;
                self.update(Message::TabClose(id))
            }
            keyboard::Key::Named(Named::Tab) if modifiers.control() => {
                self.update(Message::TabNext(modifiers.shift()))
            }
            keyboard::Key::Character("a") if modifiers.control() => self.update(Message::SelectAll),
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
            keyboard::Key::Named(Named::ArrowLeft) if modifiers.alt() => {
                self.update(Message::NavigateBack)
            }
            keyboard::Key::Named(Named::ArrowRight) if modifiers.alt() => {
                self.update(Message::NavigateForward)
            }
            keyboard::Key::Named(Named::ArrowUp) if modifiers.alt() => {
                self.update(Message::NavigateUp)
            }
            keyboard::Key::Named(Named::Backspace) if !self.quicklook_open => {
                self.update(Message::NavigateUp)
            }
            keyboard::Key::Named(Named::Space) if !modifiers.control() && !modifiers.alt() => {
                self.update(Message::ToggleQuickLook)
            }
            keyboard::Key::Named(Named::Escape) => {
                if self.context_menu.is_some() {
                    self.update(Message::CloseContextMenu)
                } else if self.breadcrumb_menu.is_some() {
                    self.update(Message::CloseBreadcrumbMenu)
                } else if self.quicklook_open {
                    self.update(Message::CloseQuickLook)
                } else {
                    self.update(Message::FilterChanged(String::new()))
                }
            }
            keyboard::Key::Character("=" | "+") if modifiers.control() => {
                self.update(Message::StepGridCardWidth(16.0))
            }
            keyboard::Key::Character("-") if modifiers.control() => {
                self.update(Message::StepGridCardWidth(-16.0))
            }
            keyboard::Key::Character("0") if modifiers.control() => {
                self.update(Message::ResetGridCardWidth)
            }
            keyboard::Key::Named(Named::PageDown) if !self.quicklook_open => {
                let page = (self.canvas_viewport_height * 0.85).max(200.0);
                operation::scroll_by(canvas_scroll_id(), AbsoluteOffset { x: 0.0, y: page })
            }
            keyboard::Key::Named(Named::PageUp) if !self.quicklook_open => {
                let page = (self.canvas_viewport_height * 0.85).max(200.0);
                operation::scroll_by(canvas_scroll_id(), AbsoluteOffset { x: 0.0, y: -page })
            }
            keyboard::Key::Named(Named::ArrowDown) => self.navigate_selection_arrow(0, 1),
            keyboard::Key::Named(Named::ArrowUp) => self.navigate_selection_arrow(0, -1),
            keyboard::Key::Named(Named::ArrowRight) => self.navigate_selection_arrow(1, 0),
            keyboard::Key::Named(Named::ArrowLeft) => self.navigate_selection_arrow(-1, 0),
            keyboard::Key::Named(Named::Enter) => {
                if let Some(focused) = self.current_tab().and_then(|t| t.selection.focused) {
                    return self.update(Message::ActivateEntry(focused));
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn open_tab_at(&mut self, path: PathBuf) -> Task<Message> {
        let id = TabId(self.next_tab_id);
        self.next_tab_id += 1;
        let req = self.alloc_request_id();
        let mut tab = TabState::new(id, path.clone());
        tab.view_mode = self.config.default_view_mode;
        tab.sort.dirs_first = self.config.dirs_first;
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

    fn autocomplete_path_draft(&mut self) -> Task<Message> {
        let raw = self.path_draft.trim().to_owned();
        if raw.is_empty() {
            return Task::none();
        }
        let ends_sep = raw.ends_with('/') || raw.ends_with('\\');
        let raw_path = PathBuf::from(&raw);
        let (parent, prefix) = if ends_sep {
            (raw_path, String::new())
        } else {
            let parent = raw_path
                .parent()
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| PathBuf::from("/"));
            let prefix = raw_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            (parent, prefix)
        };

        if let Some(tab) = self.current_tab()
            && tab.current_path == parent
        {
            let dir_names: Vec<String> = tab
                .raw_entries
                .iter()
                .filter(|e| e.kind.is_dir_like())
                .map(|e| e.name.to_string())
                .collect();
            return self.apply_path_completion(&parent, &prefix, &dir_names);
        }

        let base_draft = self.path_draft.clone();
        let vfs = Arc::clone(&self.vfs);
        Task::perform(
            async move {
                let cancel = CancellationToken::new();
                let dir_names = match vfs.list_dir(&parent, 0, cancel).await {
                    Ok(snap) => snap
                        .entries
                        .into_iter()
                        .filter(|e| e.kind.is_dir_like())
                        .map(|e| e.name.to_string())
                        .collect(),
                    Err(_) => Vec::new(),
                };
                (base_draft, parent, prefix, dir_names)
            },
            |(base_draft, parent, prefix, dir_names)| Message::PathAutocompleteLoaded {
                base_draft,
                parent,
                prefix,
                dir_names,
            },
        )
    }

    fn apply_path_completion(
        &mut self,
        parent: &std::path::Path,
        prefix: &str,
        dir_names: &[String],
    ) -> Task<Message> {
        let lower_prefix = prefix.to_lowercase();
        let mut matches: Vec<&str> = dir_names
            .iter()
            .map(String::as_str)
            .filter(|name| name.to_lowercase().starts_with(&lower_prefix))
            .collect();
        if matches.is_empty() {
            return Task::none();
        }
        matches.sort_by_key(|a| a.to_lowercase());

        if matches.len() == 1 {
            let mut completed = parent.join(matches[0]).to_string_lossy().into_owned();
            if !completed.ends_with(std::path::MAIN_SEPARATOR) {
                completed.push(std::path::MAIN_SEPARATOR);
            }
            self.path_draft = completed;
            return operation::move_cursor_to_end(path_input_id());
        }

        let common = longest_common_prefix(&matches);
        if common.len() > prefix.len() {
            self.path_draft = parent.join(common).to_string_lossy().into_owned();
            return operation::move_cursor_to_end(path_input_id());
        }
        // Quando múltiplos diretórios empatam exatamente no prefixo, cicla para o primeiro candidato.
        let mut first = parent.join(matches[0]).to_string_lossy().into_owned();
        if !first.ends_with(std::path::MAIN_SEPARATOR) {
            first.push(std::path::MAIN_SEPARATOR);
        }
        self.path_draft = first;
        operation::move_cursor_to_end(path_input_id())
    }

    fn effective_canvas_width(&self) -> f32 {
        if let Some(measured) = self.canvas_viewport_width {
            return measured.max(200.0);
        }
        let sidebar = if self.config.sidebar_collapsed {
            Spacing::SIDEBAR_COLLAPSED_WIDTH
        } else {
            views::sidebar::effective_sidebar_width(self, self.strings())
        };
        let inspector = if self.config.show_inspector {
            Spacing::INSPECTOR_WIDTH
        } else {
            0.0
        };
        (self.window_size.width - sidebar - inspector - 40.0).max(200.0)
    }

    fn navigate_selection_arrow(&mut self, dx: isize, dy: isize) -> Task<Message> {
        let Some(tab) = self.current_tab() else {
            return Task::none();
        };
        if tab.visible_entries.is_empty() {
            return Task::none();
        }

        let view_mode = tab.view_mode;
        let focused = tab.selection.focused;

        if view_mode == ViewMode::Columns && dy == 0 {
            if dx < 0 {
                return self.update(Message::NavigateUp);
            }
            if dx > 0
                && let Some(idx) = focused
                && let Some(entry) = tab.visible_entries.get(idx)
                && entry.kind.is_dir_like()
            {
                let path = entry.path.clone();
                return self.navigate_active_tab(path);
            }
            return Task::none();
        }

        let next_idx = if view_mode == ViewMode::Grid {
            let card_w = views::virtualized::clamp_grid_card_width(self.config.grid_card_width);
            let cols =
                views::virtualized::grid_columns_for_width(self.effective_canvas_width(), card_w);
            let strings = self.strings();
            let bands = views::virtualized::build_tab_grid_bands(
                &tab.visible_entries,
                cols,
                self.config.grid_group_folders,
                strings,
            );
            views::virtualized::step_grid_selection(&bands, focused, dx, dy)
        } else {
            let delta = if dy != 0 { dy } else { dx };
            let len = tab.visible_entries.len() as isize;
            let current = focused.unwrap_or(0) as isize;
            Some((current + delta).clamp(0, len - 1) as usize)
        };

        let Some(next_idx) = next_idx else {
            return Task::none();
        };

        let mut selected_entry = None;
        if let Some(tab_mut) = self.current_tab_mut() {
            tab_mut
                .selection
                .select_single(next_idx, &tab_mut.visible_entries);
            let single = tab_mut
                .selection
                .single_selected(&tab_mut.visible_entries)
                .cloned();
            tab_mut.selected_telemetry = single.as_ref().map(basic_file_telemetry);
            selected_entry = single;
        }

        let side_task = self.dispatch_selection_side_effects(selected_entry.as_ref());
        let scroll_task = self.scroll_entry_into_view(next_idx);
        Task::batch([side_task, scroll_task])
    }

    fn scroll_entry_into_view(&self, idx: usize) -> Task<Message> {
        let Some(tab) = self.current_tab() else {
            return Task::none();
        };
        let (top_y, bottom_y) = match tab.view_mode {
            ViewMode::Grid => {
                let card_w = views::virtualized::clamp_grid_card_width(self.config.grid_card_width);
                let row_h = views::virtualized::grid_card_height(card_w);
                let cols = views::virtualized::grid_columns_for_width(
                    self.effective_canvas_width(),
                    card_w,
                );
                let bands = views::virtualized::build_tab_grid_bands(
                    &tab.visible_entries,
                    cols,
                    self.config.grid_group_folders,
                    self.strings(),
                );
                let Some(span) = views::virtualized::grid_band_y_span(&bands, idx, row_h) else {
                    return Task::none();
                };
                span
            }
            ViewMode::DetailTable => {
                let top = idx as f32 * views::virtualized::DETAIL_ROW_STEP;
                (top, top + views::virtualized::DETAIL_ROW_HEIGHT)
            }
            ViewMode::Columns => {
                let top = idx as f32 * 32.0;
                (top, top + 28.0)
            }
        };

        let vh = self.canvas_viewport_height.max(120.0);
        if top_y < self.canvas_scroll_y {
            operation::scroll_to(
                canvas_scroll_id(),
                AbsoluteOffset {
                    x: None,
                    y: Some(top_y.max(0.0)),
                },
            )
        } else if bottom_y > self.canvas_scroll_y + vh {
            operation::scroll_to(
                canvas_scroll_id(),
                AbsoluteOffset {
                    x: None,
                    y: Some((bottom_y - vh).max(0.0)),
                },
            )
        } else {
            Task::none()
        }
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
                .send(Message::DirectoryChangedOnDisk {
                    watched_root: ev.watched_root,
                    has_visible_change: ev.has_visible_change,
                })
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
        .filter(|entry| {
            let hidden_ok = tab.show_hidden || !entry.is_hidden;
            let ext_ok = match tab.extension_filter.as_deref() {
                None => true,
                Some(DIR_FILTER_SENTINEL) => entry.kind.is_dir_like(),
                Some(ext) => entry
                    .extension
                    .as_deref()
                    .is_some_and(|e| e.eq_ignore_ascii_case(ext)),
            };
            hidden_ok && ext_ok
        })
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
    if fm_core::is_virtual_trash_path(path) {
        return CompactString::from("Trash");
    }
    let raw = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    CompactString::from(views::sanitize_single_line(&raw))
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
        git_status: None,
        preview_snippet: None,
    }
}

pub fn selected_id_set_contains(tab: &TabState, id: EntryId) -> bool {
    tab.selection.selected_ids.contains(&id)
}

fn longest_common_prefix<'a>(items: &[&'a str]) -> &'a str {
    let Some(first) = items.first().copied() else {
        return "";
    };
    let mut prefix_len = first.len();
    for item in &items[1..] {
        prefix_len = first
            .char_indices()
            .zip(item.chars())
            .take_while(|((_, a), b)| a.to_lowercase().eq(b.to_lowercase()))
            .last()
            .map(|((idx, a), _)| idx + a.len_utf8())
            .unwrap_or(0)
            .min(prefix_len);
    }
    &first[..prefix_len]
}

#[cfg(target_os = "windows")]
fn apply_window_vibrancy(id: window::Id, theme: ThemePreset) -> Task<Message> {
    let is_dark = theme == ThemePreset::ObsidianFlow;
    window::run(id, move |win| {
        let _ = window_vibrancy::apply_mica(win, Some(is_dark))
            .or_else(|_| window_vibrancy::apply_acrylic(win, None));
    })
    .discard()
}

#[cfg(not(target_os = "windows"))]
fn apply_window_vibrancy(_id: window::Id, _theme: ThemePreset) -> Task<Message> {
    Task::none()
}

fn apply_modifier_key_press(
    key: &keyboard::Key,
    mut modifiers: keyboard::Modifiers,
) -> keyboard::Modifiers {
    match key {
        keyboard::Key::Named(Named::Control) => modifiers.insert(keyboard::Modifiers::CTRL),
        keyboard::Key::Named(Named::Shift) => modifiers.insert(keyboard::Modifiers::SHIFT),
        keyboard::Key::Named(Named::Alt) => modifiers.insert(keyboard::Modifiers::ALT),
        keyboard::Key::Named(Named::Super) => modifiers.insert(keyboard::Modifiers::LOGO),
        _ => {}
    }
    modifiers
}

fn apply_modifier_key_release(
    key: &keyboard::Key,
    mut modifiers: keyboard::Modifiers,
) -> keyboard::Modifiers {
    match key {
        keyboard::Key::Named(Named::Control) => modifiers.remove(keyboard::Modifiers::CTRL),
        keyboard::Key::Named(Named::Shift) => modifiers.remove(keyboard::Modifiers::SHIFT),
        keyboard::Key::Named(Named::Alt) => modifiers.remove(keyboard::Modifiers::ALT),
        keyboard::Key::Named(Named::Super) => modifiers.remove(keyboard::Modifiers::LOGO),
        _ => {}
    }
    modifiers
}

fn fit_window_size_to_monitor(mon: Size) -> Size {
    let max_w = (mon.width - 24.0).max(760.0);
    let max_h = (mon.height - 96.0).max(500.0);
    let width = (mon.width * 0.86).clamp(760.0, 1320.0).min(max_w);
    let height = (mon.height * 0.82).clamp(500.0, 860.0).min(max_h);
    Size::new(width.round(), height.round())
}

fn wheel_zoom_delta(delta: mouse::ScrollDelta) -> f32 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => y * 16.0,
        mouse::ScrollDelta::Pixels { y, .. } => y * 0.25,
    }
}

fn resolve_cli_initial_dir(arg: &std::ffi::OsStr) -> Option<PathBuf> {
    let raw = arg.to_string_lossy();
    let candidate = if let Some(rest) = raw.strip_prefix("file://") {
        let decoded = percent_decode_uri_path(rest);
        #[cfg(target_os = "windows")]
        let normalized = decoded.strip_prefix('/').unwrap_or(&decoded).to_owned();
        #[cfg(not(target_os = "windows"))]
        let normalized = decoded;
        PathBuf::from(normalized)
    } else {
        PathBuf::from(arg)
    };
    if candidate.is_dir() {
        Some(candidate)
    } else if candidate.exists() {
        candidate.parent().filter(|p| p.is_dir()).map(PathBuf::from)
    } else {
        None
    }
}

fn percent_decode_uri_path(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(val) = u8::from_str_radix(hex_str, 16)
        {
            out.push(val);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Calcula a largura da barra lateral durante o arraste respeitando o limite mínimo, máximo e largura da janela.
fn compute_dragged_sidebar_width(
    initial_w: f32,
    start_x: f32,
    pos_x: f32,
    window_width: f32,
) -> f32 {
    let min_w = views::sidebar::SIDEBAR_WIDTH_MIN;
    let max_w = if window_width.is_finite() && window_width > min_w + 280.0 {
        (window_width - 280.0).min(views::sidebar::SIDEBAR_WIDTH_MAX)
    } else {
        views::sidebar::SIDEBAR_WIDTH_MAX
    };
    views::sidebar::clamp_sidebar_width(initial_w + (pos_x - start_x)).clamp(min_w, max_w)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_dragged_sidebar_width, fit_window_size_to_monitor, percent_decode_uri_path,
        resolve_cli_initial_dir, wheel_zoom_delta,
    };
    use iced::Size;
    use iced::mouse::ScrollDelta;
    use std::ffi::OsStr;

    #[test]
    fn resolves_cli_directory_and_file_uri() {
        let tmp = std::env::temp_dir();
        assert_eq!(resolve_cli_initial_dir(tmp.as_os_str()), Some(tmp.clone()));
        assert_eq!(
            percent_decode_uri_path("/tmp/My%20Folder"),
            "/tmp/My Folder"
        );
        assert!(resolve_cli_initial_dir(OsStr::new("/nonexistent_vagnarok_dir_xyz")).is_none());
    }

    #[test]
    fn fits_window_size_to_common_monitors() {
        let hd = fit_window_size_to_monitor(Size::new(1366.0, 768.0));
        assert!(hd.width <= 1342.0 && hd.width >= 1100.0);
        assert!(hd.height <= 672.0 && hd.height >= 500.0);

        let fhd = fit_window_size_to_monitor(Size::new(1920.0, 1080.0));
        assert_eq!(fhd, Size::new(1320.0, 860.0));
    }

    #[test]
    fn wheel_zoom_delta_maps_lines_and_pixels() {
        assert_eq!(
            wheel_zoom_delta(ScrollDelta::Lines { x: 0.0, y: 1.0 }),
            16.0
        );
        assert_eq!(
            wheel_zoom_delta(ScrollDelta::Lines { x: 0.0, y: -1.0 }),
            -16.0
        );
        assert_eq!(
            wheel_zoom_delta(ScrollDelta::Pixels { x: 0.0, y: 40.0 }),
            10.0
        );
        assert_eq!(
            wheel_zoom_delta(ScrollDelta::Pixels { x: 12.0, y: 0.0 }),
            0.0
        );
    }

    #[test]
    fn sidebar_resize_supports_dragging_right_and_left() {
        assert_eq!(
            compute_dragged_sidebar_width(350.0, 355.0, 435.0, 1280.0),
            430.0
        );
        assert_eq!(
            compute_dragged_sidebar_width(350.0, 355.0, 275.0, 1280.0),
            270.0
        );
        assert_eq!(
            compute_dragged_sidebar_width(350.0, 355.0, 900.0, 1280.0),
            super::views::sidebar::SIDEBAR_WIDTH_MAX
        );
        assert_eq!(
            compute_dragged_sidebar_width(350.0, 355.0, 700.0, 720.0),
            440.0
        );
    }
}
