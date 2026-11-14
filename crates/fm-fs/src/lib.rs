//! Drivers nativos de sistema de arquivos (Linux/Windows), descoberta de volumes e engine de cópia.
//!
//! Implementações específicas de SO ficam confinadas em submódulos `#[cfg(target_os = ...)]`.

use std::path::{Path, PathBuf};

use fm_core::{DirectorySnapshot, DriveInfo, FsError, VfsProvider, VolumeSpace};
use tokio_util::sync::CancellationToken;

/// Implementação local de `VfsProvider` (será preenchida na Fase P1 e otimizada na Fase P3).
#[derive(Debug, Default)]
pub struct LocalFs;

impl LocalFs {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl VfsProvider for LocalFs {
    async fn list_dir(
        &self,
        path: &Path,
        _request_id: u64,
        _cancel: CancellationToken,
    ) -> Result<DirectorySnapshot, FsError> {
        Err(FsError::Io {
            path: path.to_path_buf(),
            message: "LocalFs::list_dir será implementado na Fase P1".to_owned(),
        })
    }

    async fn list_drives(&self) -> Result<Vec<DriveInfo>, FsError> {
        Ok(Vec::new())
    }

    async fn volume_space(&self, _path: &Path) -> Result<Option<VolumeSpace>, FsError> {
        Ok(None)
    }

    async fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::Io {
            path: path.to_path_buf(),
            message: "não implementado na Fase P0".to_owned(),
        })
    }

    async fn create_file(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::Io {
            path: path.to_path_buf(),
            message: "não implementado na Fase P0".to_owned(),
        })
    }

    async fn rename(&self, from: &Path, _to: &Path) -> Result<(), FsError> {
        Err(FsError::Io {
            path: from.to_path_buf(),
            message: "não implementado na Fase P0".to_owned(),
        })
    }

    async fn move_to_trash(&self, paths: &[PathBuf]) -> Result<(), FsError> {
        Err(FsError::Io {
            path: paths.first().cloned().unwrap_or_default(),
            message: "não implementado na Fase P0".to_owned(),
        })
    }

    async fn delete_permanent(&self, paths: &[PathBuf]) -> Result<(), FsError> {
        Err(FsError::Io {
            path: paths.first().cloned().unwrap_or_default(),
            message: "não implementado na Fase P0".to_owned(),
        })
    }

    async fn open_default(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::Io {
            path: path.to_path_buf(),
            message: "não implementado na Fase P0".to_owned(),
        })
    }
}
