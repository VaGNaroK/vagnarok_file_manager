//! Workers canceláveis de telemetria contextual: Git (`gix`), SHA-256 (`sha2`), MIME (`infer`) e contagem rasa.

pub mod git;
pub mod hash;
pub mod mime;
mod open_shared;
pub mod stats;

use compact_str::CompactString;

pub use git::detect_git_branch;
pub use hash::compute_sha256;
pub use mime::detect_mime_type;
pub use stats::count_direct_children;

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
