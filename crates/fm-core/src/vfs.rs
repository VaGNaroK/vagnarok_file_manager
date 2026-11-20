use std::path::{Path, PathBuf};

use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::domain::{DirectorySnapshot, DriveInfo, OpenWithAssociation, VolumeSpace};

/// Erros de sistema de arquivos tratados de forma explícita na camada de UI.
#[derive(Debug, Error, Clone)]
pub enum FsError {
    #[error("caminho não encontrado: {0}")]
    NotFound(PathBuf),
    #[error("permissão negada ao acessar: {0}")]
    PermissionDenied(PathBuf),
    #[error("operação cancelada")]
    Cancelled,
    #[error("senha necessária para extrair o arquivo: {0}")]
    PasswordRequired(PathBuf),
    #[error("senha incorreta para o arquivo: {0}")]
    InvalidPassword(PathBuf),
    #[error("falha de I/O em {path}: {message}")]
    Io { path: PathBuf, message: String },
}

/// Abstração assíncrona de sistema de arquivos (LocalFs hoje; ArchiveVfs / SftpVfs no futuro).
#[async_trait::async_trait]
pub trait VfsProvider: Send + Sync {
    async fn list_dir(
        &self,
        path: &Path,
        request_id: u64,
        cancel: CancellationToken,
    ) -> Result<DirectorySnapshot, FsError>;

    async fn list_drives(&self) -> Result<Vec<DriveInfo>, FsError>;
    async fn volume_space(&self, path: &Path) -> Result<Option<VolumeSpace>, FsError>;
    async fn create_dir(&self, path: &Path) -> Result<(), FsError>;
    async fn create_file(&self, path: &Path) -> Result<(), FsError>;
    async fn create_shortcuts(
        &self,
        targets: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>, FsError>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;
    async fn move_to_trash(&self, paths: &[PathBuf]) -> Result<(), FsError>;
    async fn restore_from_trash(&self, paths: &[PathBuf]) -> Result<usize, FsError>;
    async fn empty_trash(&self) -> Result<(), FsError>;
    async fn delete_permanent(&self, paths: &[PathBuf]) -> Result<(), FsError>;
    async fn open_default(&self, path: &Path) -> Result<(), FsError>;
    async fn open_with(&self, path: &Path, target: &OpenWithAssociation) -> Result<(), FsError>;
    async fn open_terminal(&self, path: &Path) -> Result<(), FsError>;
    async fn open_properties(&self, path: &Path) -> Result<(), FsError>;
    async fn read_bytes(
        &self,
        path: &Path,
        max_bytes: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<u8>, FsError>;
}
