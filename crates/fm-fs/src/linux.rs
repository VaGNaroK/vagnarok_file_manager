use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::{File, Metadata};
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
        let mut cmd = Command::new("flatpak-spawn");
        cmd.arg("--host")
            .arg(format!("--directory={}", path.to_string_lossy()))
            .arg(bin);
        return spawn_detached(&mut cmd).is_ok();
    }
    let mut cmd = Command::new(bin);
    cmd.current_dir(path);
    spawn_detached(&mut cmd).is_ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectLaunchKind {
    NotExecutable,
    ExecuteDirectly,
    AppImageMissingExecBit,
}

fn is_appimage_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("appimage"))
}

fn has_elf_magic(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).is_ok() && magic == *b"\x7fELF"
}

/// Classifica se o arquivo deve ser executado diretamente (AppImage ou binário ELF com `+x`).
/// Arquivos comuns com bit `+x` (ex.: montagens FAT32/NTFS) continuam abrindo via MIME/sistema.
fn classify_linux_launch(path: &Path) -> DirectLaunchKind {
    let Ok(meta) = std::fs::metadata(path) else {
        return DirectLaunchKind::NotExecutable;
    };
    if !meta.is_file() {
        return DirectLaunchKind::NotExecutable;
    }
    let is_exec = fm_core::is_posix_executable(meta.permissions().mode());
    let is_appimage = is_appimage_path(path);
    if is_appimage && !is_exec {
        return DirectLaunchKind::AppImageMissingExecBit;
    }
    if is_exec && (is_appimage || has_elf_magic(path)) {
        return DirectLaunchKind::ExecuteDirectly;
    }
    DirectLaunchKind::NotExecutable
}

fn spawn_executable_detached(path: &Path) -> Result<(), FsError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut cmd = if std::env::var_os("FLATPAK_ID").is_some() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host");
        if let Some(dir) = parent {
            let mut dir_arg = OsString::from("--directory=");
            dir_arg.push(dir.as_os_str());
            c.arg(dir_arg);
        }
        c.arg("--").arg(path);
        c
    } else {
        let mut c = Command::new(path);
        if let Some(dir) = parent {
            c.current_dir(dir);
        }
        c
    };
    spawn_detached(&mut cmd).map_err(|e| map_io_err(path, e))
}

pub fn open_default(path: &Path) -> Result<(), FsError> {
    match classify_linux_launch(path) {
        DirectLaunchKind::ExecuteDirectly => return spawn_executable_detached(path),
        DirectLaunchKind::AppImageMissingExecBit => {
            return Err(FsError::Io {
                path: path.to_path_buf(),
                message:
                    "AppImage sem permissão de execução — ative 'Permitir execução' em Propriedades"
                        .to_owned(),
            });
        }
        DirectLaunchKind::NotExecutable => {}
    }

    if std::env::var_os("FLATPAK_ID").is_some() {
        // No Flatpak, `open::that_detached` aciona o shim `/usr/bin/xdg-open` do sandbox
        // (org.freedesktop.portal.OpenURI), que ignora `~/.config/mimeapps.list` e abre
        // repetidamente o diálogo AppChooser. Despachamos para o `gio open` / `xdg-open`
        // do host desacoplado para respeitar os aplicativos padrão reais do sistema.
        let mut gio = host_command("gio");
        gio.arg("open").arg(path);
        if spawn_detached(&mut gio).is_ok() {
            return Ok(());
        }
        let mut xdg = host_command("xdg-open");
        xdg.arg(path);
        if spawn_detached(&mut xdg).is_ok() {
            return Ok(());
        }
    } else {
        let mut gio = Command::new("gio");
        gio.arg("open").arg(path);
        if spawn_detached(&mut gio).is_ok() {
            return Ok(());
        }
        let mut xdg = Command::new("xdg-open");
        xdg.arg(path);
        if spawn_detached(&mut xdg).is_ok() {
            return Ok(());
        }
    }
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

/// Aplica alterações de permissões POSIX / modo somente leitura no Linux (com suporte a recursividade em pastas).
pub fn apply_entry_properties(path: &Path, patch: &PropertyPatch) -> Result<(), FsError> {
    if let Some(mode) = patch.posix_mode {
        let masked = mode & 0o7777;
        let perms = std::fs::Permissions::from_mode(masked);
        std::fs::set_permissions(path, perms).map_err(|e| map_io_err(path, e))?;
        if patch.recursive && path.is_dir() {
            apply_posix_mode_recursive(path, masked);
        }
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

fn apply_posix_mode_recursive(root: &Path, mode: u32) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for item in entries.flatten() {
            let child = item.path();
            let Ok(ft) = item.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let _ = std::fs::set_permissions(&child, std::fs::Permissions::from_mode(mode));
            if ft.is_dir() {
                stack.push(child);
            }
        }
    }
}

/// Cria um link simbólico POSIX apontando para `target` dentro de `dest_dir` sem sobrescrever entradas existentes.
pub fn create_shortcut(target: &Path, dest_dir: &Path) -> Result<PathBuf, FsError> {
    let link_meta = std::fs::symlink_metadata(target).map_err(|e| map_io_err(target, e))?;
    let is_dir = if link_meta.file_type().is_symlink() {
        std::fs::metadata(target)
            .map(|m| m.is_dir())
            .unwrap_or(false)
    } else {
        link_meta.is_dir()
    };

    let abs_target = resolve_absolute_target(target);
    let same_dir = target
        .parent()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .zip(std::fs::canonicalize(dest_dir).ok())
        .is_some_and(|(a, b)| a == b);

    for attempt in 0..10_000u32 {
        let candidate_name =
            fm_core::suggest_shortcut_name(&abs_target, is_dir, false, attempt, same_dir);
        let candidate = dest_dir.join(&candidate_name);
        if std::fs::symlink_metadata(&candidate).is_ok() {
            continue;
        }
        match std::os::unix::fs::symlink(&abs_target, &candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(map_io_err(&candidate, err)),
        }
    }

    Err(FsError::Io {
        path: dest_dir.to_path_buf(),
        message: "não foi possível encontrar um nome livre para o link simbólico".to_owned(),
    })
}

/// Resolve caminho absoluto preservando o componente final (não segue symlink do próprio alvo).
fn resolve_absolute_target(target: &Path) -> PathBuf {
    if let (Some(parent), Some(file_name)) = (target.parent(), target.file_name()) {
        let abs_parent = if parent.as_os_str().is_empty() {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            std::fs::canonicalize(parent).unwrap_or_else(|_| {
                if parent.is_absolute() {
                    parent.to_path_buf()
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(parent))
                        .unwrap_or_else(|_| parent.to_path_buf())
                }
            })
        };
        return abs_parent.join(file_name);
    }
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(target))
            .unwrap_or_else(|_| target.to_path_buf())
    }
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
        cmd.arg("--host").arg("--").arg(program);
        cmd
    } else {
        Command::new(program)
    }
}

/// Inicia um processo externo completamente desacoplado da sessão e do grupo de processos do app.
/// Garante que aplicativos abertos (VLC, editores, terminais, etc.) sobrevivam ao fechamento do gerenciador.
pub(crate) fn spawn_detached(cmd: &mut Command) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // SAFETY: `setsid()` é async-signal-safe segundo POSIX e não acessa estado compartilhado do processo pai.
    unsafe {
        cmd.pre_exec(|| {
            let _ = rustix::process::setsid();
            Ok(())
        });
    }

    cmd.spawn().map(|_| ())
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

    let raw_lsblk = read_lsblk_devices();
    let disks_with_children: HashSet<String> = raw_lsblk
        .iter()
        .filter(|d| !d.pkname.is_empty())
        .map(|d| d.pkname.clone())
        .collect();
    let disk_models: std::collections::HashMap<String, String> = raw_lsblk
        .iter()
        .filter(|d| !d.model.is_empty())
        .map(|d| (d.name.clone(), d.model.clone()))
        .collect();

    for dev in raw_lsblk {
        // Blindagem: ignora o dispositivo pai `disk` quando ele possui partições filhas
        // (evita duplicidade real em pendrives híbridos / ISOs gravadas com `dd`).
        if should_skip_parent_disk(&dev, &disks_with_children) {
            continue;
        }
        let is_loop_backed = dev.dev_type == "loop" || dev.pkname.starts_with("loop");
        if !matches!(
            dev.dev_type.as_str(),
            "part" | "disk" | "rom" | "crypt" | "loop"
        ) {
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
        // Ignora loops de sistema/Snap sem ponto de montagem ou que apontem para /snap / /var/lib/snapd.
        if is_loop_backed
            && dev.mountpoint.as_ref().is_some_and(|mp| {
                mp.starts_with("/snap")
                    || mp.starts_with("/var/lib/snapd")
                    || mp.starts_with("/run/flatpak")
            })
        {
            continue;
        }
        if dev.dev_type == "loop" && dev.mountpoint.is_none() {
            continue;
        }

        let is_disk_image = is_loop_backed || matches!(dev.fstype.as_str(), "iso9660" | "udf");
        let model = resolve_device_model(&dev, &disk_models);

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
            if dev.dev_type == "rom" || is_disk_image {
                kind = DriveKind::OpticalOrIso;
            } else if (dev.removable || dev.hotplug) && mount_point != Path::new("/") {
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
            let is_ejectable = (kind == DriveKind::Removable || dev.dev_type == "rom")
                && !is_disk_image
                && mount_point != Path::new("/");
            drives.push(DriveInfo {
                label,
                mount_path: mount_point.clone(),
                device_path: Some(dev.path),
                model,
                fs_type: Some(CompactString::from(dev.fstype.as_str())),
                kind,
                is_mounted: true,
                can_unmount: !is_critical_os_mount(mount_point),
                is_ejectable,
                is_disk_image,
                total_bytes_hint,
                space,
            });
        } else {
            if !seen_devices.insert(dev.path.clone()) {
                continue;
            }
            let kind = if dev.dev_type == "rom" || is_disk_image {
                DriveKind::OpticalOrIso
            } else if dev.removable || dev.hotplug {
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
                model,
                fs_type: Some(CompactString::from(dev.fstype.as_str())),
                kind,
                is_mounted: false,
                can_unmount: false,
                is_ejectable: (kind == DriveKind::Removable || dev.dev_type == "rom")
                    && !is_disk_image,
                is_disk_image,
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

        let is_disk_image = mount.device.starts_with("/dev/loop")
            || matches!(mount.fs_type.as_str(), "iso9660" | "udf");
        let kind = if is_disk_image {
            DriveKind::OpticalOrIso
        } else {
            classify_drive(&mount.mount_point, &mount.fs_type)
        };
        let label = drive_label(&mount.mount_point, kind);
        let dev_path = mount
            .device
            .starts_with("/dev/")
            .then(|| PathBuf::from(&mount.device));
        drives.push(DriveInfo {
            label,
            mount_path: mount.mount_point.clone(),
            device_path: dev_path,
            model: None,
            fs_type: Some(mount.fs_type),
            kind,
            is_mounted: true,
            can_unmount: !is_critical_os_mount(&mount.mount_point),
            is_ejectable: kind == DriveKind::Removable && !is_disk_image,
            is_disk_image,
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
/// Quando o dispositivo é um `loop` (`.iso`/`.img`), também limpa o loop device com `udisksctl loop-delete`.
pub fn unmount_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    let target_err_path = mount_path
        .or(device_path)
        .unwrap_or_else(|| Path::new("/"))
        .to_path_buf();

    let resolved_dev = device_path
        .map(Path::to_path_buf)
        .or_else(|| mount_path.and_then(find_device_for_mount_point));

    let mut unmounted = false;
    if let Some(dev) = resolved_dev.as_deref()
        && dev.to_string_lossy().starts_with("/dev/")
    {
        let dev_str = dev.to_string_lossy().into_owned();
        if let Ok(output) = host_command("udisksctl")
            .args(["unmount", "-b", &dev_str])
            .output()
        {
            if output.status.success() {
                unmounted = true;
            } else if mount_path.is_none() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
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

    if !unmounted {
        if let Some(mp) = mount_path {
            let mp_str = mp.to_string_lossy().into_owned();
            if let Ok(output) = host_command("gio").args(["mount", "-u", &mp_str]).output()
                && output.status.success()
            {
                unmounted = true;
            }
            if !unmounted {
                let output =
                    host_command("umount")
                        .arg(&mp_str)
                        .output()
                        .map_err(|e| FsError::Io {
                            path: target_err_path.clone(),
                            message: format!("falha ao desmontar unidade: {e}"),
                        })?;
                if !output.status.success() {
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
            }
        } else {
            return Err(FsError::Io {
                path: target_err_path,
                message: "nenhum dispositivo ou ponto de montagem informado".to_owned(),
            });
        }
    }

    if let Some(dev) = resolved_dev.as_deref()
        && let Some(loop_root) = loop_root_device(dev)
    {
        cleanup_loop_device_if_idle(&loop_root);
    }

    Ok(())
}

/// Monta imagem `.iso` ou `.img` em modo somente leitura via `udisksctl loop-setup` + `udisksctl mount`.
pub fn mount_disk_image(image_path: &Path) -> Result<PathBuf, FsError> {
    if !image_path.is_file() {
        return Err(FsError::NotFound(image_path.to_path_buf()));
    }
    if !fm_core::is_mountable_disk_image(image_path) {
        return Err(FsError::Io {
            path: image_path.to_path_buf(),
            message: "formato de imagem não suportado (use .iso ou .img)".to_owned(),
        });
    }

    let abs_image = std::fs::canonicalize(image_path).unwrap_or_else(|_| image_path.to_path_buf());
    let output = host_command("udisksctl")
        .args(["loop-setup", "-r", "-f"])
        .arg(&abs_image)
        .output()
        .map_err(|e| FsError::Io {
            path: image_path.to_path_buf(),
            message: format!("falha ao executar udisksctl loop-setup: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(FsError::Io {
            path: image_path.to_path_buf(),
            message: if stderr.is_empty() {
                format!(
                    "udisksctl loop-setup retornou código {:?}",
                    output.status.code()
                )
            } else {
                stderr
            },
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(loop_dev) = parse_udisks_loop_device(&stdout) else {
        return Err(FsError::Io {
            path: image_path.to_path_buf(),
            message: format!("não foi possível identificar o dispositivo loop em: {stdout}"),
        });
    };

    // Aguarda brevemente caso o ambiente gráfico já monte o dispositivo loop automaticamente
    // ou para o kernel enumerar partições internas de arquivos `.img`.
    for _ in 0..5 {
        if let Some(mp) = find_mount_point_for_loop(&loop_dev) {
            return Ok(mp);
        }
        std::thread::sleep(Duration::from_millis(160));
    }

    let mount_target =
        pick_mountable_target_for_loop(&loop_dev).unwrap_or_else(|| loop_dev.clone());
    match mount_drive(&mount_target) {
        Ok(mp) => Ok(mp),
        Err(err) => {
            if let Some(mp) = find_mount_point_for_loop(&loop_dev) {
                return Ok(mp);
            }
            let _ = delete_loop_device(&loop_dev);
            Err(err)
        }
    }
}

/// Remove mídia removível com segurança: desmonta todas as partições do disco pai e desliga energia (`udisksctl power-off`).
pub fn eject_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    let target_err_path = mount_path
        .or(device_path)
        .unwrap_or_else(|| Path::new("/"))
        .to_path_buf();

    let resolved_dev = device_path
        .map(Path::to_path_buf)
        .or_else(|| mount_path.and_then(find_device_for_mount_point));

    if let Some(dev) = resolved_dev.as_deref()
        && let Some(loop_root) = loop_root_device(dev)
    {
        unmount_drive(Some(dev), mount_path)?;
        let _ = delete_loop_device(&loop_root);
        return Ok(());
    }

    let lsblk_all = read_lsblk_devices();
    let parent_disk = resolved_dev
        .as_deref()
        .and_then(|dev| resolve_parent_disk(dev, &lsblk_all))
        .or_else(|| resolved_dev.clone());

    if let Some(ref disk) = parent_disk {
        let disk_name = disk
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut unmounted_any = false;
        for entry in &lsblk_all {
            if (entry.path == *disk || (!disk_name.is_empty() && entry.pkname == disk_name))
                && let Some(mp) = entry.mountpoint.as_deref()
            {
                unmount_drive(Some(&entry.path), Some(mp))?;
                unmounted_any = true;
            }
        }
        if !unmounted_any && mount_path.is_some() {
            unmount_drive(resolved_dev.as_deref(), mount_path)?;
        }

        let disk_str = disk.to_string_lossy().into_owned();
        if let Ok(output) = host_command("udisksctl")
            .args(["power-off", "-b", &disk_str])
            .output()
            && output.status.success()
        {
            return Ok(());
        }
        if disk_name.starts_with("sr")
            && let Ok(output) = host_command("eject").arg(&disk_str).output()
            && output.status.success()
        {
            return Ok(());
        }
        return Ok(());
    }

    if mount_path.is_some() {
        return unmount_drive(device_path, mount_path);
    }

    Err(FsError::Io {
        path: target_err_path,
        message: "não foi possível identificar o dispositivo para ejeção segura".to_owned(),
    })
}

/// Assinatura leve (sem fork de processos) do estado de montagens e dispositivos de bloco para hotplug.
pub fn drives_fingerprint() -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };

    if let Ok(mounts) = std::fs::read("/proc/mounts") {
        feed(&mounts);
    }
    if let Ok(entries) = std::fs::read_dir("/sys/block") {
        let mut names: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            feed(name.as_bytes());
            feed(b"\n");
        }
    }
    hash
}

fn find_device_for_mount_point(mount_point: &Path) -> Option<PathBuf> {
    read_proc_mounts().into_iter().find_map(|m| {
        (m.mount_point == mount_point && m.device.starts_with("/dev/"))
            .then(|| PathBuf::from(m.device))
    })
}

fn parse_udisks_loop_device(stdout: &str) -> Option<PathBuf> {
    for token in stdout.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| matches!(c, '.' | ',' | ';' | '\'' | '"'));
        if cleaned.starts_with("/dev/loop") {
            return Some(PathBuf::from(cleaned));
        }
    }
    None
}

fn loop_root_device(dev: &Path) -> Option<PathBuf> {
    let s = dev.to_str()?;
    let rest = s.strip_prefix("/dev/loop")?;
    let digits_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 {
        return None;
    }
    Some(PathBuf::from(format!("/dev/loop{}", &rest[..digits_len])))
}

fn find_mount_point_for_loop(loop_dev: &Path) -> Option<PathBuf> {
    let loop_name = loop_dev.file_name()?.to_string_lossy();
    for entry in read_lsblk_devices() {
        if (entry.path == loop_dev || entry.pkname == loop_name)
            && let Some(mp) = entry.mountpoint
        {
            return Some(mp);
        }
    }
    find_mount_point_for_device(loop_dev)
}

fn pick_mountable_target_for_loop(loop_dev: &Path) -> Option<PathBuf> {
    let loop_name = loop_dev.file_name()?.to_string_lossy();
    let devices = read_lsblk_devices();
    // Prefere partição filha com filesystem válido (ex.: imagens `.img` particionadas).
    for entry in &devices {
        if entry.pkname == loop_name
            && !entry.fstype.is_empty()
            && !IGNORED_FS_TYPES.contains(&entry.fstype.as_str())
            && entry.fstype != "swap"
        {
            return Some(entry.path.clone());
        }
    }
    // Caso contrário usa o próprio loop (ex.: `.iso` ou `.img` sem tabela de partições).
    devices
        .into_iter()
        .find(|entry| entry.path == loop_dev && !entry.fstype.is_empty())
        .map(|entry| entry.path)
}

fn cleanup_loop_device_if_idle(loop_root: &Path) {
    if find_mount_point_for_loop(loop_root).is_none() {
        let _ = delete_loop_device(loop_root);
    }
}

fn delete_loop_device(loop_root: &Path) -> Result<(), FsError> {
    let dev_str = loop_root.to_string_lossy().into_owned();
    let output = host_command("udisksctl")
        .args(["loop-delete", "-b", &dev_str])
        .output()
        .map_err(|e| FsError::Io {
            path: loop_root.to_path_buf(),
            message: format!("falha ao liberar dispositivo loop: {e}"),
        })?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(FsError::Io {
            path: loop_root.to_path_buf(),
            message: if stderr.is_empty() {
                "falha ao remover dispositivo loop".to_owned()
            } else {
                stderr
            },
        })
    }
}

fn resolve_parent_disk(dev: &Path, lsblk_all: &[LsblkDevice]) -> Option<PathBuf> {
    let entry = lsblk_all.iter().find(|d| d.path == dev)?;
    if !entry.pkname.is_empty() {
        let parent_path = PathBuf::from(format!("/dev/{}", entry.pkname));
        return Some(parent_path);
    }
    Some(entry.path.clone())
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
        let mut cmd = Command::new(&exe);
        cmd.arg(target_dir);
        spawn_detached(&mut cmd).map_err(|e| map_io_err(target_dir, e))?;
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
    spawn_detached(&mut cmd).map_err(|e| FsError::Io {
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

fn should_skip_parent_disk(dev: &LsblkDevice, disks_with_children: &HashSet<String>) -> bool {
    dev.dev_type == "disk" && disks_with_children.contains(&dev.name)
}

fn resolve_device_model(
    dev: &LsblkDevice,
    disk_models: &std::collections::HashMap<String, String>,
) -> Option<CompactString> {
    let raw = if !dev.model.is_empty() {
        Some(dev.model.as_str())
    } else if !dev.pkname.is_empty() {
        disk_models.get(&dev.pkname).map(String::as_str)
    } else {
        None
    };
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(CompactString::from)
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
    pkname: String,
    model: String,
}

fn read_lsblk_devices() -> Vec<LsblkDevice> {
    let Ok(output) = host_command("lsblk")
        .args([
            "-P",
            "-b",
            "-o",
            "NAME,PATH,LABEL,FSTYPE,SIZE,MOUNTPOINT,RM,HOTPLUG,TYPE,PKNAME,MODEL",
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
        pkname: get("PKNAME"),
        model: get("MODEL").trim().to_owned(),
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

    #[test]
    fn parses_udisks_loop_device_and_root_paths() {
        assert_eq!(
            parse_udisks_loop_device("Mapped file /home/u/ubuntu.iso as /dev/loop4."),
            Some(PathBuf::from("/dev/loop4"))
        );
        assert_eq!(parse_udisks_loop_device("nothing here"), None);
        assert_eq!(
            loop_root_device(Path::new("/dev/loop12p1")),
            Some(PathBuf::from("/dev/loop12"))
        );
        assert_eq!(
            loop_root_device(Path::new("/dev/loop3")),
            Some(PathBuf::from("/dev/loop3"))
        );
        assert_eq!(loop_root_device(Path::new("/dev/sdb1")), None);
    }

    #[test]
    fn skips_parent_disk_when_partitions_exist_and_inherits_model() {
        let disk = parse_lsblk_line(
            r#"NAME="sde" PATH="/dev/sde" LABEL="LIVEUSB" FSTYPE="iso9660" SIZE="16000000000" MOUNTPOINT="" RM="1" HOTPLUG="1" TYPE="disk" PKNAME="" MODEL="SanDisk Ultra   ""#,
        )
        .expect("parse disk");
        let part = parse_lsblk_line(
            r#"NAME="sde1" PATH="/dev/sde1" LABEL="LIVEUSB" FSTYPE="iso9660" SIZE="16000000000" MOUNTPOINT="" RM="1" HOTPLUG="1" TYPE="part" PKNAME="sde" MODEL="""#,
        )
        .expect("parse part");

        let mut disks_with_children = HashSet::new();
        disks_with_children.insert(part.pkname.clone());
        let mut models = std::collections::HashMap::new();
        models.insert(disk.name.clone(), disk.model.clone());

        assert!(should_skip_parent_disk(&disk, &disks_with_children));
        assert!(!should_skip_parent_disk(&part, &disks_with_children));
        assert_eq!(
            resolve_device_model(&part, &models).as_deref(),
            Some("SanDisk Ultra")
        );
    }

    #[test]
    fn spawn_detached_starts_child_in_independent_session() {
        let out_file =
            std::env::temp_dir().join(format!("vagnarok_detached_sid_{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&out_file);

        let parent_sid = rustix::process::getsid(None)
            .map(|p| p.as_raw_nonzero().get())
            .unwrap_or(0);

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg("awk '{print $6}' /proc/self/stat > \"$1\"")
            .arg("_")
            .arg(&out_file);
        spawn_detached(&mut cmd).expect("spawn_detached deve iniciar processo filho");

        let mut child_sid = None;
        for _ in 0..50 {
            if let Ok(raw) = std::fs::read_to_string(&out_file)
                && let Ok(parsed) = raw.trim().parse::<i32>()
            {
                child_sid = Some(parsed);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = std::fs::remove_file(&out_file);

        let sid = child_sid.expect("filho desacoplado deve gravar seu SID");
        assert_ne!(
            sid, parent_sid,
            "processo desacoplado deve rodar em uma nova sessão POSIX"
        );
    }

    #[test]
    fn classifies_appimage_and_elf_executables_without_executing_regular_docs() {
        let dir = std::env::temp_dir().join(format!("vagnarok_launch_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let appimage = dir.join("DemoApp.AppImage");
        std::fs::write(&appimage, b"\x7fELFdummy").expect("write appimage");
        std::fs::set_permissions(&appimage, std::fs::Permissions::from_mode(0o644))
            .expect("chmod 644");
        assert_eq!(
            classify_linux_launch(&appimage),
            DirectLaunchKind::AppImageMissingExecBit
        );
        assert!(open_default(&appimage).is_err());

        std::fs::set_permissions(&appimage, std::fs::Permissions::from_mode(0o755))
            .expect("chmod 755");
        assert_eq!(
            classify_linux_launch(&appimage),
            DirectLaunchKind::ExecuteDirectly
        );

        let elf_bin = dir.join("my_tool");
        std::fs::write(&elf_bin, b"\x7fELF\x02\x01\x01\x00").expect("write elf");
        std::fs::set_permissions(&elf_bin, std::fs::Permissions::from_mode(0o755))
            .expect("chmod 755");
        assert_eq!(
            classify_linux_launch(&elf_bin),
            DirectLaunchKind::ExecuteDirectly
        );

        // Arquivo comum com +x (ex.: partição FAT32/NTFS) não deve ser executado diretamente.
        let doc = dir.join("relatorio.pdf");
        std::fs::write(&doc, b"%PDF-1.7\n").expect("write pdf");
        std::fs::set_permissions(&doc, std::fs::Permissions::from_mode(0o755)).expect("chmod 755");
        assert_eq!(classify_linux_launch(&doc), DirectLaunchKind::NotExecutable);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
