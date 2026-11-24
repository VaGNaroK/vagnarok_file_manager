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

pub fn open_default(path: &Path) -> Result<(), FsError> {
    open::that_detached(path).map_err(|e| crate::map_io_err(path, e))
}

pub fn open_properties(path: &Path) -> Result<(), FsError> {
    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "diálogo nativo de propriedades indisponível nesta plataforma".to_owned(),
    })
}

pub fn extract_permissions_and_hidden(name: &str, _meta: &Metadata) -> (PermissionsMeta, bool) {
    (PermissionsMeta::PosixMode(0o644), name.starts_with('.'))
}

pub fn apply_entry_properties(path: &Path, patch: &crate::PropertyPatch) -> Result<(), FsError> {
    if let Some(readonly) = patch.readonly {
        let meta = std::fs::metadata(path).map_err(|e| crate::map_io_err(path, e))?;
        let mut perms = meta.permissions();
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms).map_err(|e| crate::map_io_err(path, e))?;
    }
    Ok(())
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

pub fn mount_drive(device_path: &Path) -> Result<std::path::PathBuf, FsError> {
    Err(FsError::Io {
        path: device_path.to_path_buf(),
        message: "montagem de discos não suportada nesta plataforma".to_owned(),
    })
}

pub fn unmount_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    Err(FsError::Io {
        path: mount_path
            .or(device_path)
            .unwrap_or_else(|| Path::new("/"))
            .to_path_buf(),
        message: "desmontagem de discos não suportada nesta plataforma".to_owned(),
    })
}

pub fn mount_disk_image(image_path: &Path) -> Result<std::path::PathBuf, FsError> {
    Err(FsError::Io {
        path: image_path.to_path_buf(),
        message: "montagem de imagens de disco não suportada nesta plataforma".to_owned(),
    })
}

pub fn eject_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    unmount_drive(device_path, mount_path)
}

pub fn drives_fingerprint() -> u64 {
    0
}

pub fn supports_open_as_root() -> bool {
    false
}

pub fn is_elevated_session() -> bool {
    false
}

pub fn open_as_root(path: &Path) -> Result<(), FsError> {
    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "elevação de privilégios não suportada nesta plataforma".to_owned(),
    })
}

pub fn create_shortcut(target: &Path, _dest_dir: &Path) -> Result<std::path::PathBuf, FsError> {
    Err(FsError::Io {
        path: target.to_path_buf(),
        message: "criação de atalhos não suportada nesta plataforma".to_owned(),
    })
}

pub fn resolve_shortcut_target(path: &Path) -> Result<std::path::PathBuf, FsError> {
    std::fs::canonicalize(path).map_err(|e| crate::map_io_err(path, e))
}
