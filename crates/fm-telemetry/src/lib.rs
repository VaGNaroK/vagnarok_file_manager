//! Workers canceláveis de telemetria contextual: Git (`gix`), SHA-256 (`sha2`), MIME (`infer`) e contagem rasa.

pub mod duplicates;
pub mod git;
pub mod hardlink;
pub mod hash;
pub mod md5;
pub mod media_tags;
pub mod meta_cache;
pub mod mime;
mod open_shared;
pub mod stats;

use compact_str::CompactString;

pub use duplicates::run_duplicate_scan;
pub use git::{detect_file_git_status, detect_git_branch};
pub use hash::compute_sha256;
pub use meta_cache::{CachedFileTelemetry, DiskMetadataCache, mtime_nanos};
pub use mime::detect_mime_type;
pub use stats::{
    compute_folder_size_cached, compute_paths_size_cached, count_direct_children,
    count_direct_children_cached,
};

/// Atualização incremental de telemetria enviada do worker para a UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryUpdate {
    GitBranch(Option<CompactString>),
    Sha256Computed {
        prefix: CompactString,
        full: CompactString,
    },
    MimeDetected(CompactString),
}
