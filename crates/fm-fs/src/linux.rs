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

use crate::{PropertyPatch, make_file_entry, map_io_err};

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
    .map_err(|errno| {
        map_io_err(
            path,
            std::io::Error::from_raw_os_error(errno.raw_os_error()),
        )
    })?;

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

        let Some(link_stat) = query_entry_stat(dir_fd.as_fd(), raw_name, AtFlags::SYMLINK_NOFOLLOW)
        else {
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
                modified_at: unix_timespec_to_system_time(stat.st_mtime, stat.st_mtime_nsec as u32),
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
    let in_flatpak = std::env::var_os("FLATPAK_ID").is_some();

    if let Some(term) = env_term.as_deref().filter(|s| !s.trim().is_empty())
        && spawn_terminal_cmd(term, path, in_flatpak)
    {
        return Ok(());
    }

    for bin in candidates {
        if spawn_terminal_cmd(bin, path, in_flatpak) {
            return Ok(());
        }
    }

    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "nenhum emulador de terminal encontrado no PATH ($TERMINAL)".to_owned(),
    })
}

fn spawn_terminal_cmd(bin: &str, path: &Path, in_flatpak: bool) -> bool {
    if in_flatpak {
        let has_host_bin = Command::new("flatpak-spawn")
            .args([
                "--host",
                "sh",
                "-c",
                "command -v \"$1\" >/dev/null 2>&1",
                "sh",
                bin,
            ])
            .status()
            .is_ok_and(|st| st.success());
        if !has_host_bin {
            return false;
        }
        return Command::new("flatpak-spawn")
            .arg("--host")
            .arg(format!("--directory={}", path.to_string_lossy()))
            .arg(bin)
            .spawn()
            .is_ok();
    }
    Command::new(bin).current_dir(path).spawn().is_ok()
}

pub fn open_default(path: &Path) -> Result<(), FsError> {
    open::that_detached(path).map_err(|e| map_io_err(path, e))
}

/// Solicita a janela nativa de Propriedades via D-Bus `org.freedesktop.FileManager1` quando disponível.
pub fn open_properties(path: &Path) -> Result<(), FsError> {
    let uri = format!("file://{}", path.to_string_lossy());
    if let Ok(st) = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.FileManager1",
            "--object-path",
            "/org/freedesktop/FileManager1",
            "--method",
            "org.freedesktop.FileManager1.ShowItemProperties",
            &format!("['{uri}']"),
            "",
        ])
        .status()
        && st.success()
    {
        return Ok(());
    }

    if let Ok(st) = Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItemProperties",
            &format!("array:string:{uri}"),
            "string:",
        ])
        .status()
        && st.success()
    {
        return Ok(());
    }

    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "nenhum serviço org.freedesktop.FileManager1 ativo; use o Inspetor (Alt+I)"
            .to_owned(),
    })
}

/// Aplica alterações de permissões POSIX / modo somente leitura no Linux.
pub fn apply_entry_properties(path: &Path, patch: &PropertyPatch) -> Result<(), FsError> {
    if let Some(mode) = patch.posix_mode {
        let perms = std::fs::Permissions::from_mode(mode & 0o7777);
        std::fs::set_permissions(path, perms).map_err(|e| map_io_err(path, e))?;
        return Ok(());
    }
    if let Some(readonly) = patch.readonly {
        let meta = std::fs::metadata(path).map_err(|e| map_io_err(path, e))?;
        let mut perms = meta.permissions();
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms).map_err(|e| map_io_err(path, e))?;
    }
    Ok(())
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
    let probe_path = if path == Path::new("/") && std::env::var_os("FLATPAK_ID").is_some() {
        [
            Path::new("/run/host/os-release"),
            Path::new("/run/host/etc"),
        ]
        .into_iter()
        .find(|candidate| candidate.exists())
        .unwrap_or(path)
    } else {
        path
    };
    let stat = rustix::fs::statvfs(probe_path).ok()?;
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

pub(crate) fn host_command(program: &str) -> Command {
    if std::env::var_os("FLATPAK_ID").is_some() {
        let mut cmd = Command::new("flatpak-spawn");
        cmd.arg("--host").arg(program);
        cmd
    } else {
        Command::new(program)
    }
}

fn is_ignored_system_mount(mount_point: &Path) -> bool {
    mount_point.starts_with("/boot")
        || mount_point.starts_with("/snap")
        || mount_point.starts_with("/sys")
        || mount_point.starts_with("/proc")
        || mount_point.starts_with("/dev")
}

/// Filtra pontos de montagem internos de SO/container/Flatpak (bwrap) na varredura de fallback.
fn is_ignored_fallback_path(mount_point: &Path) -> bool {
    if is_ignored_system_mount(mount_point) {
        return true;
    }
    if matches!(
        mount_point.to_string_lossy().as_ref(),
        "/" | "/media" | "/mnt" | "/run" | "/run/media" | "/run/user"
    ) {
        return true;
    }

    const IGNORED_PREFIXES: &[&str] = &[
        "/usr",
        "/app",
        "/etc",
        "/var",
        "/lib",
        "/lib32",
        "/lib64",
        "/bin",
        "/sbin",
        "/opt",
        "/srv",
        "/root",
        "/tmp",
        "/home",
        "/cdrom",
        "/lost+found",
        "/timeshift",
        "/sysroot",
        "/run/host",
        "/run/flatpak",
        "/run/docker",
        "/run/containerd",
        "/run/netns",
    ];
    if IGNORED_PREFIXES
        .iter()
        .any(|prefix| mount_point.starts_with(prefix))
    {
        return true;
    }

    mount_point
        .components()
        .nth(1)
        .and_then(|comp| comp.as_os_str().to_str())
        .is_some_and(|first| first.ends_with(".usr-is-merged"))
}

fn should_include_fallback_mount(
    mount: &MountEntry,
    seen_devices: &HashSet<PathBuf>,
    seen_mounts: &HashSet<PathBuf>,
) -> bool {
    if IGNORED_FS_TYPES.contains(&mount.fs_type.as_str())
        || is_ignored_fallback_path(&mount.mount_point)
        || seen_mounts.contains(&mount.mount_point)
    {
        return false;
    }
    if mount.device.starts_with("/dev/") && seen_devices.contains(Path::new(&mount.device)) {
        return false;
    }
    true
}

/// No Flatpak, `/` aponta para o `tmpfs` do sandbox; consulta um bind-mount de `/run/host` do mesmo dispositivo.
fn space_for_lsblk_mount(
    mount_point: &Path,
    device_path: &Path,
    mounts: &[MountEntry],
) -> Option<VolumeSpace> {
    if mount_point == Path::new("/") && std::env::var_os("FLATPAK_ID").is_some() {
        let dev_str = device_path.to_string_lossy();
        if let Some(host_probe) = mounts.iter().find(|m| {
            m.mount_point.starts_with("/run/host/") && m.device.as_str() == dev_str.as_ref()
        }) && let Some(mut space) = volume_space(&host_probe.mount_point)
        {
            space.label = CompactString::new("Root (/)");
            return Some(space);
        }
    }
    volume_space(mount_point)
}

fn is_critical_os_mount(mount_point: &Path) -> bool {
    mount_point == Path::new("/")
        || matches!(
            mount_point.to_string_lossy().as_ref(),
            "/home" | "/var" | "/usr" | "/etc" | "/opt" | "/srv" | "/root"
        )
}

pub fn list_drives() -> Vec<DriveInfo> {
    let mounts = read_proc_mounts();
    let mut seen_mounts = HashSet::new();
    let mut seen_devices = HashSet::new();
    let mut drives = Vec::new();

    for dev in read_lsblk_devices() {
        if !matches!(dev.dev_type.as_str(), "part" | "disk" | "rom" | "crypt") {
            continue;
        }
        if dev.fstype.is_empty()
            || IGNORED_FS_TYPES.contains(&dev.fstype.as_str())
            || matches!(
                dev.fstype.as_str(),
                "swap" | "linux_raid_member" | "LVM2_member" | "crypto_LUKS"
            )
        {
            continue;
        }

        if let Some(mount_point) = dev.mountpoint.as_ref() {
            if is_ignored_system_mount(mount_point) {
                continue;
            }
            if !seen_mounts.insert(mount_point.clone()) {
                continue;
            }
            let space = space_for_lsblk_mount(mount_point, &dev.path, &mounts);
            if space.is_none() {
                continue;
            }
            seen_devices.insert(dev.path.clone());
            let mut kind = classify_drive(mount_point, &dev.fstype);
            if (dev.removable || dev.hotplug) && mount_point != Path::new("/") {
                kind = DriveKind::Removable;
            }
            let label = if mount_point == Path::new("/") {
                CompactString::new("Root (/)")
            } else if !dev.label.is_empty() {
                CompactString::from(dev.label.as_str())
            } else {
                drive_label(mount_point, kind)
            };
            let total_bytes_hint = space.as_ref().map(|s| s.total_bytes).or(dev.size_bytes);
            drives.push(DriveInfo {
                label,
                mount_path: mount_point.clone(),
                device_path: Some(dev.path),
                fs_type: Some(CompactString::from(dev.fstype.as_str())),
                kind,
                is_mounted: true,
                can_unmount: !is_critical_os_mount(mount_point),
                total_bytes_hint,
                space,
            });
        } else {
            if !seen_devices.insert(dev.path.clone()) {
                continue;
            }
            let kind = if dev.removable || dev.hotplug {
                DriveKind::Removable
            } else {
                DriveKind::SecondaryDisk
            };
            let label = if !dev.label.is_empty() {
                CompactString::from(dev.label.as_str())
            } else {
                CompactString::from(dev.name.as_str())
            };
            drives.push(DriveInfo {
                label,
                mount_path: PathBuf::new(),
                device_path: Some(dev.path),
                fs_type: Some(CompactString::from(dev.fstype.as_str())),
                kind,
                is_mounted: false,
                can_unmount: false,
                total_bytes_hint: dev.size_bytes,
                space: None,
            });
        }
    }

    // Completa com montagens virtuais/rede/GVFS presentes em `/proc/mounts` que não vêm do `lsblk`.
    for mount in mounts {
        if !should_include_fallback_mount(&mount, &seen_devices, &seen_mounts)
            || !mount.mount_point.is_dir()
        {
            continue;
        }
        seen_mounts.insert(mount.mount_point.clone());

        let space = volume_space(&mount.mount_point);
        if space.is_none() {
            continue;
        }

        let kind = classify_drive(&mount.mount_point, &mount.fs_type);
        let label = drive_label(&mount.mount_point, kind);
        let dev_path = mount
            .device
            .starts_with("/dev/")
            .then(|| PathBuf::from(&mount.device));
        drives.push(DriveInfo {
            label,
            mount_path: mount.mount_point.clone(),
            device_path: dev_path,
            fs_type: Some(mount.fs_type),
            kind,
            is_mounted: true,
            can_unmount: !is_critical_os_mount(&mount.mount_point),
            total_bytes_hint: space.as_ref().map(|s| s.total_bytes),
            space,
        });
    }

    drives
}

/// Monta dispositivo de bloco via `udisksctl` (ou `gio mount -d`) e retorna o ponto de montagem.
pub fn mount_drive(device_path: &Path) -> Result<PathBuf, FsError> {
    let dev_str = device_path.to_string_lossy().into_owned();
    let output = host_command("udisksctl")
        .args(["mount", "-b", &dev_str])
        .output()
        .map_err(|e| FsError::Io {
            path: device_path.to_path_buf(),
            message: format!("falha ao executar udisksctl: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let msg = if stderr.is_empty() {
            format!("udisksctl mount retornou código {:?}", output.status.code())
        } else {
            stderr
        };
        return Err(FsError::Io {
            path: device_path.to_path_buf(),
            message: msg,
        });
    }

    if let Some(found) = find_mount_point_for_device(device_path) {
        return Ok(found);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(idx) = stdout.rfind(" at ") {
        let candidate = stdout[idx + 4..].trim().trim_end_matches('.');
        if !candidate.is_empty() {
            return Ok(PathBuf::from(candidate));
        }
    }

    Err(FsError::Io {
        path: device_path.to_path_buf(),
        message: "dispositivo montado, mas ponto de montagem não localizado".to_owned(),
    })
}

/// Desmonta dispositivo ou ponto de montagem via `udisksctl`, `gio mount -u` ou `umount`.
pub fn unmount_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    let target_err_path = mount_path
        .or(device_path)
        .unwrap_or_else(|| Path::new("/"))
        .to_path_buf();

    if let Some(dev) = device_path
        && dev.to_string_lossy().starts_with("/dev/")
    {
        let dev_str = dev.to_string_lossy().into_owned();
        if let Ok(output) = host_command("udisksctl")
            .args(["unmount", "-b", &dev_str])
            .output()
        {
            if output.status.success() {
                return Ok(());
            }
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if mount_path.is_none() {
                return Err(FsError::Io {
                    path: target_err_path,
                    message: if stderr.is_empty() {
                        "falha ao desmontar unidade via udisksctl".to_owned()
                    } else {
                        stderr
                    },
                });
            }
        }
    }

    if let Some(mp) = mount_path {
        let mp_str = mp.to_string_lossy().into_owned();
        if let Ok(output) = host_command("gio").args(["mount", "-u", &mp_str]).output()
            && output.status.success()
        {
            return Ok(());
        }
        let output = host_command("umount")
            .arg(&mp_str)
            .output()
            .map_err(|e| FsError::Io {
                path: target_err_path.clone(),
                message: format!("falha ao desmontar unidade: {e}"),
            })?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(FsError::Io {
            path: target_err_path,
            message: if stderr.is_empty() {
                format!("umount retornou código {:?}", output.status.code())
            } else {
                stderr
            },
        });
    }

    Err(FsError::Io {
        path: target_err_path,
        message: "nenhum dispositivo ou ponto de montagem informado".to_owned(),
    })
}

pub fn supports_open_as_root() -> bool {
    true
}

pub fn is_elevated_session() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .is_some_and(|raw| parse_euid_root_from_proc_status(&raw))
}

fn parse_euid_root_from_proc_status(raw: &str) -> bool {
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let mut cols = rest.split_whitespace();
            let _ruid = cols.next();
            if let Some(euid) = cols.next() {
                return euid == "0";
            }
        }
    }
    false
}

/// Abre nova janela do gerenciador de arquivos com privilégios root via Polkit (`pkexec`), preservando sessão gráfica.
pub fn open_as_root(path: &Path) -> Result<(), FsError> {
    let target_dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    let exe = std::env::current_exe().map_err(|e| map_io_err(target_dir, e))?;

    if is_elevated_session() {
        Command::new(&exe)
            .arg(target_dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| map_io_err(target_dir, e))?;
        return Ok(());
    }

    let mut cmd = host_command("pkexec");
    cmd.arg("/usr/bin/env");

    if let Some(display) = std::env::var_os("DISPLAY") {
        cmd.arg(format!("DISPLAY={}", display.to_string_lossy()));
        if let Some(xauth) = std::env::var_os("XAUTHORITY") {
            cmd.arg(format!("XAUTHORITY={}", xauth.to_string_lossy()));
        } else if let Some(home) = std::env::var_os("HOME") {
            let default_xauth = PathBuf::from(home).join(".Xauthority");
            if default_xauth.exists() {
                cmd.arg(format!("XAUTHORITY={}", default_xauth.to_string_lossy()));
            }
        }
    } else if let Some(wayland) = std::env::var_os("WAYLAND_DISPLAY") {
        cmd.arg(format!("WAYLAND_DISPLAY={}", wayland.to_string_lossy()));
        if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            cmd.arg(format!("XDG_RUNTIME_DIR={}", runtime_dir.to_string_lossy()));
        }
    }

    for var in ["LANG", "LC_ALL"] {
        if let Some(val) = std::env::var_os(var) {
            cmd.arg(format!("{var}={}", val.to_string_lossy()));
        }
    }

    cmd.arg(&exe).arg(target_dir);
    cmd.spawn().map(|_| ()).map_err(|e| FsError::Io {
        path: target_dir.to_path_buf(),
        message: format!("falha ao iniciar pkexec: {e}"),
    })
}

fn find_mount_point_for_device(device_path: &Path) -> Option<PathBuf> {
    let target_canon = std::fs::canonicalize(device_path).ok();
    for dev in read_lsblk_devices() {
        let same_path = dev.path == device_path
            || target_canon
                .as_ref()
                .is_some_and(|c| std::fs::canonicalize(&dev.path).ok().as_ref() == Some(c));
        if same_path && let Some(mp) = dev.mountpoint {
            return Some(mp);
        }
    }
    read_proc_mounts().into_iter().find_map(|m| {
        let mp_dev = Path::new(m.device.as_str());
        let same = mp_dev == device_path
            || target_canon
                .as_ref()
                .is_some_and(|c| std::fs::canonicalize(mp_dev).ok().as_ref() == Some(c));
        same.then_some(m.mount_point)
    })
}

struct LsblkDevice {
    name: String,
    path: PathBuf,
    label: String,
    fstype: String,
    size_bytes: Option<u64>,
    mountpoint: Option<PathBuf>,
    removable: bool,
    hotplug: bool,
    dev_type: String,
}

fn read_lsblk_devices() -> Vec<LsblkDevice> {
    let Ok(output) = host_command("lsblk")
        .args([
            "-P",
            "-b",
            "-o",
            "NAME,PATH,LABEL,FSTYPE,SIZE,MOUNTPOINT,RM,HOTPLUG,TYPE",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().filter_map(parse_lsblk_line).collect()
}

fn parse_lsblk_line(line: &str) -> Option<LsblkDevice> {
    let pairs = parse_key_value_pairs(line);
    let get = |k: &str| -> String {
        pairs
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, val)| val.clone())
            .unwrap_or_default()
    };

    let name = get("NAME");
    let path_str = get("PATH");
    if name.is_empty() || path_str.is_empty() {
        return None;
    }
    let mount_raw = get("MOUNTPOINT");
    let mountpoint = (!mount_raw.is_empty()).then(|| PathBuf::from(mount_raw));

    Some(LsblkDevice {
        name,
        path: PathBuf::from(path_str),
        label: get("LABEL"),
        fstype: get("FSTYPE"),
        size_bytes: get("SIZE").parse::<u64>().ok(),
        mountpoint,
        removable: get("RM") == "1",
        hotplug: get("HOTPLUG") == "1",
        dev_type: get("TYPE"),
    })
}

fn parse_key_value_pairs(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            break;
        }
        let key = line[key_start..i].to_owned();
        i += 1;
        if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let mut val = String::new();
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    if bytes[i + 1] == b'x'
                        && i + 3 < bytes.len()
                        && let Ok(hex_str) = std::str::from_utf8(&bytes[i + 2..i + 4])
                        && let Ok(byte_val) = u8::from_str_radix(hex_str, 16)
                    {
                        val.push(byte_val as char);
                        i += 4;
                        continue;
                    }
                    val.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                let ch_slice = &line[i..];
                if let Some(ch) = ch_slice.chars().next() {
                    val.push(ch);
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }
            if i < bytes.len() && bytes[i] == b'"' {
                i += 1;
            }
            out.push((key, val));
        }
    }
    out
}

struct MountEntry {
    device: String,
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
            let device = parts.next()?.to_owned();
            let mount_raw = parts.next()?;
            let fs_type = parts.next()?;
            Some(MountEntry {
                device,
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
            device: m.device.clone(),
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

    #[test]
    fn parses_lsblk_key_value_line() {
        let line = r#"NAME="sda1" PATH="/dev/sda1" LABEL="JOGOS WIN" FSTYPE="ntfs" SIZE="480102055936" MOUNTPOINT="" RM="0" HOTPLUG="1" TYPE="part""#;
        let parsed = parse_lsblk_line(line).expect("lsblk parsed");
        assert_eq!(parsed.name, "sda1");
        assert_eq!(parsed.path, Path::new("/dev/sda1"));
        assert_eq!(parsed.label, "JOGOS WIN");
        assert_eq!(parsed.fstype, "ntfs");
        assert_eq!(parsed.size_bytes, Some(480_102_055_936));
        assert!(parsed.mountpoint.is_none());
        assert!(parsed.hotplug);
        assert_eq!(parsed.dev_type, "part");
    }

    #[test]
    fn detects_root_euid_from_proc_status() {
        let root_status = "Name:\tvagnarok\nUid:\t1000\t0\t0\t0\n";
        let user_status = "Name:\tvagnarok\nUid:\t1000\t1000\t1000\t1000\n";
        assert!(parse_euid_root_from_proc_status(root_status));
        assert!(!parse_euid_root_from_proc_status(user_status));
    }

    #[test]
    fn filters_flatpak_bind_mounts_and_system_paths_from_fallback() {
        let mut seen_devices = HashSet::new();
        seen_devices.insert(PathBuf::from("/dev/nvme0n1p4"));
        let mut seen_mounts = HashSet::new();
        seen_mounts.insert(PathBuf::from("/"));

        let sandbox_mounts = [
            ("/dev/nvme0n1p4", "/usr", "ext4"),
            ("/dev/nvme0n1p4", "/app", "ext4"),
            (
                "/dev/nvme0n1p4",
                "/usr/lib/x86_64-linux-gnu/GL/nvidia-580-178-04",
                "ext4",
            ),
            ("/dev/nvme0n1p4", "/etc/machine-id", "ext4"),
            ("/dev/nvme0n1p4", "/bin.usr-is-merged", "ext4"),
            ("/dev/nvme0n1p4", "/run/host/usr", "ext4"),
            ("/dev/nvme0n1p4", "/home", "ext4"),
            ("/dev/nvme0n1p4", "/media/vagnarok/BindCopy", "ext4"),
        ];
        for (dev, mp, fs) in sandbox_mounts {
            let entry = MountEntry {
                device: dev.to_owned(),
                mount_point: PathBuf::from(mp),
                fs_type: CompactString::new(fs),
            };
            assert!(
                !should_include_fallback_mount(&entry, &seen_devices, &seen_mounts),
                "bind-mount interno não deveria aparecer como drive: {mp}"
            );
        }

        let valid_mounts = [
            ("//nas/media", "/mnt/nas", "cifs"),
            (
                "sshfs#user@host:/",
                "/run/user/1000/gvfs/sftp:host",
                "fuse.sshfs",
            ),
            ("/dev/sde1", "/media/vagnarok/PENDRIVE", "vfat"),
        ];
        for (dev, mp, fs) in valid_mounts {
            let entry = MountEntry {
                device: dev.to_owned(),
                mount_point: PathBuf::from(mp),
                fs_type: CompactString::new(fs),
            };
            assert!(
                should_include_fallback_mount(&entry, &seen_devices, &seen_mounts),
                "montagem válida deveria ser incluída no fallback: {mp}"
            );
        }
    }
}
