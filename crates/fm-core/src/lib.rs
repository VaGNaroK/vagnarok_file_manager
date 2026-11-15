//! Domínio puro, contratos VFS, estado de abas/seleção e internacionalização do Vagnarok File Manager.
//!
//! Este crate não depende de `iced` nem de APIs específicas de sistema operacional.

pub mod domain;
pub mod i18n;
pub mod state;
pub mod vfs;

pub use domain::{
    DirectorySnapshot, DirectoryTelemetry, DriveInfo, DriveKind, EntryId, EntryKind, FileEntry,
    FolderSummary, PermissionsMeta, SelectedFileTelemetry, SortDirection, SortField, SortSpec,
    UserPlaces, ViewMode, VolumeSpace, compute_entry_id, format_age, format_bytes,
};
pub use i18n::Strings;
pub use state::{
    AppConfig, Language, SelectionModifiers, SelectionState, TabId, TabState, ThemePreset,
    WindowDecorationsMode,
};
pub use vfs::{FsError, VfsProvider};
