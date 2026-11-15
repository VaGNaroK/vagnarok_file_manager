use std::collections::HashSet;
use std::fs::Metadata;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use compact_str::CompactString;
use fm_core::{DriveInfo, DriveKind, PermissionsMeta, VolumeSpace};

const IGNORED_FS_TYPES: &[&str] = &[
    "proc",
    "sysfs",
    "devtmpfs",
    "devpts",
    "tmpfs",
    "cgroup",
    "cgroup2",
    "pstore",
    "securityfs",
    "debugfs",
    "tracefs",
    "configfs",
    "fusectl",
    "efivarfs",
    "hugetlbfs",
    "mqueue",
    "binfmt_misc",
    "autofs",
    "rpc_pipefs",
    "overlay",
    "squashfs",
    "nsfs",
    "ramfs",
    "bpf",
    "fuse.portal",
];

pub fn extract_permissions_and_hidden(name: &str, meta: &Metadata) -> (PermissionsMeta, bool) {
    let mode = meta.permissions().mode();
    let is_hidden = name.starts_with('.');
    (PermissionsMeta::PosixMode(mode), is_hidden)
}

pub fn volume_space(path: &Path) -> Option<VolumeSpace> {
    let stat = rustix::fs::statvfs(path).ok()?;
    let block_size = if stat.f_frsize > 0 {
        stat.f_frsize
    } else {
        stat.f_bsize
    };
    let total_bytes = stat.f_blocks.saturating_mul(block_size);
    let free_bytes = stat.f_bavail.saturating_mul(block_size);
    if total_bytes == 0 {
        return None;
    }

    let label = mount_label_for_path(path);
    Some(VolumeSpace {
        label,
        free_bytes,
        total_bytes,
    })
}

pub fn fs_type_for_path(path: &Path) -> Option<CompactString> {
    let mounts = read_proc_mounts();
    best_mount_for_path(path, &mounts).map(|m| m.fs_type)
}

pub fn list_drives() -> Vec<DriveInfo> {
    let mounts = read_proc_mounts();
    let mut seen = HashSet::new();
    let mut drives = Vec::new();

    for mount in mounts {
        if IGNORED_FS_TYPES.contains(&mount.fs_type.as_str()) {
            continue;
        }
        if mount.mount_point.starts_with("/boot")
            || mount.mount_point.starts_with("/snap")
            || mount.mount_point.starts_with("/sys")
            || mount.mount_point.starts_with("/proc")
            || mount.mount_point.starts_with("/dev")
        {
            continue;
        }
        if !seen.insert(mount.mount_point.clone()) {
            continue;
        }

        let space = volume_space(&mount.mount_point);
        if space.is_none() {
            continue;
        }

        let kind = classify_drive(&mount.mount_point, &mount.fs_type);
        let label = drive_label(&mount.mount_point, kind);
        drives.push(DriveInfo {
            label,
            mount_path: mount.mount_point,
            fs_type: Some(mount.fs_type),
            kind,
            space,
        });
    }

    drives
}

struct MountEntry {
    mount_point: PathBuf,
    fs_type: CompactString,
}

fn read_proc_mounts() -> Vec<MountEntry> {
    let Ok(raw) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _device = parts.next()?;
            let mount_raw = parts.next()?;
            let fs_type = parts.next()?;
            Some(MountEntry {
                mount_point: PathBuf::from(unescape_proc_mount(mount_raw)),
                fs_type: CompactString::new(fs_type),
            })
        })
        .collect()
}

fn best_mount_for_path(path: &Path, mounts: &[MountEntry]) -> Option<MountEntry> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    mounts
        .iter()
        .filter(|m| canonical.starts_with(&m.mount_point))
        .max_by_key(|m| m.mount_point.as_os_str().len())
        .map(|m| MountEntry {
            mount_point: m.mount_point.clone(),
            fs_type: m.fs_type.clone(),
        })
}

fn classify_drive(mount_point: &Path, fs_type: &str) -> DriveKind {
    if mount_point == Path::new("/") {
        return DriveKind::InternalSsd;
    }
    if matches!(fs_type, "nfs" | "nfs4" | "cifs" | "smb3" | "fuse.sshfs") {
        return DriveKind::Network;
    }
    if mount_point.starts_with("/run/media") || mount_point.starts_with("/media") {
        return DriveKind::Removable;
    }
    DriveKind::SecondaryDisk
}

fn drive_label(mount_point: &Path, kind: DriveKind) -> CompactString {
    if mount_point == Path::new("/") {
        return CompactString::new("Root (/)");
    }
    match kind {
        DriveKind::InternalSsd => CompactString::new("SSD Principal"),
        _ => mount_point
            .file_name()
            .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| CompactString::from(mount_point.to_string_lossy().as_ref())),
    }
}

fn mount_label_for_path(path: &Path) -> CompactString {
    let mounts = read_proc_mounts();
    if let Some(best) = best_mount_for_path(path, &mounts) {
        let kind = classify_drive(&best.mount_point, &best.fs_type);
        return drive_label(&best.mount_point, kind);
    }
    CompactString::new("Disk")
}

/// Decodifica escapes octais usados pelo kernel em `/proc/mounts` (ex.: `\040` -> espaço).
fn unescape_proc_mount(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let oct = &bytes[i + 1..i + 4];
            if oct.iter().all(|b| matches!(b, b'0'..=b'7'))
                && let Ok(s) = std::str::from_utf8(oct)
                && let Ok(val) = u8::from_str_radix(s, 8)
            {
                out.push(val);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_proc_mount_spaces() {
        assert_eq!(unescape_proc_mount("/mnt/My\\040Drive"), "/mnt/My Drive");
    }
}
