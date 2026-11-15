use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{File, Metadata};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use compact_str::CompactString;
use fm_core::{
    DirectorySnapshot, DriveInfo, DriveKind, EntryKind, FsError, PermissionsMeta, VolumeSpace,
};
use rustix::fd::{AsFd, BorrowedFd};
use rustix::fs::{AtFlags, Mode, OFlags, RawDir, StatxFlags};
use tokio_util::sync::CancellationToken;

use crate::{make_file_entry, map_io_err};

/// Tenta clone instantâneo Copy-on-Write (`FICLONE`, Btrfs/XFS) no Linux.
/// Retorna `None` quando o filesystem não suporta reflink para prosseguir com `copy_file_range`.
pub fn try_ficlone(in_file: &File, out_file: &File) -> Option<std::io::Result<()>> {
    match rustix::fs::ioctl_ficlone(out_file, in_file) {
        Ok(()) => Some(Ok(())),
        Err(errno)
            if errno == rustix::io::Errno::XDEV
                || errno == rustix::io::Errno::OPNOTSUPP
                || errno == rustix::io::Errno::INVAL
                || errno == rustix::io::Errno::NOSYS
                || errno == rustix::io::Errno::NOTTY
                || errno == rustix::io::Errno::ISDIR =>
        {
            None
        }
        Err(errno) => Some(Err(std::io::Error::from_raw_os_error(errno.raw_os_error()))),
    }
}

/// Lista diretório no Linux via `open(O_DIRECTORY)` + `RawDir` (`getdents64`) + `statat`,
/// evitando construir `PathBuf`s para cada `lstat` e abortando em cancelamento.
pub fn scan_directory_native(
    path: &Path,
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let dir_fd = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|errno| map_io_err(path, std::io::Error::from_raw_os_error(errno.raw_os_error())))?;

    let mut buf = vec![MaybeUninit::<u8>::uninit(); 32 * 1024];
    let mut entries = Vec::new();
    let mut iter = RawDir::new(&dir_fd, &mut buf);
    let mut processed = 0usize;

    while let Some(item_res) = iter.next() {
        if processed.is_multiple_of(256) && cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        processed += 1;

        let Ok(raw_entry) = item_res else {
            continue;
        };

        let raw_name = raw_entry.file_name();
        let name_bytes = raw_name.to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }

        let Some(link_stat) = query_entry_stat(dir_fd.as_fd(), raw_name, AtFlags::SYMLINK_NOFOLLOW) else {
            continue;
        };

        let link_fmt = link_stat.mode & 0o170_000;
        let (kind, effective_stat) = if link_fmt == 0o120_000 {
            match query_entry_stat(dir_fd.as_fd(), raw_name, AtFlags::empty()) {
                Some(target_stat) => {
                    let target_is_dir = (target_stat.mode & 0o170_000) == 0o040_000;
                    (EntryKind::Symlink { target_is_dir }, target_stat)
                }
                None => (
                    EntryKind::Symlink {
                        target_is_dir: false,
                    },
                    link_stat,
                ),
            }
        } else if link_fmt == 0o040_000 {
            (EntryKind::Directory, link_stat)
        } else if link_fmt == 0o100_000 {
            (EntryKind::File, link_stat)
        } else {
            (EntryKind::Special, link_stat)
        };

        let name_os = OsStr::from_bytes(name_bytes);
        let name = CompactString::from(name_os.to_string_lossy().as_ref());
        let entry_path = path.join(name_os);
        let is_hidden = name_bytes.starts_with(b".");
        let permissions = PermissionsMeta::PosixMode(effective_stat.mode);

        entries.push(make_file_entry(
            name,
            entry_path,
            kind,
            effective_stat.size_bytes,
            effective_stat.modified_at,
            effective_stat.created_at,
            permissions,
            is_hidden,
        ));
    }

    let fs_type = fs_type_for_path(path);

    Ok(DirectorySnapshot {
        request_id,
        path: path.to_path_buf(),
        entries,
        fs_type,
    })
}

struct NativeEntryStat {
    mode: u32,
    size_bytes: u64,
    modified_at: Option<SystemTime>,
    created_at: Option<SystemTime>,
}

fn query_entry_stat(
    dir_fd: BorrowedFd<'_>,
    name: &std::ffi::CStr,
    flags: AtFlags,
) -> Option<NativeEntryStat> {
    match rustix::fs::statx(
        dir_fd,
        name,
        flags,
        StatxFlags::BASIC_STATS | StatxFlags::BTIME,
    ) {
        Ok(stx) => {
            let modified_at = if (stx.stx_mask & StatxFlags::MTIME.bits()) != 0 {
                unix_timespec_to_system_time(stx.stx_mtime.tv_sec, stx.stx_mtime.tv_nsec)
            } else {
                None
            };
            let created_at = if (stx.stx_mask & StatxFlags::BTIME.bits()) != 0 {
                unix_timespec_to_system_time(stx.stx_btime.tv_sec, stx.stx_btime.tv_nsec)
            } else {
                None
            };
            Some(NativeEntryStat {
                mode: u32::from(stx.stx_mode),
                size_bytes: stx.stx_size,
                modified_at,
                created_at,
            })
        }
        Err(errno)
            if errno == rustix::io::Errno::NOSYS || errno == rustix::io::Errno::OPNOTSUPP =>
        {
            let stat = rustix::fs::statat(dir_fd, name, flags).ok()?;
            Some(NativeEntryStat {
                mode: stat.st_mode,
                size_bytes: u64::try_from(stat.st_size).unwrap_or(0),
                modified_at: unix_timespec_to_system_time(
                    stat.st_mtime,
                    stat.st_mtime_nsec as u32,
                ),
                created_at: None,
            })
        }
        Err(_) => None,
    }
}

fn unix_timespec_to_system_time(secs: i64, nsec: u32) -> Option<SystemTime> {
    let nanos = nsec.min(999_999_999);
    if secs >= 0 {
        UNIX_EPOCH.checked_add(Duration::new(secs as u64, nanos))
    } else {
        let abs_secs = secs.unsigned_abs();
        UNIX_EPOCH
            .checked_sub(Duration::from_secs(abs_secs))?
            .checked_add(Duration::from_nanos(u64::from(nanos)))
    }
}

/// Abre o emulador de terminal disponível no ambiente Linux com `cwd` apontando para `path`.
pub fn open_terminal_at(path: &Path) -> Result<(), FsError> {
    let env_term = std::env::var("TERMINAL").ok();
    let candidates: [&str; 9] = [
        "xdg-terminal-exec",
        "x-terminal-emulator",
        "kitty",
        "alacritty",
        "wezterm",
        "foot",
        "gnome-terminal",
        "konsole",
        "xterm",
    ];

    if let Some(term) = env_term.as_deref().filter(|s| !s.trim().is_empty())
        && Command::new(term).current_dir(path).spawn().is_ok()
    {
        return Ok(());
    }

    for bin in candidates {
        if Command::new(bin).current_dir(path).spawn().is_ok() {
            return Ok(());
        }
    }

    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "nenhum emulador de terminal encontrado no PATH ($TERMINAL)".to_owned(),
    })
}

/// Tenta transferência kernel-to-kernel via `copy_file_range(2)` no Linux.
/// Retorna `None` quando o par de descritores/filesystem exige fallback para read/write em userspace.
pub fn try_copy_file_range_chunk(
    in_file: &File,
    out_file: &File,
    len: usize,
) -> Option<std::io::Result<usize>> {
    match rustix::fs::copy_file_range(in_file, None, out_file, None, len) {
        Ok(bytes) => Some(Ok(bytes)),
        Err(errno)
            if errno == rustix::io::Errno::XDEV
                || errno == rustix::io::Errno::NOSYS
                || errno == rustix::io::Errno::INVAL
                || errno == rustix::io::Errno::OPNOTSUPP =>
        {
            None
        }
        Err(errno) => Some(Err(std::io::Error::from_raw_os_error(errno.raw_os_error()))),
    }
}

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
