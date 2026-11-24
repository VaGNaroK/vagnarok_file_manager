//! Modelos de domínio para o Analisador de Espaço em Disco e Arquivos Grandes.

use std::path::{Path, PathBuf};

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Categoria semântica de uso de espaço para gráficos proporcionais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiskCategory {
    IsoAndDisks,
    BuildAndCache,
    Archives,
    Media,
    Documents,
    Other,
}

impl DiskCategory {
    pub const ALL: [Self; 6] = [
        Self::IsoAndDisks,
        Self::BuildAndCache,
        Self::Archives,
        Self::Media,
        Self::Documents,
        Self::Other,
    ];

    /// Classifica um arquivo a partir do caminho/extensão para agrupamento no gráfico por tipo.
    #[must_use]
    pub fn classify(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        match ext.as_str() {
            "iso" | "img" | "vdi" | "vmdk" | "qcow2" | "vhd" | "vhdx" | "bin" | "cue" | "dmg" => {
                return Self::IsoAndDisks;
            }
            "zip" | "tar" | "gz" | "xz" | "zst" | "bz2" | "7z" | "rar" | "deb" | "rpm"
            | "appimage" => return Self::Archives,
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "mp3" | "flac" | "wav" | "ogg" | "png"
            | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "psd" | "raw" => return Self::Media,
            "pdf" | "doc" | "docx" | "odt" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md"
            | "csv" | "json" | "toml" | "yaml" | "yml" => return Self::Documents,
            "o" | "a" | "so" | "dll" | "pdb" | "rlib" | "rmeta" | "class" | "pyc" | "whl" => {
                return Self::BuildAndCache;
            }
            _ => {}
        }

        for comp in path.components() {
            if let std::path::Component::Normal(seg) = comp
                && let Some(s) = seg.to_str()
                && matches!(
                    s,
                    "target"
                        | "node_modules"
                        | ".cache"
                        | ".pub-cache"
                        | ".cargo"
                        | "__pycache__"
                        | ".gradle"
                        | "build"
                        | "dist"
                )
            {
                return Self::BuildAndCache;
            }
        }

        Self::Other
    }
}

/// Entrada de arquivo analisada pelo scanner de espaço em disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskScanFileEntry {
    pub path: PathBuf,
    pub name: CompactString,
    pub size_bytes: u64,
    pub category: DiskCategory,
}

/// Nó de diretório na árvore agregada do analisador de espaço.
#[derive(Debug, Clone)]
pub struct DiskScanDirNode {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub name: CompactString,
    pub path: PathBuf,
    pub total_bytes: u64,
    pub file_count: u64,
    /// Índices de subpastas diretas ordenados do maior para o menor tamanho.
    pub children: Vec<usize>,
    /// Arquivos diretos nesta pasta ordenados do maior para o menor tamanho.
    pub direct_files: Vec<DiskScanFileEntry>,
}

/// Resultado consolidado da varredura de espaço em disco.
#[derive(Debug, Clone)]
pub struct DiskScanResult {
    pub roots: Vec<PathBuf>,
    pub root_node_ids: Vec<usize>,
    pub nodes: Vec<DiskScanDirNode>,
    pub top_files: Vec<DiskScanFileEntry>,
    pub category_totals: [(DiskCategory, u64); 6],
    pub total_bytes: u64,
    pub total_files: u64,
    pub elapsed_ms: u64,
    pub skipped_errors: usize,
}

/// Progresso incremental emitido em background durante a varredura.
#[derive(Debug, Clone)]
pub struct DiskScanProgress {
    pub current_path: PathBuf,
    pub files_scanned: u64,
    pub bytes_scanned: u64,
    pub elapsed_secs: u64,
}

/// Eventos enviados pelo worker de análise de espaço para a UI.
#[derive(Debug, Clone)]
pub enum DiskScanEvent {
    Progress(DiskScanProgress),
    Completed(DiskScanResult),
    Cancelled,
}

/// Preset de filtro rápido por tamanho mínimo na lista de limpeza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiskMinSizeFilter {
    #[default]
    All,
    Mb100,
    Gb1,
    Gb5,
}

impl DiskMinSizeFilter {
    #[must_use]
    pub const fn min_bytes(self) -> u64 {
        match self {
            Self::All => 0,
            Self::Mb100 => 100 * 1024 * 1024,
            Self::Gb1 => 1024 * 1024 * 1024,
            Self::Gb5 => 5 * 1024 * 1024 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_disk_categories_by_extension_and_build_folders() {
        assert_eq!(
            DiskCategory::classify(Path::new("/home/u/Downloads/ubuntu.iso")),
            DiskCategory::IsoAndDisks
        );
        assert_eq!(
            DiskCategory::classify(Path::new("/home/u/Backups/site.tar.gz")),
            DiskCategory::Archives
        );
        assert_eq!(
            DiskCategory::classify(Path::new("/home/u/Videos/clip.mkv")),
            DiskCategory::Media
        );
        assert_eq!(
            DiskCategory::classify(Path::new("/home/u/proj/target/debug/deps/foo.d")),
            DiskCategory::BuildAndCache
        );
    }
}
