//! Domínio puro, contratos VFS, estado de abas/seleção e internacionalização do Vagnarok File Manager.
//!
//! Este crate não depende de `iced` nem de APIs específicas de sistema operacional.

pub mod domain;
pub mod i18n;
pub mod state;
pub mod vfs;

pub use domain::{
    ClipboardTransfer, ConflictPolicy, CopyMode, DirectorySnapshot, DirectoryTelemetry, DriveInfo,
    DriveKind, EntryExtraInfo, EntryId, EntryKind, FileEntry, FolderSummary, JobProgress, JobState,
    LocalTimestampKind, LocaleFormatSpec, PermissionsMeta, RecursiveSize, SelectedFileTelemetry,
    SortDirection, SortField, SortSpec, UserPlaces, VIRTUAL_TRASH_PATH, ViewMode, VolumeSpace,
    compute_entry_id, format_bytes, format_grouped_u64, is_virtual_trash_path,
    local_timestamp_kind,
};
pub use i18n::Strings;
pub use state::{
    AppConfig, DIR_FILTER_SENTINEL, Language, SelectionModifiers, SelectionState, TabId, TabState,
    ThemePreset, WindowDecorationsMode,
};
pub use vfs::{FsError, VfsProvider};
