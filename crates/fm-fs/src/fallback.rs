use std::fs::Metadata;
use std::path::Path;

use compact_str::CompactString;
use fm_core::{DriveInfo, PermissionsMeta, VolumeSpace};

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
