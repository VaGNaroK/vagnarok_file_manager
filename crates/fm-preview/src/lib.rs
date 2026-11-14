//! Pipeline de geração de thumbnails SIMD, cache LRU/persistente e QuickLook (Fase P2).

use std::path::PathBuf;

/// Chave única para cache de miniaturas baseada em identidade temporal do arquivo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbnailKey {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_nanos: u128,
}
