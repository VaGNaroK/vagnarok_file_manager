//! Watchers de sistema de arquivos com debounce e busca fuzzy instantânea (Fase P2).

use std::path::PathBuf;

/// Eventos normalizados de mudança em diretórios observados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    Modified,
    Created,
    Removed,
    Renamed,
}

/// Placeholder do serviço de indexação e observação para o workspace P0.
#[derive(Debug, Default)]
pub struct DirectoryIndex {
    pub active_root: Option<PathBuf>,
}
