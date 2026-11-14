//! Workers canceláveis de telemetria contextual: Git (`gix`), SHA-256 (`sha2`), MIME (`infer`) e contagem rasa.

use compact_str::CompactString;

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
