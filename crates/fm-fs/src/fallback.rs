use std::fs::Metadata;
use std::path::Path;

use compact_str::CompactString;
use fm_core::{DirectorySnapshot, DriveInfo, FsError, PermissionsMeta, VolumeSpace};
use tokio_util::sync::CancellationToken;

pub fn scan_directory_native(
    path: &Path,
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    crate::scan_directory_blocking(path, request_id, cancel)
}

pub fn open_terminal_at(path: &Path) -> Result<(), FsError> {
    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "abertura de terminal não suportada nesta plataforma".to_owned(),
    })
}

pub fn extract_permissions_and_hidden(name: &str, _meta: &Metadata) -> (PermissionsMeta, bool) {
    (PermissionsMeta::PosixMode(0o644), name.starts_with('.'))
}

pub fn volume_space(_path: &Path) -> Option<VolumeSpace> {
    None
}

pub fn fs_type_for_path(_path: &Path) -> Option<CompactString> {
    None
}

pub fn list_drives() -> Vec<DriveInfo> {
    Vec::new()
}
