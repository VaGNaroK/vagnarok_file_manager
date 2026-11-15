use std::fs::Metadata;
use std::os::windows::fs::MetadataExt;
use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

use compact_str::CompactString;
use fm_core::{DriveInfo, DriveKind, PermissionsMeta, VolumeSpace};
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
};
use windows::core::PCWSTR;

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

pub fn extract_permissions_and_hidden(_name: &str, meta: &Metadata) -> (PermissionsMeta, bool) {
    let attrs = meta.file_attributes();
    let is_hidden = (attrs & FILE_ATTRIBUTE_HIDDEN) != 0;
    (PermissionsMeta::WinAttributes(attrs), is_hidden)
}

pub fn volume_space(path: &Path) -> Option<VolumeSpace> {
    let root = drive_root_for_path(path).unwrap_or_else(|| path.to_path_buf());
    let wide = to_wide(&root);

    let mut free_available: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free: u64 = 0;

    // SAFETY: `wide` é null-terminated válido e os ponteiros apontam para variáveis locais mutáveis.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut free_available),
            Some(&mut total_bytes),
            Some(&mut total_free),
        )
    };
    if ok.is_err() || total_bytes == 0 {
        return None;
    }

    let (vol_name, _) = query_volume_info(&root);
    let label = vol_name.unwrap_or_else(|| {
        CompactString::from(root.to_string_lossy().trim_end_matches('\\'))
    });

    Some(VolumeSpace {
        label,
        free_bytes: free_available,
        total_bytes,
    })
}

pub fn fs_type_for_path(path: &Path) -> Option<CompactString> {
    let root = drive_root_for_path(path)?;
    let (_, fs_name) = query_volume_info(&root);
    fs_name
}

pub fn list_drives() -> Vec<DriveInfo> {
    let mut buffer = vec![0u16; 512];
    // SAFETY: buffer pré-alocado de 512 UTF-16 code units para receber strings separadas por NUL.
    let len = unsafe { GetLogicalDriveStringsW(Some(&mut buffer)) } as usize;
    if len == 0 || len > buffer.len() {
        return Vec::new();
    }

    let mut drives = Vec::new();
    let mut idx = 0usize;
    for (i, slice) in buffer[..len].split(|&c| c == 0).enumerate() {
        if slice.is_empty() {
            continue;
        }
        let root_str = String::from_utf16_lossy(slice);
        let mount_path = PathBuf::from(&root_str);
        let wide = to_wide(&mount_path);

        // SAFETY: `wide` é uma string UTF-16 terminada em NUL representando a raiz do drive.
        let drive_type = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
        let kind = match drive_type {
            DRIVE_REMOVABLE => DriveKind::Removable,
            DRIVE_REMOTE => DriveKind::Network,
            DRIVE_FIXED if i == 0 => DriveKind::InternalSsd,
            _ => DriveKind::SecondaryDisk,
        };

        let (vol_name, fs_type) = query_volume_info(&mount_path);
        let space = volume_space(&mount_path);
        let display = match vol_name {
            Some(name) if !name.is_empty() => {
                CompactString::from(format!("{name} ({})", root_str.trim_end_matches('\\')))
            }
            _ => CompactString::from(root_str.trim_end_matches('\\')),
        };

        drives.push(DriveInfo {
            label: display,
            mount_path,
            fs_type,
            kind,
            space,
        });
        idx += 1;
        let _ = idx;
    }

    drives
}

fn query_volume_info(root: &Path) -> (Option<CompactString>, Option<CompactString>) {
    let wide = to_wide(root);
    let mut vol_name_buf = [0u16; 260];
    let mut fs_name_buf = [0u16; 64];

    // SAFETY: buffers válidos na stack e `wide` terminado em NUL.
    let res = unsafe {
        GetVolumeInformationW(
            PCWSTR(wide.as_ptr()),
            Some(&mut vol_name_buf),
            None,
            None,
            None,
            Some(&mut fs_name_buf),
        )
    };
    if res.is_err() {
        return (None, None);
    }

    (
        decode_wide_z(&vol_name_buf),
        decode_wide_z(&fs_name_buf),
    )
}

fn drive_root_for_path(path: &Path) -> Option<PathBuf> {
    let mut comps = path.components();
    match comps.next()? {
        Component::Prefix(prefix) => {
            let mut root = PathBuf::from(prefix.as_os_str());
            root.push("\\");
            Some(root)
        }
        _ => None,
    }
}

fn to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn decode_wide_z(buf: &[u16]) -> Option<CompactString> {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if end == 0 {
        return None;
    }
    Some(CompactString::from(String::from_utf16_lossy(&buf[..end])))
}
