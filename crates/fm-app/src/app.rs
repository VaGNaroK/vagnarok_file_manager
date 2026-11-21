use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use compact_str::CompactString;
use fm_core::{
    AppConfig, ClipboardTransfer, ConflictPolicy, CopyMode, DirectorySnapshot, DriveInfo,
    EntryExtraInfo, EntryId, FileEntry, FolderSummary, FsError, IconThemeMode, JobProgress,
    JobState, Language, OpenWithAssociation, OpenWithCatalog, PermissionsMeta, RecursiveSize,
    SelectedFileTelemetry, SelectionModifiers, SortDirection, SortField, Strings, SystemIconBucket,
    SystemIconKey, SystemPlaceKind, TabId, TabState, ThemePreset, UserPlaces, VfsProvider,
    ViewMode, VolumeSpace, WindowDecorationsMode,
};
use fm_fs::LocalFs;
use fm_index::SearchHit;
use fm_preview::{DiskThumbnailCache, ThumbnailKey, ThumbnailRgba, rasterize_system_icon};
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

use crate::archive_jobs::{
    ArchiveJobRuntime, ArchivePasswordModalState, ArchiveUiMsg, CompressModalState,
};
pub use crate::bulk_rename::{
    BulkRenameExtensionKind, BulkRenameInsertKind, BulkRenameModalState, BulkRenameMsg,
    effective_bulk_rename_modal_rect,
};
use crate::duplicate_finder::{DuplicateFinderMsg, DuplicateFinderState};
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

pub fn filter_ext_input_id() -> Id {
    Id::new("fm_filter_ext_input")
}

pub fn archive_password_input_id() -> Id {
    Id::new("fm_archive_password_input")
}

pub fn properties_name_input_id(window_id: window::Id) -> Id {
    Id::from(format!("fm_properties_name_input_{window_id:?}"))
}

/// Alvo de um campo de texto que suporta menu de contexto (Recortar/Copiar/Colar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputTarget {
    DialogInput,
    PropertiesName(window::Id),
    SearchBar,
    DeepSearchInput,
    PathDraft,
}

/// Estado do menu de contexto aberto sobre um `text_input`.
#[derive(Debug, Clone)]
pub struct TextInputMenuState {
    pub window_id: window::Id,
    pub target: TextInputTarget,
    pub position: Point,
    pub selection: Option<(usize, usize)>,
    pub cursor_index: usize,
}

#[derive(Debug, Clone)]
pub enum ContextMenuKind {
    Entry {
        index: usize,
        path: PathBuf,
        is_dir: bool,
        posix_mode: Option<u32>,
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
        is_ejectable: bool,
        is_disk_image: bool,
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
    pub send_to_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertiesTab {
    #[default]
    General,
    Security,
    History,
    Metadata,
    Permissions,
    OpenWith,
}

#[derive(Debug, Clone)]
pub struct ExecPickerState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPropertiesSummary {
    pub paths: Vec<PathBuf>,
    pub dir_count: usize,
    pub file_count: usize,
    pub common_parent: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PropertiesWindowState {
    pub request_id: u64,
    pub entry: FileEntry,
    pub multi: Option<MultiPropertiesSummary>,
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
    pub open_with_loading: bool,
    pub open_with_catalog: Option<OpenWithCatalog>,
    pub open_with_selected_id: Option<String>,
    pub open_with_custom_command: String,
    pub exec_picker: Option<ExecPickerState>,
    pub cancel: Option<CancellationToken>,
}

impl PropertiesWindowState {
    pub fn is_multi(&self) -> bool {
        self.multi.is_some()
    }

    pub fn supports_open_with(&self) -> bool {
        !self.is_multi()
            && !self.entry.kind.is_dir_like()
            && !self.is_archive()
            && !fm_core::is_virtual_trash_path(&self.entry.path)
    }

    pub fn is_archive(&self) -> bool {
        fm_fs::archive::is_inside_archive_path(&self.entry.path)
    }

    pub fn is_dirty(&self) -> bool {
        if self.is_multi() || self.is_archive() {
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

pub const MAX_UNDO_HISTORY: usize = 32;

/// Ação reversível registrada na pilha de Desfazer (`Ctrl+Z`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoAction {
    Trash { paths: Vec<PathBuf> },
    Rename { from: PathBuf, to: PathBuf },
    BulkRename { pairs: Vec<(PathBuf, PathBuf)> },
}

/// Identifica o painel esquerdo ou direito no modo de tela dividida (Dual Pane).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPane {
    Left,
    Right,
}

/// Estado do painel secundário (direito) no modo de tela dividida.
#[derive(Debug, Clone)]
pub struct SplitViewState {
    pub right_tab: TabState,
    pub active_pane: SplitPane,
    /// Proporção de largura do painel esquerdo (`0.25..=0.75`).
    pub ratio: f32,
    pub divider_hovered: bool,
    pub resizing: Option<(f32, f32)>,
    pub right_scroll_y: f32,
    pub right_viewport_height: f32,
    pub right_viewport_width: Option<f32>,
}

/// Estado de arraste interno (drag & drop entre painéis ou sobre subpastas).
#[derive(Debug, Clone)]
pub struct InternalEntryDrag {
    pub paths: Vec<PathBuf>,
    pub primary_label: String,
}

pub fn canvas_scroll_id() -> Id {
    Id::new("fm_canvas_scroll")
}

pub fn secondary_canvas_scroll_id() -> Id {
    Id::new("fm_canvas_scroll_secondary")
}

pub fn pane_scroll_id(pane: SplitPane) -> Id {
    match pane {
        SplitPane::Left => canvas_scroll_id(),
        SplitPane::Right => secondary_canvas_scroll_id(),
    }
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
    OpenManyInNewTabs(Vec<PathBuf>),
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
    MountDiskImage(PathBuf),
    DiskImageMounted {
        image_path: PathBuf,
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
    EjectDrive {
        device_path: Option<PathBuf>,
        mount_path: Option<PathBuf>,
        label: String,
    },
    DriveEjected {
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
        is_ejectable: bool,
        is_disk_image: bool,
    },
    SelectEntry {
        pane: SplitPane,
        index: usize,
        modifiers: SelectionModifiers,
    },
    ClearSelection(SplitPane),
    MarqueeSelect {
        pane: SplitPane,
        rect: views::virtualized::ContentRect,
        viewport_width: f32,
        additive: bool,
        is_initial: bool,
    },
    MarqueeFinished(SplitPane),
    SelectAll,
    ActivateEntry(usize),
    SetViewMode(ViewMode),
    ToggleSplitView,
    FocusSplitPane(SplitPane),
    SplitNavigateBack(SplitPane),
    SplitNavigateForward(SplitPane),
    SplitNavigateUp(SplitPane),
    StartSplitResize,
    SplitDividerHovered(bool),
    ResetSplitRatio,
    StartEntryDrag {
        pane: SplitPane,
        index: usize,
    },
    PanePointerEnter(SplitPane),
    PanePointerExit(SplitPane),
    DragHoverFolderEnter {
        pane: SplitPane,
        path: PathBuf,
    },
    DragHoverFolderExit(PathBuf),
    CycleSortField,
    SortByField {
        pane: SplitPane,
        field: SortField,
    },
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
        pane: SplitPane,
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
    SystemClipboardPasteRead(Option<String>),
    SystemClipboardPathsResolved {
        mode: CopyMode,
        paths: Vec<PathBuf>,
    },
    CancelTransfer,
    TransferProgress(JobProgress),
    FileOpFinished {
        result: Result<(), FsError>,
        select_after: Option<PathBuf>,
    },
    MoveToTrashFinished {
        paths: Vec<PathBuf>,
        result: Result<(), FsError>,
    },
    RenameOpFinished {
        from: PathBuf,
        to: PathBuf,
        result: Result<(), FsError>,
    },
    UndoLastAction,
    UndoTrashFinished {
        paths: Vec<PathBuf>,
        result: Result<usize, FsError>,
    },
    UndoRenameFinished {
        restored_path: PathBuf,
        result: Result<(), FsError>,
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
        clear_trash_undo: bool,
        result: Result<(), FsError>,
    },
    ConfigSaved(Option<String>),
    OpenDefaultFinished(Result<(), FsError>),
    OpenProperties(PathBuf),
    OpenWithDialog(PathBuf),
    PropertiesEntryStatLoaded {
        result: Result<FileEntry, FsError>,
        tab: PropertiesTab,
    },
    PropertiesFolderSizeReady {
        window_id: window::Id,
        request_id: u64,
        size: Option<RecursiveSize>,
    },
    PropertiesVolumeSpaceReady {
        window_id: window::Id,
        request_id: u64,
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
    PropertiesToggleExecutable(window::Id),
    ToggleEntryExecutable {
        path: PathBuf,
        next_mode: u32,
    },
    EntryExecutableToggled {
        path: PathBuf,
        result: Result<FileEntry, FsError>,
    },
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
    PropertiesOpenWithLoaded {
        window_id: window::Id,
        path: PathBuf,
        result: Result<OpenWithCatalog, FsError>,
    },
    OpenWithIconsBatchLoaded(Vec<(String, Option<image::Handle>)>),
    OpenWithSelectApp(window::Id, String),
    OpenWithCustomCommandChanged(window::Id, String),
    OpenWithAddCustom(window::Id),
    OpenWithSetDefault(window::Id),
    OpenWithResetDefault(window::Id),
    OpenWithSystemSyncFinished(Result<(), FsError>),
    OpenWithOpenNow(window::Id),
    OpenWithToggleExecPicker(window::Id),
    OpenWithExecPickerNavigate(window::Id, PathBuf),
    OpenWithExecPickerLoaded {
        window_id: window::Id,
        dir: PathBuf,
        result: Result<DirectorySnapshot, FsError>,
    },
    OpenWithExecPickerPick(window::Id, PathBuf),
    OpenWithExecPickerClose(window::Id),
    DragPropertiesWindow(window::Id),
    ClosePropertiesWindow(window::Id),
    OpenBulkRenameDialog,
    BulkRename(BulkRenameMsg),
    UndoLastBulkRename,
    UndoBulkRenameFinished(Result<usize, FsError>),
    Archive(ArchiveUiMsg),
    DuplicateFinder(DuplicateFinderMsg),
    OpenNewMenu,
    OpenFilterMenu,
    SetExtensionFilter(Option<CompactString>),
    FilterExtDraftChanged(String),
    SubmitFilterExtDraft,
    OpenEntryContextMenu {
        pane: SplitPane,
        index: usize,
    },
    OpenCanvasContextMenu(SplitPane),
    CloseContextMenu,
    OpenTextInputContextMenu {
        window_id: window::Id,
        target: TextInputTarget,
        position: Point,
        selection: Option<(usize, usize)>,
        cursor_index: usize,
    },
    CloseTextInputContextMenu,
    TextInputContextCut,
    TextInputContextCopy,
    TextInputContextPaste,
    TextInputContextPasteRead {
        target: TextInputTarget,
        selection: Option<(usize, usize)>,
        cursor_index: usize,
        text: Option<String>,
    },
    TextInputContextSelectAll,
    ContextMenuSetSendToOpen(bool),
    CreateShortcutInDir {
        dest_dir: Option<PathBuf>,
        fallback_target: PathBuf,
    },
    ShortcutsCreated {
        dest_dir: PathBuf,
        result: Result<Vec<PathBuf>, FsError>,
    },
    CopyPath(PathBuf),
    CopyText(String, String),
    StartColumnResize(usize),
    StartSidebarResize,
    StartPropertiesResize,
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
    pub table_col_weights: [f32; 5],
    pub resizing_col: Option<(usize, f32, [f32; 5])>,
    pub resizing_sidebar: Option<(f32, f32)>,
    pub sidebar_divider_hovered: bool,
    pub properties_modal_size: (f32, f32),
    pub properties_modal_offset: Option<(f32, f32)>,
    pub resizing_properties_modal: Option<(Point, f32, f32, f32, f32)>,
    pub dragging_properties_modal: Option<(Point, f32, f32)>,
    pub deep_search_open: bool,
    pub deep_search_query: String,
    pub deep_search_loading: bool,
    pub deep_search_results: Vec<SearchHit>,
    pub deep_search_request_id: u64,
    pub deep_search_cancel: Option<CancellationToken>,
    pub active_dialog: Option<ActiveDialog>,
    pub bulk_rename_modal: Option<BulkRenameModalState>,
    pub last_bulk_rename_undo: Option<Vec<(PathBuf, PathBuf)>>,
    pub undo_stack: Vec<UndoAction>,
    pub compress_modal: Option<CompressModalState>,
    pub archive_password_modal: Option<ArchivePasswordModalState>,
    pub archive_jobs: BTreeMap<u64, ArchiveJobRuntime>,
    pub duplicate_finder: Option<DuplicateFinderState>,
    pub prefs_open: bool,
    pub prefs_tab: PreferencesTab,
    pub clipboard: Option<ClipboardTransfer>,
    pub next_job_id: u64,
    pub active_transfer: Option<JobProgress>,
    pub transfer_cancel: Option<CancellationToken>,
    pub pending_select_path: Option<PathBuf>,
    pub pending_select_paths: Vec<PathBuf>,
    pub marquee_base_selection: HashSet<EntryId>,
    pub metadata_cache: Option<Arc<DiskMetadataCache>>,
    pub disk_thumbnail_cache: Option<Arc<DiskThumbnailCache>>,
    pub gpu_thumbnails: HashMap<ThumbnailKey, image::Handle>,
    pub in_flight_thumbs: HashSet<PathBuf>,
    pub gpu_system_icons: HashMap<(SystemIconKey, SystemIconBucket), image::Handle>,
    pub system_icon_misses: HashSet<(SystemIconKey, SystemIconBucket)>,
    pub in_flight_system_icons: HashSet<(SystemIconKey, SystemIconBucket)>,
    pub system_icon_generation: u64,
    pub open_with_icons: HashMap<String, image::Handle>,
    pub open_with_icons_in_flight: HashSet<String>,
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
    pub text_input_menu: Option<TextInputMenuState>,
    pub filter_ext_draft: String,
    pub pending_dropped_paths: Vec<PathBuf>,
    pub drop_batch_id: u64,
    pub canvas_scroll_y: f32,
    pub canvas_viewport_height: f32,
    pub canvas_viewport_width: Option<f32>,
    pub main_window_id: window::Id,
    pub properties_windows: HashMap<window::Id, PropertiesWindowState>,
    pub next_properties_request_id: u64,
    pub status_notice: Option<String>,
    pub last_entry_click: Option<(TabId, PathBuf, Instant)>,
    pub last_watcher_reload: Option<(PathBuf, Instant)>,
    pub grid_zoom_dirty: bool,
    pub zoom_save_seq: u64,
    pub split_view: Option<SplitViewState>,
    pub entry_drag: Option<InternalEntryDrag>,
    pub drag_hover_pane: Option<SplitPane>,
    pub drag_hover_folder: Option<(SplitPane, PathBuf)>,
    pub suppress_click_until: Option<Instant>,
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
            table_col_weights: [48.0, 11.0, 9.0, 17.0, 15.0],
            resizing_col: None,
            resizing_sidebar: None,
            sidebar_divider_hovered: false,
            properties_modal_size: (760.0, 560.0),
            properties_modal_offset: None,
            resizing_properties_modal: None,
            dragging_properties_modal: None,
            deep_search_open: false,
            deep_search_query: String::new(),
            deep_search_loading: false,
            deep_search_results: Vec::new(),
            deep_search_request_id: 0,
            deep_search_cancel: None,
            active_dialog: None,
            bulk_rename_modal: None,
            last_bulk_rename_undo: None,
            undo_stack: Vec::new(),
            compress_modal: None,
            archive_password_modal: None,
            archive_jobs: BTreeMap::new(),
            duplicate_finder: None,
            prefs_open: false,
            prefs_tab: PreferencesTab::General,
            clipboard: None,
            next_job_id: 1,
            active_transfer: None,
            transfer_cancel: None,
            pending_select_path: None,
            pending_select_paths: Vec::new(),
            marquee_base_selection: HashSet::new(),
            metadata_cache: None,
            disk_thumbnail_cache: None,
            gpu_thumbnails: HashMap::new(),
            in_flight_thumbs: HashSet::new(),
            gpu_system_icons: HashMap::new(),
            system_icon_misses: HashSet::new(),
            in_flight_system_icons: HashSet::new(),
            system_icon_generation: 0,
            open_with_icons: HashMap::new(),
            open_with_icons_in_flight: HashSet::new(),
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
            text_input_menu: None,
            filter_ext_draft: String::new(),
            pending_dropped_paths: Vec::new(),
            drop_batch_id: 0,
            canvas_scroll_y: 0.0,
            canvas_viewport_height: 700.0,
            canvas_viewport_width: None,
            main_window_id,
            properties_windows: HashMap::new(),
            next_properties_request_id: 0,
            status_notice: None,
            last_entry_click: None,
            last_watcher_reload: None,
            grid_zoom_dirty: false,
            zoom_save_seq: 0,
            split_view: None,
            entry_drag: None,
            drag_hover_pane: None,
            drag_hover_folder: None,
            suppress_click_until: None,
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
            if let Some(multi) = &props.multi {
                return format!(
                    "{}{}",
                    self.strings().properties_header_prefix,
                    self.strings().format_items_count(multi.paths.len())
                );
            }
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

    pub fn active_pane(&self) -> SplitPane {
        self.split_view
            .as_ref()
            .map(|s| s.active_pane)
            .unwrap_or(SplitPane::Left)
    }

    pub fn active_tab_id(&self) -> TabId {
        match &self.split_view {
            Some(split) if split.active_pane == SplitPane::Right => split.right_tab.id,
            _ => self.active_tab,
        }
    }

    pub fn left_tab(&self) -> Option<&TabState> {
        self.tabs.iter().find(|t| t.id == self.active_tab)
    }

    pub fn pane_tab(&self, pane: SplitPane) -> Option<&TabState> {
        match pane {
            SplitPane::Left => self.left_tab(),
            SplitPane::Right => self.split_view.as_ref().map(|s| &s.right_tab),
        }
    }

    pub fn pane_tab_mut(&mut self, pane: SplitPane) -> Option<&mut TabState> {
        match pane {
            SplitPane::Left => {
                let active = self.active_tab;
                self.tabs.iter_mut().find(|t| t.id == active)
            }
            SplitPane::Right => self.split_view.as_mut().map(|s| &mut s.right_tab),
        }
    }

    pub fn pane_scroll_metrics(&self, pane: SplitPane) -> (f32, f32, Option<f32>) {
        match pane {
            SplitPane::Left => (
                self.canvas_scroll_y,
                self.canvas_viewport_height,
                self.canvas_viewport_width,
            ),
            SplitPane::Right => self
                .split_view
                .as_ref()
                .map(|s| {
                    (
                        s.right_scroll_y,
                        s.right_viewport_height,
                        s.right_viewport_width,
                    )
                })
                .unwrap_or((0.0, self.canvas_viewport_height, None)),
        }
    }

    fn set_active_scroll_y(&mut self, y: f32) {
        match self.active_pane() {
            SplitPane::Left => self.canvas_scroll_y = y,
            SplitPane::Right => {
                if let Some(split) = &mut self.split_view {
                    split.right_scroll_y = y;
                }
            }
        }
    }

    fn active_canvas_scroll_id(&self) -> Id {
        pane_scroll_id(self.active_pane())
    }

    fn find_tab_mut(&mut self, id: TabId) -> Option<&mut TabState> {
        find_tab_mut_parts(&mut self.tabs, &mut self.split_view, id)
    }

    fn focus_split_pane(&mut self, pane: SplitPane) {
        if let Some(split) = &mut self.split_view
            && split.active_pane != pane
        {
            split.active_pane = pane;
            self.path_editing = false;
            self.quicklook_open = false;
            self.breadcrumb_menu = None;
            self.last_entry_click = None;
        }
    }

    pub fn current_tab(&self) -> Option<&TabState> {
        self.pane_tab(self.active_pane())
    }

    pub fn is_current_tab_trash(&self) -> bool {
        self.current_tab().is_some_and(|tab| {
            fm_core::is_virtual_trash_path(&tab.current_path)
                || self
                    .places
                    .trash
                    .as_ref()
                    .is_some_and(|tr| tab.current_path.starts_with(tr))
        })
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut TabState> {
        self.pane_tab_mut(self.active_pane())
    }

    pub fn text_input_value(&self, target: &TextInputTarget) -> String {
        match target {
            TextInputTarget::DialogInput => match &self.active_dialog {
                Some(ActiveDialog::CreateItem { name, .. })
                | Some(ActiveDialog::Rename { name, .. }) => name.clone(),
                _ => String::new(),
            },
            TextInputTarget::PropertiesName(win_id) => self
                .properties_windows
                .get(win_id)
                .map(|p| p.draft_name.clone())
                .unwrap_or_default(),
            TextInputTarget::SearchBar => self
                .current_tab()
                .map(|t| t.filter_query.clone())
                .unwrap_or_default(),
            TextInputTarget::DeepSearchInput => self.deep_search_query.clone(),
            TextInputTarget::PathDraft => self.path_draft.clone(),
        }
    }

    fn set_text_input_value(&mut self, target: &TextInputTarget, val: String) -> Task<Message> {
        match target {
            TextInputTarget::DialogInput => self.update(Message::DialogInputChanged(val)),
            TextInputTarget::PropertiesName(win_id) => {
                self.update(Message::PropertiesNameChanged(*win_id, val))
            }
            TextInputTarget::SearchBar => self.update(Message::FilterChanged(val)),
            TextInputTarget::DeepSearchInput => self.update(Message::DeepSearchQueryChanged(val)),
            TextInputTarget::PathDraft => self.update(Message::PathDraftChanged(val)),
        }
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
            Message::OpenManyInNewTabs(paths) => {
                self.context_menu = None;
                self.open_tabs_at(paths)
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
                    self.focus_split_pane(SplitPane::Left);
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
                let active = self.active_tab_id();
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
                let active = self.active_tab_id();
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
                let active = self.active_tab_id();
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
            Message::MountDiskImage(image_path) => {
                self.context_menu = None;
                let img_for_cb = image_path.clone();
                Task::perform(fm_fs::mount_disk_image(image_path), move |result| {
                    Message::DiskImageMounted {
                        image_path: img_for_cb,
                        result,
                    }
                })
            }
            Message::DiskImageMounted { image_path, result } => {
                let refresh_task = self.update(Message::RefreshDrives);
                match result {
                    Ok(mount_path) => {
                        self.status_notice = Some(format!(
                            "{}: {}",
                            self.strings().image_mounted,
                            mount_path.to_string_lossy()
                        ));
                        let tab_task = self.open_tab_at(mount_path);
                        Task::batch([refresh_task, tab_task])
                    }
                    Err(err) => {
                        let _ = image_path;
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
                let (had_open_tab, release_task) = if let Some(mp) = mount_path.as_deref() {
                    self.release_tabs_under_mount(mp)
                } else {
                    (false, Task::none())
                };
                let mp_for_cb = mount_path.clone();
                let label_for_cb = label.clone();
                let unmount_task = Task::perform(
                    async move {
                        if had_open_tab {
                            tokio::time::sleep(Duration::from_millis(120)).await;
                        }
                        fm_fs::unmount_drive(device_path, mount_path).await
                    },
                    move |result| Message::DriveUnmounted {
                        mount_path: mp_for_cb,
                        label: label_for_cb,
                        result,
                    },
                );
                Task::batch([release_task, unmount_task])
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
                        let nav_task = if let Some(mp) = mount_path.as_deref() {
                            self.release_tabs_under_mount(mp).1
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
            Message::EjectDrive {
                device_path,
                mount_path,
                label,
            } => {
                self.context_menu = None;
                let (had_open_tab, release_task) = if let Some(mp) = mount_path.as_deref() {
                    self.release_tabs_under_mount(mp)
                } else {
                    (false, Task::none())
                };
                let mp_for_cb = mount_path.clone();
                let label_for_cb = label.clone();
                let eject_task = Task::perform(
                    async move {
                        if had_open_tab {
                            tokio::time::sleep(Duration::from_millis(120)).await;
                        }
                        fm_fs::eject_drive(device_path, mount_path).await
                    },
                    move |result| Message::DriveEjected {
                        mount_path: mp_for_cb,
                        label: label_for_cb,
                        result,
                    },
                );
                Task::batch([release_task, eject_task])
            }
            Message::DriveEjected {
                mount_path,
                label,
                result,
            } => {
                let refresh_task = self.update(Message::RefreshDrives);
                match result {
                    Ok(()) => {
                        self.status_notice =
                            Some(format!("{}: {label}", self.strings().drive_ejected));
                        let nav_task = if let Some(mp) = mount_path.as_deref() {
                            self.release_tabs_under_mount(mp).1
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
                is_ejectable,
                is_disk_image,
            } => {
                self.breadcrumb_menu = None;
                self.context_menu = Some(ContextMenuState {
                    kind: ContextMenuKind::Drive {
                        label,
                        mount_path,
                        device_path,
                        is_mounted,
                        can_unmount,
                        is_ejectable,
                        is_disk_image,
                    },
                    position: self.cursor_position,
                    send_to_open: false,
                });
                Task::none()
            }
            Message::SelectEntry {
                pane,
                index,
                modifiers,
            } => {
                if self.entry_drag.is_some()
                    || self
                        .suppress_click_until
                        .is_some_and(|until| Instant::now() < until)
                {
                    return Task::none();
                }
                self.context_menu = None;
                self.focus_split_pane(pane);
                let active_id = self.active_tab_id();
                let clicked_entry = self
                    .current_tab()
                    .and_then(|t| t.visible_entries.get(index))
                    .map(|e| (active_id, e.path.clone()));
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

                let visual_order = if modifiers.shift {
                    self.current_tab_visual_order()
                } else {
                    Vec::new()
                };
                let mut single_entry = None;
                if let Some(tab) = self.current_tab_mut() {
                    if modifiers.shift {
                        tab.selection.select_range_in_order(
                            index,
                            &visual_order,
                            modifiers.ctrl,
                            &tab.visible_entries,
                        );
                    } else {
                        tab.selection
                            .apply_click(index, modifiers, &tab.visible_entries);
                    }
                    single_entry = tab.selection.single_selected(&tab.visible_entries).cloned();
                    tab.selected_telemetry = single_entry.as_ref().map(basic_file_telemetry);
                }
                self.dispatch_selection_side_effects(single_entry.as_ref())
            }
            Message::ClearSelection(pane) => {
                self.context_menu = None;
                self.focus_split_pane(pane);
                self.last_entry_click = None;
                self.marquee_base_selection.clear();
                if let Some(tab) = self.current_tab_mut() {
                    tab.selection.clear();
                    tab.selected_telemetry = None;
                }
                Task::none()
            }
            Message::MarqueeSelect {
                pane,
                rect,
                viewport_width,
                additive,
                is_initial,
            } => {
                self.context_menu = None;
                self.focus_split_pane(pane);
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
            Message::MarqueeFinished(pane) => {
                self.focus_split_pane(pane);
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
                    if fm_fs::archive::is_inside_archive_path(&entry.path) {
                        return self.update(Message::ToggleQuickLook);
                    }
                    let path = entry.path;
                    return self.open_file_respecting_preference(path);
                }
                Task::none()
            }
            Message::SetViewMode(mode) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.view_mode = mode;
                }
                self.set_active_scroll_y(0.0);
                let scroll_id = self.active_canvas_scroll_id();
                let thumb_task = self.queue_visible_thumbnails();
                Task::batch([
                    operation::snap_to(scroll_id, RelativeOffset::START),
                    thumb_task,
                ])
            }
            Message::ToggleSplitView => {
                self.context_menu = None;
                self.entry_drag = None;
                self.drag_hover_pane = None;
                self.drag_hover_folder = None;
                if let Some(split) = self.split_view.take() {
                    if let Some(cancel) = self.tab_cancels.remove(&split.right_tab.id) {
                        cancel.cancel();
                    }
                    self.canvas_viewport_width = None;
                    return Task::none();
                }

                let (initial_path, view_mode, sort, show_hidden) = self
                    .left_tab()
                    .map(|t| (t.current_path.clone(), t.view_mode, t.sort, t.show_hidden))
                    .unwrap_or_else(|| {
                        (
                            self.places.home.clone(),
                            self.config.default_view_mode,
                            fm_core::SortSpec {
                                dirs_first: self.config.dirs_first,
                                ..fm_core::SortSpec::default()
                            },
                            false,
                        )
                    });
                let right_id = TabId(self.next_tab_id);
                self.next_tab_id += 1;
                let req = self.alloc_request_id();
                let mut right_tab = TabState::new(right_id, initial_path.clone());
                right_tab.view_mode = view_mode;
                right_tab.sort = sort;
                right_tab.show_hidden = show_hidden;
                right_tab.active_request_id = req;
                right_tab.is_loading = true;
                self.canvas_viewport_width = None;
                self.split_view = Some(SplitViewState {
                    right_tab,
                    active_pane: SplitPane::Left,
                    ratio: 0.5,
                    divider_hovered: false,
                    resizing: None,
                    right_scroll_y: 0.0,
                    right_viewport_height: self.canvas_viewport_height,
                    right_viewport_width: None,
                });
                let cancel = self.reset_tab_cancel(right_id);
                Task::batch([
                    spawn_list_task(Arc::clone(&self.vfs), right_id, initial_path, req, cancel),
                    operation::snap_to(secondary_canvas_scroll_id(), RelativeOffset::START),
                ])
            }
            Message::FocusSplitPane(pane) => {
                self.context_menu = None;
                self.focus_split_pane(pane);
                Task::none()
            }
            Message::SplitNavigateBack(pane) => {
                self.focus_split_pane(pane);
                self.update(Message::NavigateBack)
            }
            Message::SplitNavigateForward(pane) => {
                self.focus_split_pane(pane);
                self.update(Message::NavigateForward)
            }
            Message::SplitNavigateUp(pane) => {
                self.focus_split_pane(pane);
                self.update(Message::NavigateUp)
            }
            Message::StartSplitResize => {
                if let Some(split) = &mut self.split_view {
                    split.resizing = Some((self.cursor_position.x, split.ratio));
                }
                Task::none()
            }
            Message::SplitDividerHovered(hovered) => {
                if let Some(split) = &mut self.split_view {
                    split.divider_hovered = hovered;
                }
                Task::none()
            }
            Message::ResetSplitRatio => {
                if let Some(split) = &mut self.split_view {
                    split.ratio = 0.5;
                    split.resizing = None;
                    split.right_viewport_width = None;
                }
                self.canvas_viewport_width = None;
                Task::none()
            }
            Message::StartEntryDrag { pane, index } => {
                self.context_menu = None;
                let drag_info = self.pane_tab_mut(pane).and_then(|tab| {
                    let entry = tab.visible_entries.get(index)?;
                    if fm_core::is_virtual_trash_path(&entry.path)
                        || fm_fs::archive::is_inside_archive_path(&entry.path)
                    {
                        return None;
                    }
                    if !tab.selection.selected_ids.contains(&entry.id) {
                        tab.selection.select_single(index, &tab.visible_entries);
                    }
                    let (paths, primary_label) = collect_drag_paths(
                        index,
                        &tab.visible_entries,
                        &tab.selection.selected_ids,
                    )?;
                    Some(InternalEntryDrag {
                        paths,
                        primary_label,
                    })
                });
                if let Some(drag) = drag_info {
                    self.focus_split_pane(pane);
                    self.entry_drag = Some(drag);
                    self.drag_hover_pane = Some(pane);
                    self.drag_hover_folder = None;
                }
                Task::none()
            }
            Message::PanePointerEnter(pane) => {
                if self.entry_drag.is_some() {
                    self.drag_hover_pane = Some(pane);
                }
                Task::none()
            }
            Message::PanePointerExit(pane) => {
                if self.drag_hover_pane == Some(pane) {
                    self.drag_hover_pane = None;
                }
                if self
                    .drag_hover_folder
                    .as_ref()
                    .is_some_and(|(p, _)| *p == pane)
                {
                    self.drag_hover_folder = None;
                }
                Task::none()
            }
            Message::DragHoverFolderEnter { pane, path } => {
                if self.entry_drag.is_some() {
                    self.drag_hover_pane = Some(pane);
                    self.drag_hover_folder = Some((pane, path));
                }
                Task::none()
            }
            Message::DragHoverFolderExit(path) => {
                if self
                    .drag_hover_folder
                    .as_ref()
                    .is_some_and(|(_, p)| p == &path)
                {
                    self.drag_hover_folder = None;
                }
                Task::none()
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
            Message::SortByField { pane, field } => {
                self.focus_split_pane(pane);
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
                self.set_active_scroll_y(0.0);
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
                        send_to_open: false,
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
                pane,
                offset_y,
                viewport_height,
                viewport_width,
            } => {
                match pane {
                    SplitPane::Left => {
                        self.canvas_scroll_y = offset_y;
                        if viewport_height > 0.0 {
                            self.canvas_viewport_height = viewport_height;
                        }
                        if viewport_width > 0.0 {
                            self.canvas_viewport_width = Some(viewport_width);
                        }
                    }
                    SplitPane::Right => {
                        if let Some(split) = &mut self.split_view {
                            split.right_scroll_y = offset_y;
                            if viewport_height > 0.0 {
                                split.right_viewport_height = viewport_height;
                            }
                            if viewport_width > 0.0 {
                                split.right_viewport_width = Some(viewport_width);
                            }
                        }
                    }
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
                let active_id = self.active_tab_id();
                let is_visible_pane = tab_id == self.active_tab
                    || self
                        .split_view
                        .as_ref()
                        .is_some_and(|s| s.right_tab.id == tab_id);

                if let Some(tab) = find_tab_mut_parts(&mut self.tabs, &mut self.split_view, tab_id)
                {
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

                            if tab_id == active_id {
                                let pending_batch = std::mem::take(&mut self.pending_select_paths);
                                if !pending_batch.is_empty() {
                                    let target_set: HashSet<&PathBuf> =
                                        pending_batch.iter().collect();
                                    tab.selection.selected_ids.clear();
                                    let mut first_idx = None;
                                    for (idx, entry) in tab.visible_entries.iter().enumerate() {
                                        if target_set.contains(&entry.path) {
                                            tab.selection.selected_ids.insert(entry.id);
                                            if first_idx.is_none() {
                                                first_idx = Some(idx);
                                            }
                                        }
                                    }
                                    tab.selection.anchor = first_idx;
                                    tab.selection.focused = first_idx;
                                    if let Some(idx) = first_idx {
                                        let single = (tab.selection.selected_ids.len() == 1)
                                            .then(|| tab.visible_entries[idx].clone());
                                        tab.selected_telemetry =
                                            single.as_ref().map(basic_file_telemetry);
                                        pending_selected_entry = single;
                                    }
                                    self.pending_select_path = None;
                                } else if let Some(target_path) = self.pending_select_path.take()
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

                if pending_selected_entry.is_some() {
                    tasks.push(
                        self.dispatch_selection_side_effects(pending_selected_entry.as_ref()),
                    );
                } else if is_visible_pane {
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
                let active = self.active_tab_id();
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
                if let Some(tab) = self.find_tab_mut(tab_id)
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
                if let Some(tab) = self.find_tab_mut(tab_id)
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
                if let Some(tab) = self.find_tab_mut(tab_id)
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
                if let Some(q) = self.current_tab().map(|t| t.filter_query.clone())
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
                    if self.gpu_thumbnails.len() >= fm_preview::DEFAULT_THUMBNAIL_CACHE_CAPACITY {
                        self.gpu_thumbnails.clear();
                    }
                    self.gpu_thumbnails.insert(key, rgba_arc_to_handle(thumb));
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
                if let Some(thumb) = rgba {
                    self.system_icon_misses.remove(&(key.clone(), bucket));
                    if self.gpu_system_icons.len() >= fm_preview::DEFAULT_ICON_CACHE_CAPACITY {
                        self.gpu_system_icons.clear();
                    }
                    self.gpu_system_icons
                        .insert((key, bucket), rgba_arc_to_handle(thumb));
                } else {
                    self.system_icon_misses.insert((key, bucket));
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
                let selected_count = self
                    .current_tab()
                    .map(|t| t.selection.selected_ids.len())
                    .unwrap_or(0);
                if selected_count > 1 {
                    return crate::bulk_rename::open_bulk_rename_modal(self);
                }
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
            Message::OpenBulkRenameDialog => crate::bulk_rename::open_bulk_rename_modal(self),
            Message::BulkRename(msg) => crate::bulk_rename::handle_bulk_rename_msg(self, msg),
            Message::UndoLastBulkRename => crate::bulk_rename::undo_last_bulk_rename(self),
            Message::UndoBulkRenameFinished(res) => match res {
                Ok(count) => {
                    self.status_notice = Some(self.strings().format_items_count(count));
                    self.update(Message::RefreshActiveTab)
                }
                Err(err) => {
                    self.pending_select_paths.clear();
                    self.status_notice = Some(err.to_string());
                    self.update(Message::RefreshActiveTab)
                }
            },
            Message::Archive(msg) => crate::archive_jobs::handle_archive_msg(self, msg),
            Message::DuplicateFinder(msg) => {
                crate::duplicate_finder::handle_duplicate_finder_msg(self, msg)
            }
            Message::RequestDelete(permanent) => {
                self.context_menu = None;
                let paths = self.selected_paths();
                if paths.is_empty() {
                    return Task::none();
                }
                let permanent = permanent || self.is_current_tab_trash();
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
            Message::SystemClipboardPasteRead(raw) => {
                let Some(text) = raw else {
                    return Task::none();
                };
                let Some((mode, candidates)) = parse_clipboard_transfer_text(&text) else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            candidates
                                .into_iter()
                                .filter(|p| p.exists())
                                .collect::<Vec<_>>()
                        })
                        .await
                        .unwrap_or_default()
                    },
                    move |paths| Message::SystemClipboardPathsResolved { mode, paths },
                )
            }
            Message::SystemClipboardPathsResolved { mode, paths } => {
                if paths.is_empty() {
                    return Task::none();
                }
                self.start_transfer_with_conflict_check(mode, paths)
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
                        self.refresh_visible_panes()
                    }
                    JobState::Cancelled => {
                        self.active_transfer = None;
                        self.transfer_cancel = None;
                        self.refresh_visible_panes()
                    }
                    JobState::Failed(msg) => {
                        self.active_transfer = None;
                        self.transfer_cancel = None;
                        self.status_notice = Some(msg.clone());
                        self.refresh_visible_panes()
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
                    self.refresh_visible_panes()
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::MoveToTrashFinished { paths, result } => match result {
                Ok(()) => {
                    if !paths.is_empty() {
                        self.push_undo(UndoAction::Trash { paths });
                        self.status_notice =
                            Some(self.strings().moved_to_trash_undo_hint.to_owned());
                    } else {
                        self.status_notice = None;
                    }
                    let active = self.refresh_visible_panes();
                    let trash_tab = self.reload_matching_dir_tabs(std::path::Path::new(
                        fm_core::VIRTUAL_TRASH_PATH,
                    ));
                    Task::batch([active, trash_tab])
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::RenameOpFinished { from, to, result } => match result {
                Ok(()) => {
                    self.status_notice = None;
                    self.pending_select_path = Some(to.clone());
                    self.push_undo(UndoAction::Rename { from, to });
                    self.refresh_visible_panes()
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::UndoLastAction => {
                self.context_menu = None;
                let action = self.undo_stack.pop().or_else(|| {
                    self.last_bulk_rename_undo
                        .take()
                        .map(|pairs| UndoAction::BulkRename { pairs })
                });
                let Some(action) = action else {
                    return Task::none();
                };
                match action {
                    UndoAction::Trash { paths } => {
                        let vfs = Arc::clone(&self.vfs);
                        let req_paths = paths.clone();
                        Task::perform(
                            async move { vfs.restore_from_trash(&req_paths).await },
                            move |result| Message::UndoTrashFinished { paths, result },
                        )
                    }
                    UndoAction::Rename { from, to } => {
                        let vfs = Arc::clone(&self.vfs);
                        let src = to.clone();
                        let dst = from.clone();
                        Task::perform(async move { vfs.rename(&src, &dst).await }, move |result| {
                            Message::UndoRenameFinished {
                                restored_path: from,
                                result,
                            }
                        })
                    }
                    UndoAction::BulkRename { pairs } => {
                        self.last_bulk_rename_undo = None;
                        let batch_id = self.next_job_id;
                        self.next_job_id = self.next_job_id.wrapping_add(1).max(1);
                        self.pending_select_paths =
                            pairs.iter().map(|(_, old)| old.clone()).collect();
                        Task::perform(
                            fm_fs::undo_bulk_rename_pairs(
                                pairs,
                                batch_id,
                                fm_core::BulkRenameTargetPolicy::host_default(),
                            ),
                            Message::UndoBulkRenameFinished,
                        )
                    }
                }
            }
            Message::UndoTrashFinished { paths, result } => match result {
                Ok(0) => {
                    self.status_notice = Some(self.strings().undo_nothing_in_trash.to_owned());
                    Task::none()
                }
                Ok(count) => {
                    self.status_notice = Some(self.strings().format_undo_restored(count));
                    self.pending_select_paths = paths.clone();
                    let mut dirs: HashSet<PathBuf> = paths
                        .iter()
                        .filter_map(|p| p.parent().map(std::path::Path::to_path_buf))
                        .collect();
                    dirs.insert(PathBuf::from(fm_core::VIRTUAL_TRASH_PATH));
                    let tasks: Vec<Task<Message>> = dirs
                        .into_iter()
                        .map(|dir| self.reload_matching_dir_tabs(&dir))
                        .collect();
                    Task::batch(tasks)
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::UndoRenameFinished {
                restored_path,
                result,
            } => match result {
                Ok(()) => {
                    self.status_notice = Some(self.strings().undo_rename_done.to_owned());
                    self.pending_select_path = Some(restored_path.clone());
                    if let Some(parent) = restored_path.parent() {
                        self.reload_matching_dir_tabs(parent)
                    } else {
                        self.update(Message::RefreshActiveTab)
                    }
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
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
                    send_to_open: false,
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
                    send_to_open: false,
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
                        clear_trash_undo: false,
                        result,
                    },
                )
            }
            Message::TrashActionFinished {
                notice,
                clear_trash_undo,
                result,
            } => {
                match result {
                    Ok(()) => {
                        if clear_trash_undo {
                            self.undo_stack
                                .retain(|a| !matches!(a, UndoAction::Trash { .. }));
                        }
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
                        send_to_open: false,
                    });
                }
                Task::none()
            }
            Message::OpenEntryContextMenu { pane, index } => {
                self.breadcrumb_menu = None;
                self.focus_split_pane(pane);
                let mut side_task = Task::none();
                let mut target_meta = None;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(entry) = tab.visible_entries.get(index)
                {
                    let already_selected = tab.selection.selected_ids.contains(&entry.id);
                    let is_dir = entry.kind.is_dir_like();
                    let posix_mode = if !is_dir
                        && !fm_core::is_virtual_trash_path(&entry.path)
                        && !fm_fs::archive::is_inside_archive_path(&entry.path)
                    {
                        match entry.permissions {
                            PermissionsMeta::PosixMode(mode) => Some(mode),
                            PermissionsMeta::WinAttributes(_) => None,
                        }
                    } else {
                        None
                    };
                    target_meta = Some((entry.path.clone(), is_dir, posix_mode));
                    if !already_selected {
                        tab.selection.select_single(index, &tab.visible_entries);
                        let single = tab.selection.single_selected(&tab.visible_entries).cloned();
                        tab.selected_telemetry = single.as_ref().map(basic_file_telemetry);
                        side_task = self.dispatch_selection_side_effects(single.as_ref());
                    }
                }
                if let Some((path, is_dir, posix_mode)) = target_meta {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::Entry {
                            index,
                            path,
                            is_dir,
                            posix_mode,
                        },
                        position: self.cursor_position,
                        send_to_open: false,
                    });
                }
                side_task
            }
            Message::OpenCanvasContextMenu(pane) => {
                self.breadcrumb_menu = None;
                self.focus_split_pane(pane);
                self.context_menu = Some(ContextMenuState {
                    kind: ContextMenuKind::Background,
                    position: self.cursor_position,
                    send_to_open: false,
                });
                Task::none()
            }
            Message::CloseContextMenu => {
                self.context_menu = None;
                self.filter_ext_draft.clear();
                Task::none()
            }
            Message::OpenTextInputContextMenu {
                window_id,
                target,
                position,
                selection,
                cursor_index,
            } => {
                self.context_menu = None;
                self.text_input_menu = Some(TextInputMenuState {
                    window_id,
                    target,
                    position,
                    selection,
                    cursor_index,
                });
                Task::none()
            }
            Message::CloseTextInputContextMenu => {
                self.text_input_menu = None;
                Task::none()
            }
            Message::TextInputContextCopy => {
                let Some(menu) = self.text_input_menu.take() else {
                    return Task::none();
                };
                let value = self.text_input_value(&menu.target);
                let input_id = text_input_id_for_target(&menu.target);
                let mut tasks = vec![operation::focus(input_id.clone())];
                if let Some((start, end)) = menu.selection {
                    tasks.push(operation::select_range(input_id, start, end));
                }
                if let Some(copied) = extract_text_input_copy(&value, menu.selection) {
                    tasks.push(clipboard::write(copied));
                }
                Task::batch(tasks)
            }
            Message::TextInputContextCut => {
                let Some(menu) = self.text_input_menu.take() else {
                    return Task::none();
                };
                let value = self.text_input_value(&menu.target);
                let input_id = text_input_id_for_target(&menu.target);
                let Some((cut_text, remaining, next_cursor)) =
                    apply_text_input_cut(&value, menu.selection)
                else {
                    return operation::focus(input_id);
                };
                let side = self.set_text_input_value(&menu.target, remaining);
                Task::batch([
                    clipboard::write(cut_text),
                    side,
                    operation::focus(input_id.clone()),
                    operation::move_cursor_to(input_id, next_cursor),
                ])
            }
            Message::TextInputContextPaste => {
                let Some(menu) = self.text_input_menu.take() else {
                    return Task::none();
                };
                let target = menu.target;
                let selection = menu.selection;
                let cursor_index = menu.cursor_index;
                clipboard::read().map(move |text| Message::TextInputContextPasteRead {
                    target: target.clone(),
                    selection,
                    cursor_index,
                    text,
                })
            }
            Message::TextInputContextPasteRead {
                target,
                selection,
                cursor_index,
                text,
            } => {
                let input_id = text_input_id_for_target(&target);
                let Some(pasted) = text else {
                    return operation::focus(input_id);
                };
                let current = self.text_input_value(&target);
                let (next_val, next_cursor) =
                    apply_text_input_paste(&current, selection, cursor_index, &pasted);
                let side = self.set_text_input_value(&target, next_val);
                Task::batch([
                    side,
                    operation::focus(input_id.clone()),
                    operation::move_cursor_to(input_id, next_cursor),
                ])
            }
            Message::TextInputContextSelectAll => {
                let Some(menu) = self.text_input_menu.take() else {
                    return Task::none();
                };
                let input_id = text_input_id_for_target(&menu.target);
                Task::batch([
                    operation::focus(input_id.clone()),
                    operation::select_all(input_id),
                ])
            }
            Message::ContextMenuSetSendToOpen(open) => {
                if let Some(menu) = self.context_menu.as_mut() {
                    menu.send_to_open = open;
                }
                Task::none()
            }
            Message::CreateShortcutInDir {
                dest_dir,
                fallback_target,
            } => {
                self.context_menu = None;
                let Some(resolved_dest) = dest_dir.or_else(|| self.places.desktop.clone()) else {
                    self.status_notice = Some(self.strings().shortcut_desktop_missing.to_owned());
                    return Task::none();
                };
                let mut targets = self.selected_paths();
                if targets.is_empty() {
                    targets.push(fallback_target);
                }
                targets.retain(|p| {
                    !fm_core::is_virtual_trash_path(p) && !fm_fs::archive::is_inside_archive_path(p)
                });
                if targets.is_empty() {
                    return Task::none();
                }
                let vfs = Arc::clone(&self.vfs);
                let dest_for_task = resolved_dest.clone();
                Task::perform(
                    async move { vfs.create_shortcuts(&targets, &dest_for_task).await },
                    move |result| Message::ShortcutsCreated {
                        dest_dir: resolved_dest.clone(),
                        result,
                    },
                )
            }
            Message::ShortcutsCreated { dest_dir, result } => match result {
                Ok(created) => {
                    if !created.is_empty() {
                        self.status_notice =
                            Some(self.strings().format_shortcuts_created(created.len()));
                    }
                    self.reload_matching_dir_tabs(&dest_dir)
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
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
                self.open_properties_request(path, PropertiesTab::General)
            }
            Message::OpenWithDialog(path) => {
                self.open_properties_request(path, PropertiesTab::OpenWith)
            }
            Message::PropertiesEntryStatLoaded { result, tab } => match result {
                Ok(entry) => self.open_properties_window(entry, tab),
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
            Message::PropertiesFolderSizeReady {
                window_id,
                request_id,
                size,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.request_id == request_id
                {
                    props.recursive_size = size;
                    props.size_loading = false;
                }
                Task::none()
            }
            Message::PropertiesVolumeSpaceReady {
                window_id,
                request_id,
                space,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.request_id == request_id
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
                    if props.is_multi() {
                        props.active_tab = PropertiesTab::General;
                        return Task::none();
                    }
                    props.active_tab = tab;
                }
                if tab == PropertiesTab::OpenWith {
                    return self.maybe_load_open_with_catalog(window_id);
                }
                Task::none()
            }
            Message::PropertiesNameChanged(window_id, val) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && !props.is_multi()
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
                    && !props.is_multi()
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
                    && !props.is_multi()
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
                    && !props.is_multi()
                    && !props.is_archive()
                    && !props.applying
                    && let Some(mode) = &mut props.posix_mode_draft
                {
                    *mode ^= bit;
                    props.readonly_draft = (*mode & 0o222) == 0;
                }
                Task::none()
            }
            Message::PropertiesToggleExecutable(window_id) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && !props.is_multi()
                    && !props.is_archive()
                    && !props.applying
                    && !props.entry.kind.is_dir_like()
                    && let Some(mode) = &mut props.posix_mode_draft
                {
                    *mode = fm_core::toggle_posix_executable_mode(*mode);
                    props.readonly_draft = (*mode & 0o222) == 0;
                }
                Task::none()
            }
            Message::ToggleEntryExecutable { path, next_mode } => {
                self.context_menu = None;
                let target_path = path.clone();
                Task::perform(
                    async move {
                        let patch = fm_fs::PropertyPatch {
                            readonly: None,
                            hidden: None,
                            posix_mode: Some(next_mode),
                        };
                        if let Err(err) =
                            fm_fs::apply_entry_properties(target_path.clone(), patch).await
                        {
                            return (target_path, Err(err));
                        }
                        let stat = fm_fs::stat_entry(target_path.clone()).await;
                        (target_path, stat)
                    },
                    |(path, result)| Message::EntryExecutableToggled { path, result },
                )
            }
            Message::EntryExecutableToggled { path, result } => match result {
                Ok(updated_entry) => {
                    for tab in &mut self.tabs {
                        if let Some(raw) = tab.raw_entries.iter_mut().find(|e| e.path == path) {
                            *raw = updated_entry.clone();
                            apply_tab_filter(tab);
                            if let Some(single) =
                                tab.selection.single_selected(&tab.visible_entries)
                                && single.path == path
                            {
                                tab.selected_telemetry = Some(basic_file_telemetry(single));
                            }
                        }
                    }
                    for props in self.properties_windows.values_mut() {
                        if !props.is_multi() && props.entry.path == path {
                            if let PermissionsMeta::PosixMode(mode) = updated_entry.permissions {
                                let perms = mode & 0o777;
                                props.posix_mode_draft = Some(perms);
                                props.initial_posix_mode = Some(perms);
                            }
                            props.entry = updated_entry.clone();
                        }
                    }
                    self.status_notice = Some(self.strings().prop_applied_notice.to_owned());
                    Task::none()
                }
                Err(err) => {
                    self.status_notice = Some(err.to_string());
                    Task::none()
                }
            },
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
            Message::PropertiesOpenDefault(path) => self.open_file_respecting_preference(path),
            Message::PropertiesOpenWithLoaded {
                window_id,
                path,
                result,
            } => {
                let mut icon_task = Task::none();
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && props.entry.path == path
                {
                    props.open_with_loading = false;
                    match result {
                        Ok(catalog) => {
                            let saved_default = self
                                .config
                                .open_with_defaults
                                .get(catalog.type_key.as_str())
                                .map(|a| a.app_id.clone());
                            if props.open_with_selected_id.is_none() {
                                props.open_with_selected_id = saved_default
                                    .or_else(|| {
                                        catalog.system_default.as_ref().map(|a| a.id.to_string())
                                    })
                                    .or_else(|| {
                                        catalog.recommended.first().map(|a| a.id.to_string())
                                    });
                            }
                            let batch_cancel = props.cancel.clone().unwrap_or_default();
                            icon_task = self.queue_open_with_icons(&catalog, batch_cancel);
                            if let Some(props_mut) = self.properties_windows.get_mut(&window_id) {
                                props_mut.open_with_catalog = Some(catalog);
                            }
                        }
                        Err(err) => {
                            if !matches!(err, FsError::Cancelled) {
                                self.status_notice = Some(err.to_string());
                            }
                        }
                    }
                }
                icon_task
            }
            Message::OpenWithIconsBatchLoaded(items) => {
                for (hint, handle) in items {
                    self.open_with_icons_in_flight.remove(&hint);
                    if let Some(h) = handle {
                        if self.open_with_icons.len() >= 64 {
                            self.open_with_icons.clear();
                        }
                        self.open_with_icons.insert(hint, h);
                    }
                }
                Task::none()
            }
            Message::OpenWithSelectApp(window_id, app_id) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id) {
                    props.open_with_selected_id = Some(app_id);
                }
                Task::none()
            }
            Message::OpenWithCustomCommandChanged(window_id, val) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id) {
                    props.open_with_custom_command = val;
                    if !props.open_with_custom_command.trim().is_empty() {
                        props.open_with_selected_id = None;
                    }
                }
                Task::none()
            }
            Message::OpenWithAddCustom(window_id) => {
                let Some(props) = self.properties_windows.get(&window_id).cloned() else {
                    return Task::none();
                };
                let cmd = props.open_with_custom_command.trim();
                let Some(assoc) = custom_association_from_command(cmd) else {
                    return Task::none();
                };
                let type_key = open_with_props_type_key(&props);
                let list = self
                    .config
                    .open_with_custom_apps
                    .entry(type_key.clone())
                    .or_default();
                if let Some(existing) = list.iter_mut().find(|a| a.app_id == assoc.app_id) {
                    *existing = assoc.clone();
                } else {
                    list.push(assoc.clone());
                }
                if let Some(props_mut) = self.properties_windows.get_mut(&window_id) {
                    props_mut.open_with_selected_id = Some(assoc.app_id.clone());
                    props_mut.open_with_custom_command.clear();
                }
                let mut custom_hints = Vec::new();
                if let Some(hint) = assoc.icon_hint.as_deref() {
                    custom_hints.push(hint.to_owned());
                }
                let batch_cancel = self
                    .properties_windows
                    .get(&window_id)
                    .and_then(|p| p.cancel.clone())
                    .unwrap_or_default();
                let icon_task = self.queue_open_with_icon_hints(custom_hints, batch_cancel);
                let save_task = self.persist_config_task();
                Task::batch([save_task, icon_task])
            }
            Message::OpenWithSetDefault(window_id) => {
                let Some(props) = self.properties_windows.get(&window_id).cloned() else {
                    return Task::none();
                };
                let Some((type_key, assoc)) = self.resolve_open_with_selection(&props) else {
                    return Task::none();
                };
                self.config
                    .open_with_defaults
                    .insert(type_key.clone(), assoc.clone());
                if assoc.app_id.starts_with("custom:") {
                    let list = self
                        .config
                        .open_with_custom_apps
                        .entry(type_key.clone())
                        .or_default();
                    if !list.iter().any(|a| a.app_id == assoc.app_id) {
                        list.push(assoc.clone());
                    }
                }
                if let Some(props_mut) = self.properties_windows.get_mut(&window_id) {
                    props_mut.open_with_selected_id = Some(assoc.app_id.clone());
                }
                self.status_notice = Some(self.strings().open_with_saved.to_owned());
                let save_cfg = self.persist_config_task();
                let sync_sys = Task::perform(
                    fm_fs::save_open_with_system_default(type_key, assoc),
                    Message::OpenWithSystemSyncFinished,
                );
                Task::batch([save_cfg, sync_sys])
            }
            Message::OpenWithResetDefault(window_id) => {
                let Some(props) = self.properties_windows.get(&window_id).cloned() else {
                    return Task::none();
                };
                let type_key = open_with_props_type_key(&props);
                self.config.open_with_defaults.remove(&type_key);
                if let Some(ext_key) = extension_fallback_key(&props.entry.path) {
                    self.config.open_with_defaults.remove(&ext_key);
                }
                if let Some(props_mut) = self.properties_windows.get_mut(&window_id) {
                    props_mut.open_with_selected_id = None;
                    props_mut.open_with_catalog = None;
                }
                self.status_notice = Some(self.strings().open_with_reset_done.to_owned());
                let save_cfg = self.persist_config_task();
                let reset_sys = Task::perform(
                    fm_fs::reset_open_with_system_default(type_key),
                    Message::OpenWithSystemSyncFinished,
                );
                let reload = self.maybe_load_open_with_catalog(window_id);
                Task::batch([save_cfg, reset_sys, reload])
            }
            Message::OpenWithSystemSyncFinished(result) => {
                if let Err(err) = result {
                    self.status_notice = Some(err.to_string());
                }
                Task::none()
            }
            Message::OpenWithOpenNow(window_id) => {
                let Some(props) = self.properties_windows.get(&window_id) else {
                    return Task::none();
                };
                let Some((_, assoc)) = self.resolve_open_with_selection(props) else {
                    return Task::none();
                };
                let path = props.entry.path.clone();
                let vfs = Arc::clone(&self.vfs);
                Task::perform(
                    async move { vfs.open_with(&path, &assoc).await },
                    Message::OpenDefaultFinished,
                )
            }
            Message::OpenWithToggleExecPicker(window_id) => {
                let Some(props) = self.properties_windows.get_mut(&window_id) else {
                    return Task::none();
                };
                if props.exec_picker.is_some() {
                    props.exec_picker = None;
                    return Task::none();
                }
                let start_dir = default_exec_picker_dir(&self.places);
                props.exec_picker = Some(ExecPickerState {
                    current_dir: start_dir.clone(),
                    entries: Vec::new(),
                    loading: true,
                    error: None,
                });
                self.load_exec_picker_dir(window_id, start_dir)
            }
            Message::OpenWithExecPickerNavigate(window_id, dir) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && let Some(picker) = &mut props.exec_picker
                {
                    picker.current_dir = dir.clone();
                    picker.loading = true;
                    picker.error = None;
                }
                self.load_exec_picker_dir(window_id, dir)
            }
            Message::OpenWithExecPickerLoaded {
                window_id,
                dir,
                result,
            } => {
                if let Some(props) = self.properties_windows.get_mut(&window_id)
                    && let Some(picker) = &mut props.exec_picker
                    && picker.current_dir == dir
                {
                    picker.loading = false;
                    match result {
                        Ok(snap) => {
                            let mut entries: Vec<FileEntry> =
                                snap.entries.into_iter().filter(|e| !e.is_hidden).collect();
                            let sort = fm_core::SortSpec::default();
                            entries.sort_by(|a, b| sort.compare(a, b));
                            picker.entries = entries;
                            picker.error = None;
                        }
                        Err(err) => {
                            picker.entries.clear();
                            picker.error = Some(err.to_string());
                        }
                    }
                }
                Task::none()
            }
            Message::OpenWithExecPickerPick(window_id, path) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id) {
                    props.open_with_custom_command = quote_exec_path_if_needed(&path);
                    props.open_with_selected_id = None;
                    props.exec_picker = None;
                }
                Task::none()
            }
            Message::OpenWithExecPickerClose(window_id) => {
                if let Some(props) = self.properties_windows.get_mut(&window_id) {
                    props.exec_picker = None;
                }
                Task::none()
            }
            Message::DragPropertiesWindow(window_id) => {
                if window_id == self.main_window_id {
                    let (x, y, _, _) = effective_properties_modal_rect(self);
                    self.properties_modal_offset = Some((x, y));
                    self.dragging_properties_modal = Some((self.cursor_position, x, y));
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
                if self.properties_windows.is_empty() {
                    self.open_with_icons.clear();
                    self.open_with_icons.shrink_to_fit();
                    self.open_with_icons_in_flight.clear();
                    self.dragging_properties_modal = None;
                    self.resizing_properties_modal = None;
                    self.properties_modal_offset = None;
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
                    self.filter_ext_draft.clear();
                    Task::none()
                } else {
                    self.filter_ext_draft.clear();
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::FilterMenu,
                        position: self.cursor_position,
                        send_to_open: false,
                    });
                    operation::focus(filter_ext_input_id())
                }
            }
            Message::SetExtensionFilter(ext) => {
                self.context_menu = None;
                self.filter_ext_draft.clear();
                if let Some(tab) = self.current_tab_mut() {
                    tab.extension_filter = ext;
                    apply_tab_filter(tab);
                }
                self.canvas_scroll_y = 0.0;
                self.queue_visible_thumbnails()
            }
            Message::FilterExtDraftChanged(val) => {
                self.filter_ext_draft = val;
                let normalized = fm_core::normalize_extension_filter_input(&self.filter_ext_draft);
                if let Some(tab) = self.current_tab_mut() {
                    tab.extension_filter = normalized;
                    apply_tab_filter(tab);
                }
                self.canvas_scroll_y = 0.0;
                self.queue_visible_thumbnails()
            }
            Message::SubmitFilterExtDraft => {
                self.context_menu = None;
                self.filter_ext_draft.clear();
                Task::none()
            }
            Message::StartColumnResize(idx) => {
                if idx < 4 {
                    self.resizing_col = Some((idx, self.cursor_position.x, self.table_col_weights));
                }
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
            Message::StartPropertiesResize => {
                let (x, y, w, h) = effective_properties_modal_rect(self);
                self.properties_modal_offset = Some((x, y));
                self.resizing_properties_modal = Some((self.cursor_position, x, y, w, h));
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
                self.resizing_properties_modal = None;
                self.dragging_properties_modal = None;
                if let Some(split) = &mut self.split_view {
                    split.resizing = None;
                }
                if let Some(modal) = &mut self.bulk_rename_modal {
                    modal.dragging_modal = None;
                    modal.resizing_modal = None;
                }
                if let Some(modal) = &mut self.compress_modal {
                    modal.dragging_modal = None;
                }
                if let Some(modal) = &mut self.duplicate_finder {
                    modal.dragging_modal = None;
                    modal.resizing_modal = None;
                }
                let mut tasks = Vec::new();
                if self.resizing_sidebar.take().is_some() {
                    tasks.push(self.persist_config_task());
                }
                if self.entry_drag.is_some() {
                    tasks.push(self.finish_entry_drag());
                }
                Task::batch(tasks)
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
                if let Some((start_x, initial_w)) = self.resizing_sidebar {
                    self.config.sidebar_width = Some(compute_dragged_sidebar_width(
                        initial_w,
                        start_x,
                        pos.x,
                        self.window_size.width,
                    ));
                }
                if let Some(split) = &mut self.split_view
                    && let Some((start_x, initial_ratio)) = split.resizing
                {
                    let sidebar_w = if self.config.sidebar_collapsed {
                        Spacing::SIDEBAR_COLLAPSED_WIDTH
                    } else {
                        self.config.sidebar_width.unwrap_or(Spacing::SIDEBAR_WIDTH)
                    };
                    let inspector_w = if self.config.show_inspector {
                        Spacing::INSPECTOR_WIDTH
                    } else {
                        0.0
                    };
                    let avail =
                        (self.window_size.width - sidebar_w - inspector_w - 40.0).max(320.0);
                    split.ratio = compute_split_ratio(initial_ratio, start_x, pos.x, avail);
                    split.right_viewport_width = None;
                    self.canvas_viewport_width = None;
                }
                if let Some((start, start_x, start_y)) = self.dragging_properties_modal {
                    let (w, h) = effective_properties_modal_size(self);
                    self.properties_modal_offset = Some(clamp_properties_modal_pos(
                        start_x + (pos.x - start.x),
                        start_y + (pos.y - start.y),
                        w,
                        h,
                        self.window_size,
                    ));
                }
                if let Some((start, anchor_x, anchor_y, initial_w, initial_h)) =
                    self.resizing_properties_modal
                {
                    self.properties_modal_size = compute_dragged_properties_modal_size(
                        anchor_x,
                        anchor_y,
                        initial_w,
                        initial_h,
                        start,
                        pos,
                        self.window_size,
                    );
                }
                if let Some(modal) = &mut self.bulk_rename_modal {
                    if let Some((start, start_x, start_y)) = modal.dragging_modal {
                        let (w, h) = crate::bulk_rename::clamp_modal_size(
                            modal.modal_size.0,
                            modal.modal_size.1,
                            crate::bulk_rename::BULK_RENAME_MIN_W,
                            crate::bulk_rename::BULK_RENAME_MIN_H,
                            self.window_size,
                        );
                        modal.modal_offset = Some(crate::bulk_rename::clamp_modal_pos(
                            start_x + (pos.x - start.x),
                            start_y + (pos.y - start.y),
                            w,
                            h,
                            self.window_size,
                        ));
                    }
                    if let Some((start, anchor_x, anchor_y, initial_w, initial_h)) =
                        modal.resizing_modal
                    {
                        modal.modal_size = crate::bulk_rename::compute_dragged_modal_size(
                            (anchor_x, anchor_y),
                            (initial_w, initial_h),
                            (
                                crate::bulk_rename::BULK_RENAME_MIN_W,
                                crate::bulk_rename::BULK_RENAME_MIN_H,
                            ),
                            start,
                            pos,
                            self.window_size,
                        );
                    }
                }
                if let Some(modal) = &mut self.compress_modal
                    && let Some((start, start_x, start_y)) = modal.dragging_modal
                {
                    modal.modal_offset = Some(crate::archive_jobs::clamp_compress_drag_pos(
                        start,
                        start_x,
                        start_y,
                        pos,
                        self.window_size,
                    ));
                }
                if let Some(modal) = &mut self.duplicate_finder {
                    if let Some((start, start_x, start_y)) = modal.dragging_modal {
                        modal.modal_offset =
                            Some(crate::duplicate_finder::clamp_duplicate_drag_pos(
                                modal,
                                start,
                                start_x,
                                start_y,
                                pos,
                                self.window_size,
                            ));
                    }
                    if let Some((start, anchor_x, anchor_y, initial_w, initial_h)) =
                        modal.resizing_modal
                    {
                        modal.modal_size = crate::duplicate_finder::clamp_duplicate_resize_size(
                            anchor_x,
                            anchor_y,
                            initial_w,
                            initial_h,
                            start,
                            pos,
                            self.window_size,
                        );
                    }
                }
                Task::none()
            }
            Message::MetaCacheReady(cache) => {
                self.metadata_cache = cache;
                Task::none()
            }
            Message::WindowResized { window_id, size } => {
                if window_id == self.main_window_id {
                    self.window_size = size;
                    self.canvas_viewport_width = None;
                    if let Some(split) = &mut self.split_view {
                        split.right_viewport_width = None;
                    }
                }
                Task::none()
            }
            Message::MonitorSizeReady(Some(mon)) => {
                let fitted = fit_window_size_to_monitor(mon);
                self.window_size = fitted;
                self.canvas_viewport_width = None;
                if let Some(split) = &mut self.split_view {
                    split.right_viewport_width = None;
                }
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
                    if let Some(mut modal) = self.bulk_rename_modal.take()
                        && let Some(cancel) = modal.tags_cancel.take()
                    {
                        cancel.cancel();
                    }
                    for (_, runtime) in std::mem::take(&mut self.archive_jobs) {
                        runtime.cancel.cancel();
                    }
                    if let Some(mut dup) = self.duplicate_finder.take()
                        && let Some(cancel) = dup.cancel.take()
                    {
                        cancel.cancel();
                    }
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
                if self
                    .text_input_menu
                    .as_ref()
                    .is_some_and(|m| m.window_id == window_id)
                {
                    self.text_input_menu = None;
                }
                if window_id == self.main_window_id {
                    self.current_modifiers = keyboard::Modifiers::empty();
                    self.entry_drag = None;
                    self.drag_hover_pane = None;
                    self.drag_hover_folder = None;
                    if let Some(split) = &mut self.split_view {
                        split.resizing = None;
                    }
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
                        keyboard::Key::Named(Named::Escape | Named::F3 | Named::F5 | Named::Tab)
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
            Subscription::run(watch_drives_stream),
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
            let props_view =
                views::modals::properties_window_view(self, window_id, props, palette, strings);
            if let Some(menu) = self
                .text_input_menu
                .as_ref()
                .filter(|m| m.window_id == window_id)
            {
                let menu_layer = views::modals::text_input_context_menu_overlay(
                    self,
                    menu,
                    Size::new(760.0, 560.0),
                    palette,
                    strings,
                );
                return stack![props_view, menu_layer].into();
            }
            return props_view;
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
        let overlay: Option<Element<'_, Message>> = if let Some(modal) = &self.bulk_rename_modal {
            Some(views::bulk_rename::bulk_rename_modal_overlay(
                self, modal, palette, strings,
            ))
        } else if let Some(modal) = &self.archive_password_modal {
            Some(views::archive_modal::archive_password_modal_overlay(
                modal, palette, strings,
            ))
        } else if let Some(modal) = self.compress_modal.as_ref().filter(|m| !m.minimized) {
            Some(views::archive_modal::compress_modal_overlay(
                self, modal, palette, strings,
            ))
        } else if let Some(modal) = self.duplicate_finder.as_ref().filter(|m| !m.minimized) {
            Some(views::duplicate_finder::duplicate_finder_modal_overlay(
                self, modal, palette, strings,
            ))
        } else if let Some(dialog) = &self.active_dialog {
            Some(views::modals::dialog_modal(
                dialog,
                self.main_window_id,
                palette,
                strings,
            ))
        } else if let Some(props) = self.properties_windows.get(&self.main_window_id) {
            Some(views::modals::properties_modal_overlay(
                self,
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

        let drag_overlay: Option<Element<'_, Message>> = self
            .entry_drag
            .as_ref()
            .map(|drag| views::canvas::drag_preview_overlay(self, drag, palette, strings));

        let text_menu_overlay: Option<Element<'_, Message>> = self
            .text_input_menu
            .as_ref()
            .filter(|m| m.window_id == self.main_window_id)
            .map(|menu| {
                views::modals::text_input_context_menu_overlay(
                    self,
                    menu,
                    self.window_size,
                    palette,
                    strings,
                )
            });

        let mut layers: Vec<Element<'_, Message>> = vec![main_shell.into()];
        if let Some(layer) = overlay {
            layers.push(layer);
        }
        if let Some(drag_layer) = drag_overlay {
            layers.push(drag_layer);
        }
        if let Some(text_menu) = text_menu_overlay {
            layers.push(text_menu);
        }
        iced::widget::Stack::with_children(layers).into()
    }

    /// Libera todas as abas (ativas, inativas e painel dividido) posicionadas dentro de `mount_path`,
    /// movendo-as para `Home` e encerrando leituras/watchers pendentes antes de desmontar/ejetar.
    fn release_tabs_under_mount(&mut self, mount_path: &std::path::Path) -> (bool, Task<Message>) {
        let mut affected_ids: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|tab| tab.current_path.starts_with(mount_path))
            .map(|tab| tab.id)
            .collect();
        if let Some(split) = &self.split_view
            && split.right_tab.current_path.starts_with(mount_path)
        {
            affected_ids.push(split.right_tab.id);
        }
        if affected_ids.is_empty() {
            return (false, Task::none());
        }
        let home = self.places.home.clone();
        let active_id = self.active_tab_id();
        let mut tasks = Vec::with_capacity(affected_ids.len());
        for tab_id in affected_ids {
            if tab_id == active_id {
                tasks.push(self.navigate_active_tab(home.clone()));
            } else {
                let req = self.alloc_request_id();
                if let Some(tab) = find_tab_mut_parts(&mut self.tabs, &mut self.split_view, tab_id)
                {
                    tab.push_navigation(home.clone());
                    prepare_tab_for_load(tab, req);
                }
                let cancel = self.reset_tab_cancel(tab_id);
                tasks.push(spawn_list_task(
                    Arc::clone(&self.vfs),
                    tab_id,
                    home.clone(),
                    req,
                    cancel,
                ));
            }
        }
        (true, Task::batch(tasks))
    }

    fn alloc_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    /// Registra uma ação reversível na pilha de Desfazer (`Ctrl+Z`), respeitando `MAX_UNDO_HISTORY`.
    pub(crate) fn push_undo(&mut self, action: UndoAction) {
        push_undo_entry(&mut self.undo_stack, action);
    }

    /// Recarrega o painel ativo e, no modo de tela dividida, também o painel secundário.
    fn refresh_visible_panes(&mut self) -> Task<Message> {
        let mut ids = vec![self.active_tab];
        if let Some(split) = &self.split_view {
            ids.push(split.right_tab.id);
        }
        let mut tasks = Vec::with_capacity(ids.len());
        for tab_id in ids {
            let req = self.alloc_request_id();
            let target_path = self.find_tab_mut(tab_id).map(|tab| {
                prepare_tab_for_load(tab, req);
                tab.current_path.clone()
            });
            if let Some(path) = target_path {
                let cancel = self.reset_tab_cancel(tab_id);
                tasks.push(spawn_list_task(
                    Arc::clone(&self.vfs),
                    tab_id,
                    path,
                    req,
                    cancel,
                ));
            }
        }
        Task::batch(tasks)
    }

    /// Recarrega todas as abas abertas (incluindo painel dividido) cujo diretório coincide com `dir`.
    pub(crate) fn reload_matching_dir_tabs(&mut self, dir: &std::path::Path) -> Task<Message> {
        let mut matching_ids: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|tab| tab.current_path == dir)
            .map(|tab| tab.id)
            .collect();
        if let Some(split) = &self.split_view
            && split.right_tab.current_path == dir
        {
            matching_ids.push(split.right_tab.id);
        }
        if matching_ids.is_empty() {
            return Task::none();
        }
        let mut tasks = Vec::with_capacity(matching_ids.len());
        for tab_id in matching_ids {
            let req = self.alloc_request_id();
            let target_path = self.find_tab_mut(tab_id).map(|tab| {
                prepare_tab_for_load(tab, req);
                tab.current_path.clone()
            });
            if let Some(path) = target_path {
                let cancel = self.reset_tab_cancel(tab_id);
                tasks.push(spawn_list_task(
                    Arc::clone(&self.vfs),
                    tab_id,
                    path,
                    req,
                    cancel,
                ));
            }
        }
        Task::batch(tasks)
    }

    fn open_properties_request(&mut self, path: PathBuf, tab: PropertiesTab) -> Task<Message> {
        self.context_menu = None;
        if tab == PropertiesTab::General {
            let selected_entries: Vec<FileEntry> = self
                .current_tab()
                .map(|t| {
                    t.visible_entries
                        .iter()
                        .filter(|e| t.selection.selected_ids.contains(&e.id))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            if selected_entries.len() > 1 && selected_entries.iter().any(|e| e.path == path) {
                return self.open_multi_properties_window(selected_entries);
            }
        }
        if let Some((&existing_id, _)) = self
            .properties_windows
            .iter()
            .find(|(_, props)| !props.is_multi() && props.entry.path == path)
        {
            let focus = if existing_id != self.main_window_id {
                window::gain_focus(existing_id)
            } else {
                Task::none()
            };
            let select = self.update(Message::PropertiesSelectTab(existing_id, tab));
            return Task::batch([focus, select]);
        }
        let entry = self.current_tab().and_then(|t| {
            t.visible_entries
                .iter()
                .find(|e| e.path == path)
                .or_else(|| t.raw_entries.iter().find(|e| e.path == path))
                .cloned()
        });
        if let Some(entry) = entry {
            return self.open_properties_window(entry, tab);
        }
        Task::perform(fm_fs::stat_entry(path), move |result| {
            Message::PropertiesEntryStatLoaded { result, tab }
        })
    }

    fn next_properties_req_id(&mut self) -> u64 {
        self.next_properties_request_id = self.next_properties_request_id.wrapping_add(1);
        self.next_properties_request_id
    }

    /// Abre janela flutuante de Propriedades agregada para múltiplos itens selecionados,
    /// somando recursivamente o tamanho total e a quantidade de itens em background.
    fn open_multi_properties_window(&mut self, entries: Vec<FileEntry>) -> Task<Message> {
        let Some(first) = entries.first().cloned() else {
            return Task::none();
        };
        let dir_count = entries.iter().filter(|e| e.kind.is_dir_like()).count();
        let file_count = entries.len().saturating_sub(dir_count);
        let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
        let common_parent = common_parent_dir(&paths).unwrap_or_else(|| {
            first
                .path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| first.path.clone())
        });
        let is_archive = paths
            .iter()
            .any(|p| fm_fs::archive::is_inside_archive_path(p));

        let initial_volume_space = self.current_tab().and_then(|t| {
            (t.current_path == common_parent)
                .then_some(t.telemetry.volume_space.clone())
                .flatten()
        });

        let win_id = self.main_window_id;
        if let Some(mut prev) = self.properties_windows.remove(&win_id)
            && let Some(prev_cancel) = prev.cancel.take()
        {
            prev_cancel.cancel();
        }
        let request_id = self.next_properties_req_id();
        let win_cancel = CancellationToken::new();
        let mut tasks: Vec<Task<Message>> = Vec::new();

        if !is_archive && initial_volume_space.is_none() {
            let vfs = Arc::clone(&self.vfs);
            let vpath = common_parent.clone();
            tasks.push(Task::perform(
                async move { vfs.volume_space(&vpath).await.ok().flatten() },
                move |space| Message::PropertiesVolumeSpaceReady {
                    window_id: win_id,
                    request_id,
                    space,
                },
            ));
        }

        let (recursive_size, size_loading) = if is_archive {
            let total_bytes: u64 = entries
                .iter()
                .filter(|e| !e.kind.is_dir_like())
                .map(|e| e.size_bytes)
                .fold(0u64, u64::saturating_add);
            let hidden_count = entries.iter().filter(|e| e.is_hidden).count() as u64;
            (
                Some(RecursiveSize {
                    total_bytes,
                    item_count: entries.len() as u64,
                    hidden_count,
                    truncated: false,
                }),
                false,
            )
        } else {
            let cancel = win_cancel.clone();
            let calc_paths = paths.clone();
            let meta_cache = self.metadata_cache.clone();
            tasks.push(Task::perform(
                async move {
                    fm_telemetry::compute_paths_size_cached(calc_paths, meta_cache, cancel).await
                },
                move |size| Message::PropertiesFolderSizeReady {
                    window_id: win_id,
                    request_id,
                    size,
                },
            ));
            (None, true)
        };

        self.properties_windows.insert(
            win_id,
            PropertiesWindowState {
                request_id,
                entry: first,
                multi: Some(MultiPropertiesSummary {
                    paths,
                    dir_count,
                    file_count,
                    common_parent,
                }),
                telemetry: None,
                recursive_size,
                volume_space: initial_volume_space,
                size_loading,
                extra: None,
                active_tab: PropertiesTab::General,
                draft_name: String::new(),
                readonly_draft: false,
                initial_readonly: false,
                hidden_draft: false,
                initial_hidden: false,
                posix_mode_draft: None,
                initial_posix_mode: None,
                applying: false,
                open_with_loading: false,
                open_with_catalog: None,
                open_with_selected_id: None,
                open_with_custom_command: String::new(),
                exec_picker: None,
                cancel: Some(win_cancel),
            },
        );
        Task::batch(tasks)
    }

    /// Opens (or focuses) a floating centered Properties window for `entry`.
    fn open_properties_window(
        &mut self,
        entry: FileEntry,
        initial_tab: PropertiesTab,
    ) -> Task<Message> {
        if let Some((&existing_id, _)) = self
            .properties_windows
            .iter()
            .find(|(_, props)| !props.is_multi() && props.entry.path == entry.path)
        {
            let focus = if existing_id != self.main_window_id {
                window::gain_focus(existing_id)
            } else {
                Task::none()
            };
            let select = self.update(Message::PropertiesSelectTab(existing_id, initial_tab));
            return Task::batch([focus, select]);
        }
        let existing_telemetry = self.current_tab().and_then(|t| {
            let selected = t.selection.single_selected(&t.visible_entries)?;
            (selected.path == entry.path)
                .then_some(t.selected_telemetry.as_ref())
                .flatten()
                .cloned()
        });
        let is_archive = fm_fs::archive::is_inside_archive_path(&entry.path);
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

        let win_id = self.main_window_id;
        if let Some(mut prev) = self.properties_windows.remove(&win_id)
            && let Some(prev_cancel) = prev.cancel.take()
        {
            prev_cancel.cancel();
        }
        let request_id = self.next_properties_req_id();
        let win_cancel = CancellationToken::new();
        let mut tasks: Vec<Task<Message>> = Vec::new();

        if !is_archive && initial_volume_space.is_none() {
            let vfs = Arc::clone(&self.vfs);
            let vpath = entry.path.clone();
            tasks.push(Task::perform(
                async move { vfs.volume_space(&vpath).await.ok().flatten() },
                move |space| Message::PropertiesVolumeSpaceReady {
                    window_id: win_id,
                    request_id,
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
                    fm_telemetry::compute_paths_size_cached(vec![size_dir], meta_cache, cancel)
                        .await
                },
                move |size| Message::PropertiesFolderSizeReady {
                    window_id: win_id,
                    request_id,
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

        let can_open_with = !entry.kind.is_dir_like()
            && !is_archive
            && !fm_core::is_virtual_trash_path(&entry.path);
        let active_tab = if initial_tab == PropertiesTab::OpenWith && !can_open_with {
            PropertiesTab::General
        } else {
            initial_tab
        };

        self.properties_windows.insert(
            win_id,
            PropertiesWindowState {
                request_id,
                entry,
                multi: None,
                telemetry,
                recursive_size: None,
                volume_space: initial_volume_space,
                size_loading,
                extra: None,
                active_tab,
                draft_name,
                readonly_draft: initial_readonly,
                initial_readonly,
                hidden_draft: initial_hidden,
                initial_hidden,
                posix_mode_draft: initial_posix_mode,
                initial_posix_mode,
                applying: false,
                open_with_loading: false,
                open_with_catalog: None,
                open_with_selected_id: None,
                open_with_custom_command: String::new(),
                exec_picker: None,
                cancel: Some(win_cancel),
            },
        );
        if active_tab == PropertiesTab::OpenWith {
            tasks.push(self.maybe_load_open_with_catalog(win_id));
        }
        Task::batch(tasks)
    }

    fn resolve_open_with_default_for_path(
        &self,
        path: &std::path::Path,
    ) -> Option<OpenWithAssociation> {
        let telemetry_mime = self
            .current_tab()
            .and_then(|t| {
                let selected = t.selection.single_selected(&t.visible_entries)?;
                (selected.path == path)
                    .then_some(t.selected_telemetry.as_ref())
                    .flatten()
            })
            .and_then(|tel| tel.mime_type.as_deref())
            .filter(|m| !m.starts_with("ext/"));

        let primary_key = fm_fs::open_with_type_key_for_path(path, telemetry_mime);
        if let Some(assoc) = self.config.open_with_defaults.get(&primary_key) {
            return Some(assoc.clone());
        }
        let glob_key = fm_fs::open_with_type_key_for_path(path, None);
        if glob_key != primary_key
            && let Some(assoc) = self.config.open_with_defaults.get(&glob_key)
        {
            return Some(assoc.clone());
        }
        if let Some(ext_key) = extension_fallback_key(path)
            && ext_key != primary_key
            && ext_key != glob_key
            && let Some(assoc) = self.config.open_with_defaults.get(&ext_key)
        {
            return Some(assoc.clone());
        }
        None
    }

    fn open_file_respecting_preference(&self, path: PathBuf) -> Task<Message> {
        let vfs = Arc::clone(&self.vfs);
        if let Some(assoc) = self.resolve_open_with_default_for_path(&path) {
            Task::perform(
                async move { vfs.open_with(&path, &assoc).await },
                Message::OpenDefaultFinished,
            )
        } else {
            Task::perform(
                async move { vfs.open_default(&path).await },
                Message::OpenDefaultFinished,
            )
        }
    }

    fn maybe_load_open_with_catalog(&mut self, window_id: window::Id) -> Task<Message> {
        let Some(props) = self.properties_windows.get_mut(&window_id) else {
            return Task::none();
        };
        if !props.supports_open_with()
            || props.open_with_loading
            || props.open_with_catalog.is_some()
        {
            return Task::none();
        }
        props.open_with_loading = true;
        let path = props.entry.path.clone();
        let mime_hint = props
            .telemetry
            .as_ref()
            .and_then(|t| t.mime_type.as_deref())
            .filter(|m| !m.starts_with("ext/"))
            .map(ToOwned::to_owned);
        let cancel = props.cancel.clone().unwrap_or_else(CancellationToken::new);

        Task::perform(
            async move {
                let result =
                    fm_fs::discover_open_with_catalog(path.clone(), mime_hint, cancel).await;
                (window_id, path, result)
            },
            |(window_id, path, result)| Message::PropertiesOpenWithLoaded {
                window_id,
                path,
                result,
            },
        )
    }

    fn queue_open_with_icons(
        &mut self,
        catalog: &OpenWithCatalog,
        cancel: CancellationToken,
    ) -> Task<Message> {
        const MAX_OTHER_OPEN_WITH_ICONS: usize = 24;
        const MAX_OPEN_WITH_ICONS_BATCH: usize = 48;

        let mut hints: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        let mut push_hint = |opt: Option<&str>| {
            if hints.len() >= MAX_OPEN_WITH_ICONS_BATCH {
                return;
            }
            if let Some(h) = opt.map(str::trim).filter(|s| !s.is_empty())
                && seen.insert(h.to_owned())
            {
                hints.push(h.to_owned());
            }
        };

        if let Some(assoc) = self
            .config
            .open_with_defaults
            .get(catalog.type_key.as_str())
        {
            push_hint(assoc.icon_hint.as_deref());
        }
        if let Some(def) = &catalog.system_default {
            push_hint(def.icon_hint.as_deref());
        }
        for app in &catalog.recommended {
            push_hint(app.icon_hint.as_deref());
        }
        if let Some(customs) = self
            .config
            .open_with_custom_apps
            .get(catalog.type_key.as_str())
        {
            for assoc in customs {
                push_hint(assoc.icon_hint.as_deref());
            }
        }
        for app in catalog.others.iter().take(MAX_OTHER_OPEN_WITH_ICONS) {
            push_hint(app.icon_hint.as_deref());
        }

        self.queue_open_with_icon_hints(hints, cancel)
    }

    fn queue_open_with_icon_hints(
        &mut self,
        hints: Vec<String>,
        cancel: CancellationToken,
    ) -> Task<Message> {
        let mut to_load = Vec::new();
        for raw in hints {
            let trimmed = raw.trim();
            if trimmed.is_empty()
                || self.open_with_icons.contains_key(trimmed)
                || !self.open_with_icons_in_flight.insert(trimmed.to_owned())
            {
                continue;
            }
            to_load.push(trimmed.to_owned());
        }
        if to_load.is_empty() {
            return Task::none();
        }

        // Um único worker sequencial evita dezenas de threads bloqueantes e segmentos extras no mimalloc.
        let sent_keys = to_load.clone();
        Task::perform(
            tokio::task::spawn_blocking(move || {
                let mut out = Vec::with_capacity(to_load.len());
                for hint in to_load {
                    if cancel.is_cancelled() {
                        out.push((hint, None));
                        continue;
                    }
                    let rgba = fm_fs::resolve_app_icon(&hint, SystemIconBucket::Small24).and_then(
                        |payload| {
                            rasterize_system_icon(payload, SystemIconBucket::Small24, &cancel)
                        },
                    );
                    out.push((hint, rgba));
                }
                out
            }),
            move |res| match res {
                Ok(items) => Message::OpenWithIconsBatchLoaded(
                    items
                        .into_iter()
                        .map(|(hint, rgba)| (hint, rgba.map(rgba_arc_to_handle)))
                        .collect(),
                ),
                Err(_) => Message::OpenWithIconsBatchLoaded(
                    sent_keys.into_iter().map(|h| (h, None)).collect(),
                ),
            },
        )
    }

    fn resolve_open_with_selection(
        &self,
        props: &PropertiesWindowState,
    ) -> Option<(String, OpenWithAssociation)> {
        let type_key = open_with_props_type_key(props);
        if let Some(sel_id) = props.open_with_selected_id.as_deref() {
            if let Some(def) = self.config.open_with_defaults.get(&type_key)
                && def.app_id == sel_id
            {
                return Some((type_key, def.clone()));
            }
            if let Some(customs) = self.config.open_with_custom_apps.get(&type_key)
                && let Some(found) = customs.iter().find(|a| a.app_id == sel_id)
            {
                return Some((type_key, found.clone()));
            }
            if let Some(cat) = &props.open_with_catalog
                && let Some(app) = cat
                    .system_default
                    .iter()
                    .chain(cat.recommended.iter())
                    .chain(cat.others.iter())
                    .find(|a| a.id.as_str() == sel_id)
            {
                return Some((type_key, app.to_association()));
            }
        }
        let custom_cmd = props.open_with_custom_command.trim();
        custom_association_from_command(custom_cmd).map(|assoc| (type_key, assoc))
    }

    fn load_exec_picker_dir(&self, window_id: window::Id, dir: PathBuf) -> Task<Message> {
        let vfs = Arc::clone(&self.vfs);
        Task::perform(
            async move {
                let result = vfs.list_dir(&dir, 0, CancellationToken::new()).await;
                (window_id, dir, result)
            },
            |(window_id, dir, result)| Message::OpenWithExecPickerLoaded {
                window_id,
                dir,
                result,
            },
        )
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
        let active = self.active_tab_id();
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
        self.status_notice = None;
        let is_right = self
            .split_view
            .as_ref()
            .is_some_and(|s| s.right_tab.id == tab_id);
        let scroll_id = if is_right {
            if let Some(split) = &mut self.split_view {
                split.right_scroll_y = 0.0;
            }
            secondary_canvas_scroll_id()
        } else {
            self.canvas_scroll_y = 0.0;
            canvas_scroll_id()
        };
        let cancel = self.reset_tab_cancel(tab_id);
        Task::batch([
            spawn_list_task(Arc::clone(&self.vfs), tab_id, path, request_id, cancel),
            operation::snap_to(scroll_id, RelativeOffset::START),
        ])
    }

    fn dispatch_selection_side_effects(&mut self, entry: Option<&FileEntry>) -> Task<Message> {
        let tel_task = self.maybe_load_selection_telemetry(entry);
        let preview_task = if self.quicklook_open || self.config.show_inspector {
            self.maybe_load_preview(entry)
        } else {
            Task::none()
        };
        let thumb_task = self.queue_visible_thumbnails();
        Task::batch([tel_task, preview_task, thumb_task])
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
                let from_undo = src.clone();
                let to_undo = dst.clone();
                Task::perform(async move { vfs.rename(&src, &dst).await }, move |result| {
                    Message::RenameOpFinished {
                        from: from_undo,
                        to: to_undo,
                        result,
                    }
                })
            }
            ActiveDialog::ConfirmDelete { paths, permanent } => {
                if permanent {
                    Task::perform(
                        async move { vfs.delete_permanent(&paths).await },
                        |result| Message::FileOpFinished {
                            result,
                            select_after: None,
                        },
                    )
                } else {
                    let trashed_paths = paths.clone();
                    Task::perform(
                        async move { vfs.move_to_trash(&paths).await },
                        move |result| Message::MoveToTrashFinished {
                            paths: trashed_paths,
                            result,
                        },
                    )
                }
            }
            ActiveDialog::ConfirmEmptyTrash => {
                let notice = self.strings().trash_emptied.to_owned();
                Task::perform(async move { vfs.empty_trash().await }, move |result| {
                    Message::TrashActionFinished {
                        notice: notice.clone(),
                        clear_trash_undo: true,
                        result,
                    }
                })
            }
            ActiveDialog::Conflict { .. } => Task::none(),
        }
    }

    fn start_clipboard_paste(&mut self) -> Task<Message> {
        let Some(transfer) = self.clipboard.clone() else {
            return clipboard::read().map(Message::SystemClipboardPasteRead);
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
        let Some(dest_dir) = self.current_tab().map(|t| t.current_path.clone()) else {
            return Task::none();
        };
        self.start_transfer_with_conflict_check_to(mode, sources, dest_dir)
    }

    fn start_transfer_with_conflict_check_to(
        &mut self,
        mode: CopyMode,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
    ) -> Task<Message> {
        if sources.is_empty() {
            return Task::none();
        }

        let dest_entries: Option<&[FileEntry]> = self
            .tabs
            .iter()
            .chain(self.split_view.iter().map(|s| &s.right_tab))
            .find(|t| t.current_path == dest_dir)
            .map(|t| t.raw_entries.as_slice());

        let conflicts = if let Some(entries) = dest_entries {
            sources
                .iter()
                .filter(|src| {
                    src.parent() != Some(&dest_dir)
                        && src.file_name().is_some_and(|n| {
                            let target_name = n.to_string_lossy();
                            entries
                                .iter()
                                .any(|e| e.name.as_str() == target_name.as_ref())
                        })
                })
                .count()
        } else {
            0
        };

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

    pub(crate) fn current_drag_mode(&self) -> CopyMode {
        if self.current_modifiers.control() {
            CopyMode::Copy
        } else {
            CopyMode::Move
        }
    }

    pub(crate) fn current_drag_destination(&self) -> Option<PathBuf> {
        let drag = self.entry_drag.as_ref()?;
        let left_dir = self.left_tab().map(|t| t.current_path.as_path());
        let right_dir = self
            .split_view
            .as_ref()
            .map(|s| s.right_tab.current_path.as_path());
        resolve_internal_drop_destination(
            drag,
            self.drag_hover_folder.as_ref(),
            self.drag_hover_pane,
            left_dir,
            right_dir,
            self.current_drag_mode(),
        )
    }

    fn finish_entry_drag(&mut self) -> Task<Message> {
        let dest = self.current_drag_destination();
        let mode = self.current_drag_mode();
        let drag = self.entry_drag.take();
        self.drag_hover_pane = None;
        self.drag_hover_folder = None;
        self.suppress_click_until = Some(Instant::now() + Duration::from_millis(80));

        let (Some(drag), Some(dest_dir)) = (drag, dest) else {
            return Task::none();
        };
        if drag.paths.is_empty() {
            return Task::none();
        }
        self.start_transfer_with_conflict_check_to(mode, drag.paths, dest_dir)
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

    fn maybe_load_selection_telemetry(&mut self, entry: Option<&FileEntry>) -> Task<Message> {
        if let Some(prev) = self.selection_telemetry_cancel.take() {
            prev.cancel();
        }
        let Some(entry) = entry else {
            return Task::none();
        };
        if entry.kind.is_dir_like() || fm_fs::archive::is_inside_archive_path(&entry.path) {
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
        let is_in_archive = fm_fs::archive::is_inside_archive_path(&path);

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
        let Some((root, include_hidden)) = self.current_tab().map(|t| {
            (
                t.current_path.clone(),
                t.show_hidden || query.starts_with('.'),
            )
        }) else {
            return Task::none();
        };

        self.deep_search_request_id += 1;
        let req_id = self.deep_search_request_id;
        let cancel = CancellationToken::new();
        self.deep_search_cancel = Some(cancel.clone());
        self.deep_search_loading = true;

        Task::perform(
            async move {
                let results =
                    fm_index::deep_search_with_hidden(&root, &query, 100, include_hidden, cancel)
                        .await;
                (req_id, results)
            },
            |(request_id, results)| Message::DeepSearchResults {
                request_id,
                results,
            },
        )
    }

    fn queue_visible_thumbnails(&mut self) -> Task<Message> {
        let mut candidates: Vec<(ThumbnailKey, PathBuf, TabId)> = Vec::new();
        let panes: &[SplitPane] = if self.split_view.is_some() {
            &[SplitPane::Left, SplitPane::Right]
        } else {
            &[SplitPane::Left]
        };

        for &pane in panes {
            let Some(tab) = self.pane_tab(pane) else {
                continue;
            };
            let (scroll_y, viewport_h, _) = self.pane_scroll_metrics(pane);
            let tab_id = tab.id;

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
                    let min_row = ((scroll_y - 32.0).max(0.0) / step).floor() as usize;
                    let max_row = ((scroll_y + viewport_h + 280.0) / step).ceil() as usize;

                    let start_idx = min_row.saturating_sub(dirs_count.div_ceil(2) + 2) * 2;
                    let end_idx =
                        (max_row.saturating_sub(dirs_count / 8) * 8 + 16).min(files.len());
                    let clamped_start = start_idx.min(end_idx);

                    for entry in &files[clamped_start..end_idx] {
                        if fm_preview::is_image_extension(entry.extension.as_deref()) {
                            candidates.push((
                                thumbnail_key_for_entry(entry),
                                entry.path.clone(),
                                tab_id,
                            ));
                        }
                    }
                }
            }

            if let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                && !selected.kind.is_dir_like()
                && fm_preview::is_image_extension(selected.extension.as_deref())
            {
                candidates.push((
                    thumbnail_key_for_entry(selected),
                    selected.path.clone(),
                    tab_id,
                ));
            }
        }

        let mut tasks = Vec::new();
        for (key, path, tab_id) in candidates {
            if tasks.len() >= 24 {
                break;
            }
            if self.gpu_thumbnails.contains_key(&key) {
                continue;
            }
            if self.in_flight_thumbs.contains(&path) {
                continue;
            }
            self.in_flight_thumbs.insert(path.clone());
            let cancel = self.tab_cancels.get(&tab_id).cloned().unwrap_or_default();
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

        let panes: &[SplitPane] = if self.split_view.is_some() {
            &[SplitPane::Left, SplitPane::Right]
        } else {
            &[SplitPane::Left]
        };
        let strings = self.strings();
        for &pane in panes {
            let Some(tab) = self.pane_tab(pane) else {
                continue;
            };
            let (scroll_y, viewport_h, _) = self.pane_scroll_metrics(pane);
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
                        self.effective_pane_width(pane),
                        card_w,
                    );
                    let bands = views::virtualized::build_tab_grid_bands(
                        &tab.visible_entries,
                        fit_cols,
                        self.config.grid_group_folders,
                        strings,
                    );
                    let win = views::virtualized::compute_band_window_with_row_height(
                        &bands, card_h, scroll_y, viewport_h, 280.0,
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
                ViewMode::DetailTable => {
                    let win = views::virtualized::compute_fixed_window(
                        tab.visible_entries.len(),
                        views::virtualized::DETAIL_ROW_STEP,
                        scroll_y,
                        viewport_h,
                        6,
                    );
                    for entry in &tab.visible_entries[win.range] {
                        push_icon(
                            SystemIconKey::for_entry(entry, fm_fs::is_archive_file(&entry.path)),
                            entry_bucket,
                        );
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
            .get(&self.active_tab_id())
            .cloned()
            .unwrap_or_default();

        let mut tasks = Vec::new();
        for (key, bucket) in needed {
            let pair = (key.clone(), bucket);
            if self.gpu_system_icons.contains_key(&pair) || self.system_icon_misses.contains(&pair)
            {
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

    fn clear_active_canvas_filters(&mut self) -> Task<Message> {
        self.filter_ext_draft.clear();
        let changed = self.current_tab_mut().is_some_and(clear_tab_filters);
        if changed {
            self.canvas_scroll_y = 0.0;
            self.queue_visible_thumbnails()
        } else {
            Task::none()
        }
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
        let compress_modal_visible = self.compress_modal.as_ref().is_some_and(|m| !m.minimized);
        let dup_modal_visible = self.duplicate_finder.as_ref().is_some_and(|m| !m.minimized);
        if is_ctrl_q {
            if self.bulk_rename_modal.is_some() {
                return self.update(Message::BulkRename(BulkRenameMsg::Close));
            }
            if self.archive_password_modal.is_some() {
                return self.update(Message::Archive(ArchiveUiMsg::ClosePasswordModal));
            }
            if compress_modal_visible {
                return self.update(Message::Archive(ArchiveUiMsg::CloseModal));
            }
            if dup_modal_visible {
                return self.update(Message::DuplicateFinder(DuplicateFinderMsg::Close));
            }
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

        if self.text_input_menu.is_some()
            && matches!(key.as_ref(), keyboard::Key::Named(Named::Escape))
        {
            self.text_input_menu = None;
            return Task::none();
        }

        if self.properties_windows.contains_key(&window_id) {
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Escape)) {
                return self.update(Message::ClosePropertiesWindow(window_id));
            }
            return Task::none();
        }
        let filter_menu_open = matches!(
            self.context_menu.as_ref().map(|m| &m.kind),
            Some(ContextMenuKind::FilterMenu)
        );
        let text_input_modal_active = self.bulk_rename_modal.is_some()
            || self.archive_password_modal.is_some()
            || compress_modal_visible
            || dup_modal_visible
            || self.active_dialog.is_some()
            || self.deep_search_open
            || self.path_editing
            || self.prefs_open
            || filter_menu_open;
        if text_input_modal_active {
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Tab)) && self.path_editing {
                return self.autocomplete_path_draft();
            }
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Enter))
                && matches!(
                    self.active_dialog,
                    Some(ActiveDialog::ConfirmDelete { .. } | ActiveDialog::ConfirmEmptyTrash)
                )
            {
                return self.update(Message::SubmitDialog);
            }
            if matches!(key.as_ref(), keyboard::Key::Named(Named::Escape)) {
                if self.bulk_rename_modal.is_some() {
                    return self.update(Message::BulkRename(BulkRenameMsg::Close));
                } else if self.archive_password_modal.is_some() {
                    return self.update(Message::Archive(ArchiveUiMsg::ClosePasswordModal));
                } else if compress_modal_visible {
                    return self.update(Message::Archive(ArchiveUiMsg::CloseModal));
                } else if dup_modal_visible {
                    return self.update(Message::DuplicateFinder(DuplicateFinderMsg::Close));
                } else if self.active_dialog.is_some() {
                    return self.update(Message::CloseDialog);
                } else if self.prefs_open {
                    return self.update(Message::ClosePreferences);
                } else if self.deep_search_open {
                    return self.update(Message::CloseDeepSearch);
                } else if filter_menu_open {
                    return self.update(Message::CloseContextMenu);
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
            keyboard::Key::Named(Named::Tab)
                if !modifiers.control() && !modifiers.shift() && !modifiers.alt() =>
            {
                if self.split_view.is_some() {
                    let next = match self.active_pane() {
                        SplitPane::Left => SplitPane::Right,
                        SplitPane::Right => SplitPane::Left,
                    };
                    self.update(Message::FocusSplitPane(next))
                } else {
                    Task::none()
                }
            }
            keyboard::Key::Named(Named::F3) => self.update(Message::ToggleSplitView),
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
            keyboard::Key::Character("z")
                if modifiers.control() && !modifiers.shift() && !modifiers.alt() =>
            {
                self.update(Message::UndoLastAction)
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
                if self.entry_drag.is_some() {
                    self.entry_drag = None;
                    self.drag_hover_pane = None;
                    self.drag_hover_folder = None;
                    Task::none()
                } else if self.context_menu.is_some() {
                    self.update(Message::CloseContextMenu)
                } else if self.breadcrumb_menu.is_some() {
                    self.update(Message::CloseBreadcrumbMenu)
                } else if self.quicklook_open {
                    self.update(Message::CloseQuickLook)
                } else {
                    self.clear_active_canvas_filters()
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
                let (_, vh, _) = self.pane_scroll_metrics(self.active_pane());
                let page = (vh * 0.85).max(200.0);
                operation::scroll_by(
                    self.active_canvas_scroll_id(),
                    AbsoluteOffset { x: 0.0, y: page },
                )
            }
            keyboard::Key::Named(Named::PageUp) if !self.quicklook_open => {
                let (_, vh, _) = self.pane_scroll_metrics(self.active_pane());
                let page = (vh * 0.85).max(200.0);
                operation::scroll_by(
                    self.active_canvas_scroll_id(),
                    AbsoluteOffset { x: 0.0, y: -page },
                )
            }
            keyboard::Key::Named(Named::ArrowDown) => {
                self.navigate_selection_arrow(0, 1, modifiers)
            }
            keyboard::Key::Named(Named::ArrowUp) => self.navigate_selection_arrow(0, -1, modifiers),
            keyboard::Key::Named(Named::ArrowRight) => {
                self.navigate_selection_arrow(1, 0, modifiers)
            }
            keyboard::Key::Named(Named::ArrowLeft) => {
                self.navigate_selection_arrow(-1, 0, modifiers)
            }
            keyboard::Key::Named(Named::Home) if !modifiers.alt() => {
                self.navigate_selection_edge(false, modifiers)
            }
            keyboard::Key::Named(Named::End) if !modifiers.alt() => {
                self.navigate_selection_edge(true, modifiers)
            }
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
        self.open_tabs_at(vec![path])
    }

    fn open_tabs_at(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        let mut seen = HashSet::new();
        let mut unique_paths = Vec::with_capacity(paths.len());
        for path in paths {
            if seen.insert(path.clone()) {
                unique_paths.push(path);
            }
        }
        if unique_paths.is_empty() {
            return Task::none();
        }

        let mut first_new_id = None;
        let mut tasks = Vec::with_capacity(unique_paths.len() + 1);
        for path in unique_paths {
            let id = TabId(self.next_tab_id);
            self.next_tab_id += 1;
            if first_new_id.is_none() {
                first_new_id = Some(id);
            }
            let req = self.alloc_request_id();
            let mut tab = TabState::new(id, path.clone());
            tab.view_mode = self.config.default_view_mode;
            tab.sort.dirs_first = self.config.dirs_first;
            tab.active_request_id = req;
            tab.is_loading = true;
            self.tabs.push(tab);
            let cancel = self.reset_tab_cancel(id);
            tasks.push(spawn_list_task(
                Arc::clone(&self.vfs),
                id,
                path,
                req,
                cancel,
            ));
        }

        if let Some(id) = first_new_id {
            self.active_tab = id;
            self.focus_split_pane(SplitPane::Left);
        }
        self.path_editing = false;
        self.quicklook_open = false;
        tasks.push(operation::snap_to(
            canvas_scroll_id(),
            RelativeOffset::START,
        ));
        Task::batch(tasks)
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

    fn effective_pane_width(&self, pane: SplitPane) -> f32 {
        let (_, _, measured) = self.pane_scroll_metrics(pane);
        if let Some(measured) = measured {
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
        let total = (self.window_size.width - sidebar - inspector - 40.0).max(200.0);
        if let Some(split) = &self.split_view {
            let frac = match pane {
                SplitPane::Left => split.ratio,
                SplitPane::Right => 1.0 - split.ratio,
            };
            ((total - 10.0) * frac).max(200.0)
        } else {
            total
        }
    }

    fn effective_canvas_width(&self) -> f32 {
        self.effective_pane_width(self.active_pane())
    }

    fn current_tab_visual_order(&self) -> Vec<usize> {
        let Some(tab) = self.current_tab() else {
            return Vec::new();
        };
        if tab.view_mode != ViewMode::Grid {
            return Vec::new();
        }
        let card_w = views::virtualized::clamp_grid_card_width(self.config.grid_card_width);
        let cols =
            views::virtualized::grid_columns_for_width(self.effective_canvas_width(), card_w);
        let bands = views::virtualized::build_tab_grid_bands(
            &tab.visible_entries,
            cols,
            self.config.grid_group_folders,
            self.strings(),
        );
        views::virtualized::grid_visual_order(&bands)
    }

    fn navigate_selection_arrow(
        &mut self,
        dx: isize,
        dy: isize,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        let Some(tab) = self.current_tab() else {
            return Task::none();
        };
        if tab.visible_entries.is_empty() {
            return Task::none();
        }

        let view_mode = tab.view_mode;
        let focused = tab.selection.focused;

        let (next_idx, visual_order) = if view_mode == ViewMode::Grid {
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
            let next = views::virtualized::step_grid_selection(&bands, focused, dx, dy);
            let order = if modifiers.shift() {
                views::virtualized::grid_visual_order(&bands)
            } else {
                Vec::new()
            };
            (next, order)
        } else {
            let delta = if dy != 0 { dy } else { dx };
            let len = tab.visible_entries.len();
            let next = match focused {
                Some(cur) => (cur as isize + delta).clamp(0, len as isize - 1) as usize,
                None if delta < 0 => len - 1,
                None => 0,
            };
            (Some(next), Vec::new())
        };

        let Some(next_idx) = next_idx else {
            return Task::none();
        };

        self.apply_keyboard_selection(next_idx, &visual_order, modifiers)
    }

    fn navigate_selection_edge(
        &mut self,
        to_end: bool,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        let Some(tab) = self.current_tab() else {
            return Task::none();
        };
        if tab.visible_entries.is_empty() {
            return Task::none();
        }

        let visual_order = self.current_tab_visual_order();
        let target_idx = if !visual_order.is_empty() {
            if to_end {
                visual_order.last().copied()
            } else {
                visual_order.first().copied()
            }
        } else if to_end {
            Some(tab.visible_entries.len() - 1)
        } else {
            Some(0)
        };

        let Some(target_idx) = target_idx else {
            return Task::none();
        };
        self.apply_keyboard_selection(target_idx, &visual_order, modifiers)
    }

    fn apply_keyboard_selection(
        &mut self,
        next_idx: usize,
        visual_order: &[usize],
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        let mut selected_entry = None;
        if let Some(tab_mut) = self.current_tab_mut() {
            if modifiers.shift() {
                tab_mut.selection.select_range_in_order(
                    next_idx,
                    visual_order,
                    modifiers.control(),
                    &tab_mut.visible_entries,
                );
            } else {
                tab_mut
                    .selection
                    .select_single(next_idx, &tab_mut.visible_entries);
            }
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
        };

        let (scroll_y, viewport_h, _) = self.pane_scroll_metrics(self.active_pane());
        let vh = viewport_h.max(120.0);
        let scroll_id = self.active_canvas_scroll_id();
        if top_y < scroll_y {
            operation::scroll_to(
                scroll_id,
                AbsoluteOffset {
                    x: None,
                    y: Some(top_y.max(0.0)),
                },
            )
        } else if bottom_y > scroll_y + vh {
            operation::scroll_to(
                scroll_id,
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
fn watch_drives_stream() -> impl iced::futures::Stream<Item = Message> {
    stream::channel(8, async move |mut output| {
        let mut last = tokio::task::spawn_blocking(fm_fs::drives_fingerprint)
            .await
            .unwrap_or(0);
        loop {
            tokio::time::sleep(Duration::from_millis(1_500)).await;
            let current = tokio::task::spawn_blocking(fm_fs::drives_fingerprint)
                .await
                .unwrap_or(last);
            if current != last {
                last = current;
                if output.send(Message::RefreshDrives).await.is_err() {
                    break;
                }
            }
        }
    })
}

#[allow(clippy::ptr_arg)]
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
                Some(filter) => fm_core::matches_extension_filter(entry, filter),
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

fn push_undo_entry(stack: &mut Vec<UndoAction>, action: UndoAction) {
    stack.push(action);
    if stack.len() > MAX_UNDO_HISTORY {
        let excess = stack.len() - MAX_UNDO_HISTORY;
        stack.drain(0..excess);
    }
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

pub fn effective_properties_modal_size(app: &App) -> (f32, f32) {
    clamp_properties_modal_size(
        app.properties_modal_size.0,
        app.properties_modal_size.1,
        app.window_size,
    )
}

pub fn effective_properties_modal_rect(app: &App) -> (f32, f32, f32, f32) {
    crate::bulk_rename::effective_modal_rect(
        app.properties_modal_size,
        app.properties_modal_offset,
        560.0,
        420.0,
        app.window_size,
    )
}

fn clamp_properties_modal_pos(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    window_size: Size,
) -> (f32, f32) {
    crate::bulk_rename::clamp_modal_pos(x, y, width, height, window_size)
}

fn clamp_properties_modal_size(width: f32, height: f32, window_size: Size) -> (f32, f32) {
    crate::bulk_rename::clamp_modal_size(width, height, 560.0, 420.0, window_size)
}

fn compute_dragged_properties_modal_size(
    anchor_x: f32,
    anchor_y: f32,
    initial_w: f32,
    initial_h: f32,
    start: Point,
    current: Point,
    window_size: Size,
) -> (f32, f32) {
    crate::bulk_rename::compute_dragged_modal_size(
        (anchor_x, anchor_y),
        (initial_w, initial_h),
        (560.0, 420.0),
        start,
        current,
        window_size,
    )
}

fn rgba_arc_to_handle(thumb: Arc<ThumbnailRgba>) -> image::Handle {
    let w = thumb.width;
    let h = thumb.height;
    let pixels = match Arc::try_unwrap(thumb) {
        Ok(owned) => owned.pixels,
        Err(shared) => shared.pixels.clone(),
    };
    image::Handle::from_rgba(w, h, pixels)
}

pub(crate) fn open_with_props_type_key(props: &PropertiesWindowState) -> String {
    if let Some(cat) = &props.open_with_catalog {
        return cat.type_key.to_string();
    }
    let mime_hint = props
        .telemetry
        .as_ref()
        .and_then(|t| t.mime_type.as_deref())
        .filter(|m| !m.starts_with("ext/"));
    fm_fs::open_with_type_key_for_path(&props.entry.path, mime_hint)
}

fn extension_fallback_key(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
}

fn custom_association_from_command(raw_cmd: &str) -> Option<OpenWithAssociation> {
    let trimmed = raw_cmd.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first_tok = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or(trimmed)
    } else if let Some(rest) = trimmed.strip_prefix('\'') {
        rest.split('\'').next().unwrap_or(trimmed)
    } else {
        trimmed.split_whitespace().next().unwrap_or(trimmed)
    };
    let display_name = std::path::Path::new(first_tok)
        .file_stem()
        .or_else(|| std::path::Path::new(first_tok).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| first_tok.to_owned());

    Some(OpenWithAssociation {
        app_id: format!("custom:{trimmed}"),
        display_name,
        exec: trimmed.to_owned(),
        icon_hint: (!first_tok.is_empty()).then(|| first_tok.to_owned()),
        runs_in_terminal: false,
    })
}

fn default_exec_picker_dir(places: &UserPlaces) -> PathBuf {
    for candidate in ["/usr/bin", r"C:\Program Files"] {
        let p = PathBuf::from(candidate);
        if p.is_dir() {
            return p;
        }
    }
    places.home.clone()
}

fn quote_exec_path_if_needed(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    if s.chars().any(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.into_owned()
    }
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

fn parse_clipboard_transfer_text(raw: &str) -> Option<(CopyMode, Vec<PathBuf>)> {
    let mut lines = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .peekable();

    let mode = match lines.peek().copied() {
        Some("copy") => {
            lines.next();
            CopyMode::Copy
        }
        Some("cut") => {
            lines.next();
            CopyMode::Move
        }
        _ => CopyMode::Copy,
    };

    let mut paths = Vec::new();
    for line in lines {
        let candidate = if let Some(rest) = line
            .strip_prefix("file://localhost/")
            .map(|r| format!("/{r}"))
            .or_else(|| line.strip_prefix("file://").map(str::to_owned))
        {
            let decoded = percent_decode_uri_path(&rest);
            #[cfg(target_os = "windows")]
            let normalized = decoded.strip_prefix('/').unwrap_or(&decoded).to_owned();
            #[cfg(not(target_os = "windows"))]
            let normalized = decoded;
            PathBuf::from(normalized)
        } else {
            let p = PathBuf::from(line);
            if !p.is_absolute() {
                continue;
            }
            p
        };
        if !candidate.as_os_str().is_empty() {
            paths.push(candidate);
        }
    }

    (!paths.is_empty()).then_some((mode, paths))
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

/// Retorna o diretório ancestral comum mais profundo entre os pais de `paths`.
fn common_parent_dir(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut parents = paths.iter().filter_map(|p| p.parent());
    let first = parents.next()?;
    let mut prefix: Vec<std::path::Component<'_>> = first.components().collect();
    for parent in parents {
        let mut matched = 0usize;
        for (a, b) in prefix.iter().zip(parent.components()) {
            if *a == b {
                matched += 1;
            } else {
                break;
            }
        }
        prefix.truncate(matched);
        if prefix.is_empty() {
            return None;
        }
    }
    let out: PathBuf = prefix.into_iter().collect();
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Limpa filtro de texto e filtro de extensão da aba e restaura a listagem normal.
fn clear_tab_filters(tab: &mut TabState) -> bool {
    if tab.filter_query.is_empty() && tab.extension_filter.is_none() {
        return false;
    }
    tab.filter_query.clear();
    tab.extension_filter = None;
    apply_tab_filter(tab);
    true
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

pub(crate) fn resolve_open_in_new_tab_targets(
    clicked_path: &std::path::Path,
    clicked_is_dir: bool,
    visible_entries: &[FileEntry],
    selected_ids: &HashSet<EntryId>,
) -> Vec<PathBuf> {
    let clicked_is_selected = visible_entries
        .iter()
        .any(|e| e.path == clicked_path && selected_ids.contains(&e.id));

    if clicked_is_selected {
        let selected_dirs: Vec<PathBuf> = visible_entries
            .iter()
            .filter(|e| selected_ids.contains(&e.id) && e.kind.is_dir_like())
            .map(|e| e.path.clone())
            .collect();
        if !selected_dirs.is_empty() {
            return selected_dirs;
        }
    }

    if clicked_is_dir {
        vec![clicked_path.to_path_buf()]
    } else {
        Vec::new()
    }
}

fn find_tab_mut_parts<'a>(
    tabs: &'a mut [TabState],
    split_view: &'a mut Option<SplitViewState>,
    id: TabId,
) -> Option<&'a mut TabState> {
    if let Some(split) = split_view
        && split.right_tab.id == id
    {
        return Some(&mut split.right_tab);
    }
    tabs.iter_mut().find(|t| t.id == id)
}

pub(crate) fn compute_split_ratio(
    initial_ratio: f32,
    start_x: f32,
    pos_x: f32,
    available_width: f32,
) -> f32 {
    if available_width <= 1.0 {
        return initial_ratio.clamp(0.25, 0.75);
    }
    let delta = (pos_x - start_x) / available_width;
    (initial_ratio + delta).clamp(0.25, 0.75)
}

pub(crate) fn collect_drag_paths(
    clicked_index: usize,
    visible_entries: &[FileEntry],
    selected_ids: &HashSet<EntryId>,
) -> Option<(Vec<PathBuf>, String)> {
    let clicked = visible_entries.get(clicked_index)?;
    let paths: Vec<PathBuf> = if selected_ids.contains(&clicked.id) {
        visible_entries
            .iter()
            .filter(|e| {
                selected_ids.contains(&e.id)
                    && !fm_core::is_virtual_trash_path(&e.path)
                    && !fm_fs::archive::is_inside_archive_path(&e.path)
            })
            .map(|e| e.path.clone())
            .collect()
    } else {
        vec![clicked.path.clone()]
    };
    if paths.is_empty() {
        return None;
    }
    Some((paths, clicked.name.to_string()))
}

pub(crate) fn resolve_internal_drop_destination(
    drag: &InternalEntryDrag,
    hover_folder: Option<&(SplitPane, PathBuf)>,
    hover_pane: Option<SplitPane>,
    left_dir: Option<&std::path::Path>,
    right_dir: Option<&std::path::Path>,
    mode: CopyMode,
) -> Option<PathBuf> {
    let dest = if let Some((_, folder_path)) = hover_folder {
        folder_path.clone()
    } else {
        match hover_pane? {
            SplitPane::Left => left_dir?.to_path_buf(),
            SplitPane::Right => right_dir?.to_path_buf(),
        }
    };

    if fm_core::is_virtual_trash_path(&dest) || fm_fs::archive::is_inside_archive_path(&dest) {
        return None;
    }

    // Impede soltar uma pasta dentro de si mesma ou de um subdiretório descendente.
    if drag
        .paths
        .iter()
        .any(|src| dest == *src || dest.starts_with(src))
    {
        return None;
    }

    // Mover para a exata pasta onde todos os itens já estão é no-op; Copiar (Ctrl) é permitido.
    if mode == CopyMode::Move
        && drag
            .paths
            .iter()
            .all(|src| src.parent() == Some(dest.as_path()))
    {
        return None;
    }

    Some(dest)
}

fn text_input_id_for_target(target: &TextInputTarget) -> Id {
    match target {
        TextInputTarget::DialogInput => dialog_input_id(),
        TextInputTarget::PropertiesName(win_id) => properties_name_input_id(*win_id),
        TextInputTarget::SearchBar => search_input_id(),
        TextInputTarget::DeepSearchInput => deep_search_input_id(),
        TextInputTarget::PathDraft => path_input_id(),
    }
}

pub(crate) fn extract_text_input_copy(
    text: &str,
    selection: Option<(usize, usize)>,
) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }
    if let Some((start, end)) = selection {
        let s = start.min(chars.len());
        let e = end.min(chars.len());
        if s < e {
            return Some(chars[s..e].iter().collect());
        }
    }
    Some(text.to_owned())
}

pub(crate) fn apply_text_input_cut(
    text: &str,
    selection: Option<(usize, usize)>,
) -> Option<(String, String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let (start, end) = match selection {
        Some((s, e)) if s < e => (s.min(chars.len()), e.min(chars.len())),
        _ => (0, chars.len()),
    };
    if start >= end {
        return None;
    }
    let cut: String = chars[start..end].iter().collect();
    let mut remaining = String::with_capacity(text.len().saturating_sub(cut.len()));
    for &c in &chars[..start] {
        remaining.push(c);
    }
    for &c in &chars[end..] {
        remaining.push(c);
    }
    Some((cut, remaining, start))
}

pub(crate) fn apply_text_input_paste(
    text: &str,
    selection: Option<(usize, usize)>,
    cursor_index: usize,
    pasted: &str,
) -> (String, usize) {
    let sanitized: String = pasted.chars().filter(|&c| c != '\r' && c != '\n').collect();
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = match selection {
        Some((s, e)) if s < e => (s.min(chars.len()), e.min(chars.len())),
        _ => {
            let pos = cursor_index.min(chars.len());
            (pos, pos)
        }
    };
    let pasted_len = sanitized.chars().count();
    let mut out = String::with_capacity(text.len() + sanitized.len());
    for &c in &chars[..start] {
        out.push(c);
    }
    out.push_str(&sanitized);
    for &c in &chars[end..] {
        out.push(c);
    }
    (out, start + pasted_len)
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

    #[test]
    fn custom_open_with_command_builds_association_and_quotes_paths() {
        let assoc = super::custom_association_from_command(r#""/opt/My App/player" --fs"#)
            .expect("valid custom command");
        assert_eq!(assoc.display_name, "player");
        assert_eq!(assoc.icon_hint.as_deref(), Some("/opt/My App/player"));
        assert_eq!(
            super::quote_exec_path_if_needed(std::path::Path::new("/usr/bin/mpv")),
            "/usr/bin/mpv"
        );
        assert_eq!(
            super::quote_exec_path_if_needed(std::path::Path::new("/opt/My App/bin")),
            "\"/opt/My App/bin\""
        );
    }

    #[test]
    fn properties_modal_resize_and_drag_clamp_within_viewport() {
        let win = Size::new(1000.0, 700.0);
        assert_eq!(
            super::compute_dragged_properties_modal_size(
                120.0,
                70.0,
                760.0,
                560.0,
                iced::Point::new(880.0, 630.0),
                iced::Point::new(920.0, 650.0),
                win,
            ),
            (800.0, 580.0)
        );
        assert_eq!(
            super::compute_dragged_properties_modal_size(
                120.0,
                70.0,
                760.0,
                560.0,
                iced::Point::new(880.0, 630.0),
                iced::Point::new(500.0, 300.0),
                win,
            ),
            (560.0, 420.0)
        );
        assert_eq!(
            super::clamp_properties_modal_pos(-20.0, 500.0, 760.0, 560.0, win),
            (8.0, 132.0)
        );
    }

    #[test]
    fn undo_stack_caps_history_and_preserves_lifo_order() {
        let mut stack = Vec::new();
        for idx in 0..(super::MAX_UNDO_HISTORY + 5) {
            super::push_undo_entry(
                &mut stack,
                super::UndoAction::Trash {
                    paths: vec![std::path::PathBuf::from(format!("/tmp/item_{idx}"))],
                },
            );
        }
        assert_eq!(stack.len(), super::MAX_UNDO_HISTORY);
        assert_eq!(
            stack.pop(),
            Some(super::UndoAction::Trash {
                paths: vec![std::path::PathBuf::from(format!(
                    "/tmp/item_{}",
                    super::MAX_UNDO_HISTORY + 4
                ))]
            })
        );
        assert_eq!(
            stack.first(),
            Some(&super::UndoAction::Trash {
                paths: vec![std::path::PathBuf::from("/tmp/item_5")]
            })
        );
    }

    #[test]
    fn common_parent_dir_handles_same_dir_and_nested_selection() {
        let same = vec![
            std::path::PathBuf::from("/home/user/Dantes"),
            std::path::PathBuf::from("/home/user/Videos"),
            std::path::PathBuf::from("/home/user/bug.mp4"),
        ];
        assert_eq!(
            super::common_parent_dir(&same),
            Some(std::path::PathBuf::from("/home/user"))
        );

        let nested = vec![
            std::path::PathBuf::from("/home/user/projects/a/file1.rs"),
            std::path::PathBuf::from("/home/user/projects/b/sub/file2.rs"),
        ];
        assert_eq!(
            super::common_parent_dir(&nested),
            Some(std::path::PathBuf::from("/home/user/projects"))
        );
    }

    #[test]
    fn clear_tab_filters_resets_extension_and_query_and_restores_directories() {
        use fm_core::{EntryId, EntryKind, FileEntry, PermissionsMeta, TabId, TabState};
        use std::path::PathBuf;

        let mut tab = TabState::new(TabId(1), PathBuf::from("/tmp"));
        tab.raw_entries = vec![
            FileEntry {
                id: EntryId(1),
                name: "Projetos".into(),
                path: PathBuf::from("/tmp/Projetos"),
                kind: EntryKind::Directory,
                size_bytes: 0,
                modified_at: None,
                created_at: None,
                permissions: PermissionsMeta::PosixMode(0o755),
                is_hidden: false,
                extension: None,
                mime_badge: None,
                child_count: Some(3),
            },
            FileEntry {
                id: EntryId(2),
                name: "backup.zip".into(),
                path: PathBuf::from("/tmp/backup.zip"),
                kind: EntryKind::File,
                size_bytes: 1024,
                modified_at: None,
                created_at: None,
                permissions: PermissionsMeta::PosixMode(0o644),
                is_hidden: false,
                extension: Some("zip".into()),
                mime_badge: None,
                child_count: None,
            },
            FileEntry {
                id: EntryId(3),
                name: "notas.txt".into(),
                path: PathBuf::from("/tmp/notas.txt"),
                kind: EntryKind::File,
                size_bytes: 64,
                modified_at: None,
                created_at: None,
                permissions: PermissionsMeta::PosixMode(0o644),
                is_hidden: false,
                extension: Some("txt".into()),
                mime_badge: None,
                child_count: None,
            },
        ];

        tab.extension_filter = Some("zip".into());
        tab.filter_query = "back".to_owned();
        super::apply_tab_filter(&mut tab);
        assert_eq!(tab.visible_entries.len(), 1);
        assert_eq!(tab.visible_entries[0].name.as_str(), "backup.zip");

        assert!(super::clear_tab_filters(&mut tab));
        assert!(tab.extension_filter.is_none());
        assert!(tab.filter_query.is_empty());
        assert_eq!(tab.visible_entries.len(), 3);
        assert!(!super::clear_tab_filters(&mut tab));
    }

    #[test]
    fn parses_system_clipboard_uris_gnome_format_and_absolute_paths() {
        let gnome_copy = "copy\nfile:///home/user/FilmCraft%20Previews\nfile:///home/user/doc.txt";
        let (mode, paths) =
            super::parse_clipboard_transfer_text(gnome_copy).expect("gnome copy parses");
        assert_eq!(mode, fm_core::CopyMode::Copy);
        assert_eq!(
            paths,
            vec![
                std::path::PathBuf::from("/home/user/FilmCraft Previews"),
                std::path::PathBuf::from("/home/user/doc.txt"),
            ]
        );

        let gnome_cut = "cut\nfile:///tmp/pasta";
        let (mode_cut, paths_cut) =
            super::parse_clipboard_transfer_text(gnome_cut).expect("gnome cut parses");
        assert_eq!(mode_cut, fm_core::CopyMode::Move);
        assert_eq!(paths_cut, vec![std::path::PathBuf::from("/tmp/pasta")]);

        assert!(super::parse_clipboard_transfer_text("apenas texto solto").is_none());
    }

    #[test]
    fn resolves_open_in_new_tab_targets_for_single_and_multi_selection() {
        use fm_core::{EntryId, EntryKind, FileEntry, PermissionsMeta};
        use std::collections::HashSet;
        use std::path::{Path, PathBuf};

        let make_entry = |id: u64, name: &str, kind: EntryKind| FileEntry {
            id: EntryId(id),
            name: name.into(),
            path: PathBuf::from(format!("/tmp/{name}")),
            kind,
            size_bytes: 0,
            modified_at: None,
            created_at: None,
            permissions: PermissionsMeta::PosixMode(0o755),
            is_hidden: false,
            extension: None,
            mime_badge: None,
            child_count: None,
        };

        let entries = vec![
            make_entry(1, "Alpha", EntryKind::Directory),
            make_entry(2, "Beta", EntryKind::Directory),
            make_entry(
                3,
                "LinkDir",
                EntryKind::Symlink {
                    target_is_dir: true,
                },
            ),
            make_entry(4, "notes.txt", EntryKind::File),
        ];

        // Multiple folders selected -> returns all selected folders in visible order
        let sel_dirs: HashSet<EntryId> = [EntryId(1), EntryId(2), EntryId(3)].into_iter().collect();
        assert_eq!(
            super::resolve_open_in_new_tab_targets(
                Path::new("/tmp/Beta"),
                true,
                &entries,
                &sel_dirs,
            ),
            vec![
                PathBuf::from("/tmp/Alpha"),
                PathBuf::from("/tmp/Beta"),
                PathBuf::from("/tmp/LinkDir"),
            ]
        );

        // Mixed selection (folders + file) -> returns only the selected folders, even if right-clicked on the file
        let sel_mixed: HashSet<EntryId> = [EntryId(1), EntryId(4)].into_iter().collect();
        assert_eq!(
            super::resolve_open_in_new_tab_targets(
                Path::new("/tmp/notes.txt"),
                false,
                &entries,
                &sel_mixed,
            ),
            vec![PathBuf::from("/tmp/Alpha")]
        );

        // Unselected folder clicked -> falls back to just the clicked folder
        let sel_other: HashSet<EntryId> = [EntryId(1)].into_iter().collect();
        assert_eq!(
            super::resolve_open_in_new_tab_targets(
                Path::new("/tmp/Beta"),
                true,
                &entries,
                &sel_other,
            ),
            vec![PathBuf::from("/tmp/Beta")]
        );

        // Single file selected -> returns empty
        let sel_file: HashSet<EntryId> = [EntryId(4)].into_iter().collect();
        assert!(
            super::resolve_open_in_new_tab_targets(
                Path::new("/tmp/notes.txt"),
                false,
                &entries,
                &sel_file,
            )
            .is_empty()
        );
    }

    #[test]
    fn split_ratio_clamps_within_bounds_and_drop_destination_validates_safety() {
        use fm_core::CopyMode;
        use std::path::{Path, PathBuf};

        assert!((super::compute_split_ratio(0.5, 500.0, 600.0, 1000.0) - 0.6).abs() < 1e-4);
        assert!((super::compute_split_ratio(0.5, 500.0, 1200.0, 1000.0) - 0.75).abs() < 1e-4);
        assert!((super::compute_split_ratio(0.5, 500.0, 0.0, 1000.0) - 0.25).abs() < 1e-4);

        let drag = super::InternalEntryDrag {
            paths: vec![PathBuf::from("/tmp/left/docs")],
            primary_label: "docs".into(),
        };

        // Arrastar do painel esquerdo para o painel direito (diretório diferente) -> válido
        assert_eq!(
            super::resolve_internal_drop_destination(
                &drag,
                None,
                Some(super::SplitPane::Right),
                Some(Path::new("/tmp/left")),
                Some(Path::new("/tmp/right")),
                CopyMode::Move,
            ),
            Some(PathBuf::from("/tmp/right"))
        );

        // Mover para o mesmo diretório -> no-op (None)
        assert_eq!(
            super::resolve_internal_drop_destination(
                &drag,
                None,
                Some(super::SplitPane::Left),
                Some(Path::new("/tmp/left")),
                Some(Path::new("/tmp/right")),
                CopyMode::Move,
            ),
            None
        );

        // Copiar (Ctrl) no mesmo diretório -> permitido
        assert_eq!(
            super::resolve_internal_drop_destination(
                &drag,
                None,
                Some(super::SplitPane::Left),
                Some(Path::new("/tmp/left")),
                Some(Path::new("/tmp/right")),
                CopyMode::Copy,
            ),
            Some(PathBuf::from("/tmp/left"))
        );

        // Impede soltar pasta dentro dela mesma ou de um subdiretório descendente
        let self_folder = (super::SplitPane::Left, PathBuf::from("/tmp/left/docs"));
        assert_eq!(
            super::resolve_internal_drop_destination(
                &drag,
                Some(&self_folder),
                Some(super::SplitPane::Left),
                Some(Path::new("/tmp/left")),
                Some(Path::new("/tmp/right")),
                CopyMode::Copy,
            ),
            None
        );
        let child_folder = (
            super::SplitPane::Right,
            PathBuf::from("/tmp/left/docs/nested"),
        );
        assert_eq!(
            super::resolve_internal_drop_destination(
                &drag,
                Some(&child_folder),
                Some(super::SplitPane::Right),
                Some(Path::new("/tmp/left")),
                Some(Path::new("/tmp/right")),
                CopyMode::Move,
            ),
            None
        );

        // Soltar sobre uma subpasta válida no painel direito -> resolve para a subpasta
        let target_folder = (super::SplitPane::Right, PathBuf::from("/tmp/right/inbox"));
        assert_eq!(
            super::resolve_internal_drop_destination(
                &drag,
                Some(&target_folder),
                Some(super::SplitPane::Right),
                Some(Path::new("/tmp/left")),
                Some(Path::new("/tmp/right")),
                CopyMode::Move,
            ),
            Some(PathBuf::from("/tmp/right/inbox"))
        );
    }

    #[test]
    fn text_input_context_copy_cut_and_paste_preserve_utf8_selection() {
        // Cópia de seleção parcial (sem a extensão) e com acentos UTF-8
        assert_eq!(
            super::extract_text_input_copy("cabello1.jpg", Some((0, 8))),
            Some("cabello1".to_owned())
        );
        assert_eq!(
            super::extract_text_input_copy("ação_final.png", Some((0, 4))),
            Some("ação".to_owned())
        );
        // Sem seleção explícita -> copia o nome completo
        assert_eq!(
            super::extract_text_input_copy("cabello1.jpg", None),
            Some("cabello1.jpg".to_owned())
        );

        // Recortar seleção parcial
        assert_eq!(
            super::apply_text_input_cut("cabello1.jpg", Some((0, 8))),
            Some(("cabello1".to_owned(), ".jpg".to_owned(), 0))
        );

        // Colar substituindo seleção e removendo quebras de linha
        assert_eq!(
            super::apply_text_input_paste("cabello1.jpg", Some((0, 8)), 8, "foto_nova\n"),
            ("foto_nova.jpg".to_owned(), 9)
        );

        // Colar na posição do cursor sem seleção
        assert_eq!(
            super::apply_text_input_paste("cabello.jpg", None, 7, "_v2"),
            ("cabello_v2.jpg".to_owned(), 10)
        );
    }
}
