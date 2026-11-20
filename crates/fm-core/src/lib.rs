//! Domínio puro, contratos VFS, estado de abas/seleção e internacionalização do Vagnarok File Manager.
//!
//! Este crate não depende de `iced` nem de APIs específicas de sistema operacional.

pub mod archive_job;
pub mod bulk_rename;
pub mod domain;
pub mod duplicates;
pub mod i18n;
pub mod state;
pub mod vfs;

pub use archive_job::{
    ArchiveFormat, ArchiveJobKind, ArchiveJobProgress, CompressionLevel, ExtractTargetMode,
    sanitize_archive_rel_path, strip_known_archive_suffix, suggest_archive_base_name,
};

pub use duplicates::{
    DuplicateConfirmLevel, DuplicateFileItem, DuplicateGroup, DuplicateMinSizePreset,
    DuplicateScanEvent, DuplicateScanOptions, DuplicateScanProgress, DuplicateScanStage,
    KeepStrategy, apply_keep_strategy, format_short_hash,
};

pub use bulk_rename::{
    BulkRenameEvaluation, BulkRenameExecutionPlan, BulkRenameInput, BulkRenameMediaTags,
    BulkRenameNameError, BulkRenamePreviewRow, BulkRenameRowStatus, BulkRenameRules,
    BulkRenameSort, BulkRenameTargetPolicy, ExtensionMode, InsertPosition, MatchScope,
    MediaComposeMode, MediaTagKind, NumberPosition, RenameStep, TextCaseMode, build_execution_plan,
    evaluate_bulk_rename,
};

pub use domain::{
    ClipboardTransfer, ConflictPolicy, CopyMode, DirectorySnapshot, DirectoryTelemetry, DriveInfo,
    DriveKind, EntryExtraInfo, EntryId, EntryKind, FileEntry, FolderSummary, JobProgress, JobState,
    LocalTimestampKind, LocaleFormatSpec, OpenWithApp, OpenWithAssociation, OpenWithCatalog,
    PermissionsMeta, RecursiveSize, SelectedFileTelemetry, SortDirection, SortField, SortSpec,
    SystemIconBucket, SystemIconKey, SystemIconPayload, SystemPlaceKind, UserPlaces,
    VIRTUAL_TRASH_PATH, ViewMode, VolumeSpace, compute_entry_id, format_bytes, format_grouped_u64,
    is_mountable_disk_image, is_posix_executable, is_virtual_trash_path, local_timestamp_kind,
    open_with_type_key, suggest_shortcut_name, toggle_posix_executable_mode,
};
pub use i18n::Strings;
pub use state::{
    AppConfig, DIR_FILTER_SENTINEL, IconThemeMode, Language, SelectionModifiers, SelectionState,
    TabId, TabState, ThemePreset, WindowDecorationsMode, format_extension_filter_badge,
    matches_extension_filter, normalize_extension_filter_input,
};
pub use vfs::{FsError, VfsProvider};
