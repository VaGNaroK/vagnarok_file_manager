use std::ffi::{OsString, c_void};
use std::fs::Metadata;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use compact_str::CompactString;
use fm_core::{
    DirectorySnapshot, DriveInfo, DriveKind, EntryKind, FsError, PermissionsMeta, VolumeSpace,
};
use tokio_util::sync::CancellationToken;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_FILES, ERROR_SUCCESS, GENERIC_READ, HANDLE, HWND,
    RPC_E_CHANGED_MODE, S_FALSE, S_OK,
};
use windows::Win32::Storage::FileSystem::{
    COPYFILE_FLAGS, COPYPROGRESSROUTINE_PROGRESS, CopyFileExW, CreateFileW,
    FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, FIND_FIRST_EX_LARGE_FETCH,
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    GetDiskFreeSpaceExW, GetDriveTypeW, GetFileAttributesW, GetLogicalDriveStringsW,
    GetLogicalDrives, GetVolumeInformationW, INVALID_FILE_ATTRIBUTES,
    LPPROGRESS_ROUTINE_CALLBACK_REASON, OPEN_EXISTING, PROGRESS_CANCEL, PROGRESS_CONTINUE,
    SetFileAttributesW, WIN32_FIND_DATAW,
};
use windows::Win32::Storage::Vhd::{
    ATTACH_VIRTUAL_DISK_FLAG_PERMANENT_LIFETIME, ATTACH_VIRTUAL_DISK_FLAG_READ_ONLY,
    AttachVirtualDisk, OPEN_VIRTUAL_DISK_FLAG_NONE, OpenVirtualDisk, VIRTUAL_DISK_ACCESS_ATTACH_RO,
    VIRTUAL_STORAGE_TYPE, VIRTUAL_STORAGE_TYPE_DEVICE_ISO, VIRTUAL_STORAGE_TYPE_VENDOR_MICROSOFT,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IPersistFile,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_STORAGE_EJECT_MEDIA,
};
use windows::Win32::UI::Shell::{
    IShellLinkW, SEE_MASK_FLAG_NO_UI, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW, ShellExecuteExW,
    ShellLink,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{BOOL, Interface, PCWSTR};

use crate::{PropertyPatch, make_file_entry, map_io_err};

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;
const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

struct FindHandle(HANDLE);

impl Drop for FindHandle {
    fn drop(&mut self) {
        // SAFETY: `self.0` é um handle válido retornado por `FindFirstFileExW`.
        let _ = unsafe { FindClose(self.0) };
    }
}

/// Lista diretório no Windows em lote via `FindFirstFileExW` (`FindExInfoBasic` + `FIND_FIRST_EX_LARGE_FETCH`).
pub fn scan_directory_native(
    path: &Path,
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let pattern = path.join("*");
    let wide = to_long_wide(&pattern);
    let mut find_data = WIN32_FIND_DATAW::default();

    // SAFETY: `wide` é null-terminated válido e `find_data` é um buffer válido na stack.
    let raw_handle = match unsafe {
        FindFirstFileExW(
            PCWSTR(wide.as_ptr()),
            FindExInfoBasic,
            (&raw mut find_data).cast(),
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        )
    } {
        Ok(h) => h,
        Err(err) => {
            if err.code() == ERROR_NO_MORE_FILES.into() {
                return Ok(DirectorySnapshot {
                    request_id,
                    path: path.to_path_buf(),
                    entries: Vec::new(),
                    fs_type: fs_type_for_path(path),
                });
            }
            let io_err = win_err_to_io(err);
            // Em raízes vazias no Windows, `FindFirstFileExW("X:\\*")` pode retornar `ERROR_FILE_NOT_FOUND` (2).
            if io_err.kind() == std::io::ErrorKind::NotFound && path.is_dir() {
                return Ok(DirectorySnapshot {
                    request_id,
                    path: path.to_path_buf(),
                    entries: Vec::new(),
                    fs_type: fs_type_for_path(path),
                });
            }
            return Err(map_io_err(path, io_err));
        }
    };

    let handle = FindHandle(raw_handle);
    let mut entries = Vec::new();
    let mut processed = 0usize;

    loop {
        if processed.is_multiple_of(256) && cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        processed += 1;

        let end = find_data
            .cFileName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(find_data.cFileName.len());
        let name_slice = &find_data.cFileName[..end];

        // Ignora entradas `.` e `..`.
        let is_dot = name_slice == [b'.' as u16] || name_slice == [b'.' as u16, b'.' as u16];
        if !is_dot && !name_slice.is_empty() {
            let name_os = OsString::from_wide(name_slice);
            let name = CompactString::from(name_os.to_string_lossy().as_ref());
            let entry_path = path.join(&name_os);

            let attrs = find_data.dwFileAttributes;
            let is_hidden = (attrs & FILE_ATTRIBUTE_HIDDEN) != 0;
            let is_reparse = (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
            let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0;

            let kind = if is_reparse {
                let target_is_dir = std::fs::metadata(&entry_path)
                    .map(|m| m.is_dir())
                    .unwrap_or(is_dir);
                EntryKind::Symlink { target_is_dir }
            } else if is_dir {
                EntryKind::Directory
            } else {
                EntryKind::File
            };

            let size_bytes =
                (u64::from(find_data.nFileSizeHigh) << 32) | u64::from(find_data.nFileSizeLow);
            let modified_at = filetime_to_system_time(
                find_data.ftLastWriteTime.dwHighDateTime,
                find_data.ftLastWriteTime.dwLowDateTime,
            );
            let created_at = filetime_to_system_time(
                find_data.ftCreationTime.dwHighDateTime,
                find_data.ftCreationTime.dwLowDateTime,
            );

            entries.push(make_file_entry(
                name,
                entry_path,
                kind,
                size_bytes,
                modified_at,
                created_at,
                PermissionsMeta::WinAttributes(attrs),
                is_hidden,
            ));
        }

        // SAFETY: `handle.0` é válido e `find_data` pertence ao frame atual.
        match unsafe { FindNextFileW(handle.0, &raw mut find_data) } {
            Ok(()) => {}
            Err(err) if err.code() == ERROR_NO_MORE_FILES.into() => break,
            Err(err) => return Err(map_io_err(path, win_err_to_io(err))),
        }
    }

    Ok(DirectorySnapshot {
        request_id,
        path: path.to_path_buf(),
        entries,
        fs_type: fs_type_for_path(path),
    })
}

fn filetime_to_system_time(high: u32, low: u32) -> Option<SystemTime> {
    const WINDOWS_TICK: u64 = 10_000_000;
    const SEC_TO_UNIX_EPOCH: u64 = 11_644_473_600;
    let ticks = (u64::from(high) << 32) | u64::from(low);
    let unix_ticks = ticks.checked_sub(SEC_TO_UNIX_EPOCH * WINDOWS_TICK)?;
    let secs = unix_ticks / WINDOWS_TICK;
    let rem_nanos = ((unix_ticks % WINDOWS_TICK) * 100) as u32;
    UNIX_EPOCH.checked_add(Duration::new(secs, rem_nanos))
}

/// Inicia um processo externo no Windows em um grupo de processos independente (desacoplado do Job atual quando permitido).
pub(crate) fn spawn_detached(cmd: &mut Command) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB);
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(err) if err.raw_os_error() == Some(5) => {
            // Job Object pai não permite breakaway; mantém apenas novo grupo de processos.
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
            cmd.spawn().map(|_| ())
        }
        Err(err) => Err(err),
    }
}

/// Abre Windows Terminal (`wt.exe`) ou fallback `cmd.exe` no diretório especificado.
pub fn open_terminal_at(path: &Path) -> Result<(), FsError> {
    let mut wt = Command::new("wt.exe");
    wt.arg("-d").arg(path);
    if spawn_detached(&mut wt).is_ok() {
        return Ok(());
    }
    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/C", "start", "cmd.exe"]).current_dir(path);
    spawn_detached(&mut cmd).map_err(|e| map_io_err(path, e))
}

pub fn open_default(path: &Path) -> Result<(), FsError> {
    shell_execute_verb(path, "open")
}

pub fn open_properties(path: &Path) -> Result<(), FsError> {
    shell_execute_verb(path, "properties")
}

fn shell_execute_verb(path: &Path, verb: &str) -> Result<(), FsError> {
    let verb_w = to_str_wide(verb);
    let file_w = to_long_wide(path);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_INVOKEIDLIST | SEE_MASK_FLAG_NO_UI,
        hwnd: HWND::default(),
        lpVerb: PCWSTR(verb_w.as_ptr()),
        lpFile: PCWSTR(file_w.as_ptr()),
        lpParameters: PCWSTR::null(),
        lpDirectory: PCWSTR::null(),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    // SAFETY: `info`, `verb_w` e `file_w` permanecem vivos e imutáveis na stack durante `ShellExecuteExW`.
    unsafe { ShellExecuteExW(&raw mut info) }.map_err(|e| map_io_err(path, win_err_to_io(e)))
}

struct WinCopyCallbackState<'a> {
    cancel: &'a CancellationToken,
    on_progress: &'a mut dyn FnMut(u64),
    cancelled_by_token: bool,
}

unsafe extern "system" fn win_copy_progress_cb(
    _total_size: i64,
    total_transferred: i64,
    _stream_size: i64,
    _stream_transferred: i64,
    _stream_number: u32,
    _reason: LPPROGRESS_ROUTINE_CALLBACK_REASON,
    _src_handle: HANDLE,
    _dst_handle: HANDLE,
    lp_data: *const c_void,
) -> COPYPROGRESSROUTINE_PROGRESS {
    if lp_data.is_null() {
        return PROGRESS_CONTINUE;
    }
    // SAFETY: `lp_data` aponta exclusivamente para `WinCopyCallbackState` válido no frame de `copy_file_native`.
    let state = unsafe { &mut *(lp_data as *mut WinCopyCallbackState<'_>) };
    if state.cancel.is_cancelled() {
        state.cancelled_by_token = true;
        return PROGRESS_CANCEL;
    }
    if total_transferred >= 0 {
        (state.on_progress)(total_transferred as u64);
    }
    PROGRESS_CONTINUE
}

/// Copia arquivo nativamente via `CopyFileExW` no Windows com suporte a caminhos longos e cancelamento.
pub fn copy_file_native(
    src: &Path,
    dst: &Path,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(u64),
) -> Result<(), FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let src_w = to_long_wide(src);
    let dst_w = to_long_wide(dst);
    let mut state = WinCopyCallbackState {
        cancel,
        on_progress,
        cancelled_by_token: false,
    };
    let mut cancel_flag = BOOL(0);

    // SAFETY: `src_w`, `dst_w`, `state` e `cancel_flag` permanecem válidos até `CopyFileExW` retornar.
    let res = unsafe {
        CopyFileExW(
            PCWSTR(src_w.as_ptr()),
            PCWSTR(dst_w.as_ptr()),
            Some(win_copy_progress_cb),
            Some((&raw mut state).cast()),
            Some(&raw mut cancel_flag),
            COPYFILE_FLAGS(0),
        )
    };

    if state.cancelled_by_token || cancel.is_cancelled() {
        let _ = std::fs::remove_file(dst);
        return Err(FsError::Cancelled);
    }

    res.map_err(|e| map_io_err(dst, win_err_to_io(e)))
}

pub fn extract_permissions_and_hidden(_name: &str, meta: &Metadata) -> (PermissionsMeta, bool) {
    let attrs = meta.file_attributes();
    let is_hidden = (attrs & FILE_ATTRIBUTE_HIDDEN) != 0;
    (PermissionsMeta::WinAttributes(attrs), is_hidden)
}

/// Atualiza atributos `READONLY` e `HIDDEN` do Windows via `GetFileAttributesW`/`SetFileAttributesW`.
pub fn apply_entry_properties(path: &Path, patch: &PropertyPatch) -> Result<(), FsError> {
    if patch.readonly.is_none() && patch.hidden.is_none() {
        return Ok(());
    }
    let wide = to_long_wide(path);
    // SAFETY: `wide` é null-terminated válido.
    let mut attrs = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
    if attrs == INVALID_FILE_ATTRIBUTES {
        return Err(map_io_err(path, std::io::Error::last_os_error()));
    }
    if let Some(readonly) = patch.readonly {
        if readonly {
            attrs |= FILE_ATTRIBUTE_READONLY;
        } else {
            attrs &= !FILE_ATTRIBUTE_READONLY;
        }
    }
    if let Some(hidden) = patch.hidden {
        if hidden {
            attrs |= FILE_ATTRIBUTE_HIDDEN;
        } else {
            attrs &= !FILE_ATTRIBUTE_HIDDEN;
        }
    }
    if attrs == 0 {
        attrs = FILE_ATTRIBUTE_NORMAL;
    }
    // SAFETY: `wide` é null-terminated válido e `attrs` é bitmask válida de atributos Win32.
    unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(attrs)) }
        .map_err(|e| map_io_err(path, win_err_to_io(e)))
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
    let label = vol_name
        .unwrap_or_else(|| CompactString::from(root.to_string_lossy().trim_end_matches('\\')));

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
            DRIVE_CDROM => DriveKind::OpticalOrIso,
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

        let is_optical = drive_type == DRIVE_CDROM;
        let is_removable = drive_type == DRIVE_REMOVABLE;
        drives.push(DriveInfo {
            label: display,
            mount_path,
            device_path: None,
            model: None,
            fs_type,
            kind,
            is_mounted: true,
            can_unmount: is_removable || is_optical,
            is_ejectable: is_removable || is_optical,
            is_disk_image: is_optical,
            total_bytes_hint: space.as_ref().map(|s| s.total_bytes),
            space,
        });
        idx += 1;
        let _ = idx;
    }

    drives
}

pub fn mount_drive(device_path: &Path) -> Result<PathBuf, FsError> {
    Err(FsError::Io {
        path: device_path.to_path_buf(),
        message: "montagem manual de partições por caminho não é suportada diretamente no Windows"
            .to_owned(),
    })
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: `self.0` é um handle válido aberto via `CreateFileW` ou `OpenVirtualDisk`.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// Desmonta e ejeta unidade removível ou ISO/CD nativamente via `DeviceIoControl` (zero PowerShell).
pub fn unmount_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    let target = mount_path
        .or(device_path)
        .unwrap_or_else(|| Path::new("C:\\"));
    let Some(letter) = drive_letter_from_path(target) else {
        return Err(FsError::Io {
            path: target.to_path_buf(),
            message: "não foi possível identificar a letra da unidade".to_owned(),
        });
    };

    let vol_path = format!(r"\\.\{letter}:");
    let vol_wide = to_str_wide(&vol_path);
    // SAFETY: `vol_wide` é UTF-16 terminado em NUL.
    let raw = unsafe {
        CreateFileW(
            PCWSTR(vol_wide.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
    .map_err(|e| map_io_err(target, win_err_to_io(e)))?;
    let handle = OwnedHandle(raw);

    let mut bytes_returned = 0u32;
    // Tenta travar e desmontar o volume antes de ejetar a mídia/ISO.
    // SAFETY: `handle.0` é um handle válido de volume e `bytes_returned` está na stack.
    let _ = unsafe {
        DeviceIoControl(
            handle.0,
            FSCTL_LOCK_VOLUME,
            None,
            0,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    };
    let _ = unsafe {
        DeviceIoControl(
            handle.0,
            FSCTL_DISMOUNT_VOLUME,
            None,
            0,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    };
    unsafe {
        DeviceIoControl(
            handle.0,
            IOCTL_STORAGE_EJECT_MEDIA,
            None,
            0,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    }
    .map_err(|e| map_io_err(target, win_err_to_io(e)))?;

    Ok(())
}

/// Remove com segurança / ejeta unidade removível ou imagem virtual no Windows.
pub fn eject_drive(device_path: Option<&Path>, mount_path: Option<&Path>) -> Result<(), FsError> {
    unmount_drive(device_path, mount_path)
}

/// Monta imagem `.iso` ou `.img` nativamente no Windows via `OpenVirtualDisk` + `AttachVirtualDisk`.
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

    let abs_image = if image_path.is_absolute() {
        image_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(image_path))
            .unwrap_or_else(|_| image_path.to_path_buf())
    };
    let clean_image = strip_verbatim_prefix(&abs_image);
    let image_w = to_wide(&clean_image);

    // SAFETY: consulta bitmask de letras de unidade atuais antes da montagem.
    let before_mask = unsafe { GetLogicalDrives() };

    let storage_type = VIRTUAL_STORAGE_TYPE {
        DeviceId: VIRTUAL_STORAGE_TYPE_DEVICE_ISO,
        VendorId: VIRTUAL_STORAGE_TYPE_VENDOR_MICROSOFT,
    };
    let mut raw_handle = HANDLE::default();
    // SAFETY: `image_w` é UTF-16 terminado em NUL e `raw_handle` é saída válida na stack.
    let open_err = unsafe {
        OpenVirtualDisk(
            &storage_type,
            PCWSTR(image_w.as_ptr()),
            VIRTUAL_DISK_ACCESS_ATTACH_RO,
            OPEN_VIRTUAL_DISK_FLAG_NONE,
            None,
            &mut raw_handle,
        )
    };
    if open_err != ERROR_SUCCESS {
        return Err(map_io_err(
            image_path,
            std::io::Error::from_raw_os_error(open_err.0 as i32),
        ));
    }
    let handle = OwnedHandle(raw_handle);

    // SAFETY: `handle.0` é um handle válido de disco virtual aberto para attach somente leitura.
    let attach_err = unsafe {
        AttachVirtualDisk(
            handle.0,
            None,
            ATTACH_VIRTUAL_DISK_FLAG_READ_ONLY | ATTACH_VIRTUAL_DISK_FLAG_PERMANENT_LIFETIME,
            0,
            None,
            None,
        )
    };
    if attach_err != ERROR_SUCCESS {
        return Err(map_io_err(
            image_path,
            std::io::Error::from_raw_os_error(attach_err.0 as i32),
        ));
    }
    drop(handle);

    for _ in 0..25 {
        // SAFETY: syscall sem estado de leitura da máscara de drives lógicos.
        let after_mask = unsafe { GetLogicalDrives() };
        let added = after_mask & !before_mask;
        if let Some(letter) = first_drive_letter_from_mask(added) {
            return Ok(PathBuf::from(format!("{letter}:\\")));
        }
        std::thread::sleep(Duration::from_millis(120));
    }

    Err(FsError::Io {
        path: image_path.to_path_buf(),
        message: "imagem anexada, mas nenhuma nova letra de unidade foi atribuída pelo Windows"
            .to_owned(),
    })
}

/// Assinatura leve (1 syscall) das letras de unidade ativas no Windows para hotplug automático.
pub fn drives_fingerprint() -> u64 {
    // SAFETY: `GetLogicalDrives` não recebe ponteiros e retorna apenas a bitmask de unidades.
    u64::from(unsafe { GetLogicalDrives() })
}

fn drive_letter_from_path(path: &Path) -> Option<char> {
    let s = path.to_string_lossy();
    let trimmed = s.strip_prefix(r"\\?\").unwrap_or(&s);
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    if first.is_ascii_alphabetic() && second == ':' {
        Some(first.to_ascii_uppercase())
    } else {
        None
    }
}

fn first_drive_letter_from_mask(mask: u32) -> Option<char> {
    if mask == 0 {
        return None;
    }
    let idx = mask.trailing_zeros();
    if idx < 26 {
        Some((b'A' + idx as u8) as char)
    } else {
        None
    }
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
        message: "abrir como root está disponível apenas no Linux".to_owned(),
    })
}

struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    fn init(err_path: &Path) -> Result<Self, FsError> {
        // SAFETY: inicializa COM em STA na thread de worker atual.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if hr == S_OK || hr == S_FALSE {
            Ok(Self {
                should_uninit: true,
            })
        } else if hr == RPC_E_CHANGED_MODE {
            Ok(Self {
                should_uninit: false,
            })
        } else {
            Err(map_io_err(
                err_path,
                win_err_to_io(windows::core::Error::from(hr)),
            ))
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            // SAFETY: corresponde exatamente à chamada bem-sucedida de `CoInitializeEx` nesta thread.
            unsafe { CoUninitialize() };
        }
    }
}

/// Cria um atalho nativo `.lnk` do Windows via COM (`IShellLinkW` + `IPersistFile`), sem precisar de privilégios de administrador.
pub fn create_shortcut(target: &Path, dest_dir: &Path) -> Result<PathBuf, FsError> {
    let link_meta = std::fs::symlink_metadata(target).map_err(|e| map_io_err(target, e))?;
    let is_dir = if link_meta.file_type().is_symlink() {
        std::fs::metadata(target)
            .map(|m| m.is_dir())
            .unwrap_or(false)
    } else {
        link_meta.is_dir()
    };

    let abs_target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(target))
            .unwrap_or_else(|_| target.to_path_buf())
    };
    let clean_target = strip_verbatim_prefix(&abs_target);
    let work_dir = if is_dir {
        clean_target.as_path()
    } else {
        clean_target.parent().unwrap_or(clean_target.as_path())
    };

    let mut chosen: Option<PathBuf> = None;
    for attempt in 0..10_000u32 {
        let candidate_name =
            fm_core::suggest_shortcut_name(&clean_target, is_dir, true, attempt, false);
        let candidate = dest_dir.join(&candidate_name);
        if std::fs::symlink_metadata(&candidate).is_err() {
            chosen = Some(candidate);
            break;
        }
    }
    let candidate = chosen.ok_or_else(|| FsError::Io {
        path: dest_dir.to_path_buf(),
        message: "não foi possível encontrar um nome livre para o atalho".to_owned(),
    })?;

    // `_com` deve ser declarado antes dos objetos COM para garantir que `link` e `persist`
    // sejam liberados antes de `CoUninitialize()`.
    let _com = ComGuard::init(&candidate)?;

    // SAFETY: COM está inicializado nesta thread; `ShellLink` é um CLSID nativo do sistema.
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| map_io_err(&candidate, win_err_to_io(e)))?;

    let target_w = to_wide(&clean_target);
    // SAFETY: `target_w` é UTF-16 terminado em NUL.
    unsafe { link.SetPath(PCWSTR(target_w.as_ptr())) }
        .map_err(|e| map_io_err(&candidate, win_err_to_io(e)))?;

    let work_dir_w = to_wide(work_dir);
    // SAFETY: `work_dir_w` é UTF-16 terminado em NUL.
    let _ = unsafe { link.SetWorkingDirectory(PCWSTR(work_dir_w.as_ptr())) };

    if let Some(desc) = clean_target.file_name() {
        let desc_w: Vec<u16> = desc.encode_wide().chain(std::iter::once(0)).collect();
        // SAFETY: `desc_w` é UTF-16 terminado em NUL.
        let _ = unsafe { link.SetDescription(PCWSTR(desc_w.as_ptr())) };
    }

    let persist: IPersistFile = link
        .cast()
        .map_err(|e| map_io_err(&candidate, win_err_to_io(e)))?;
    let dest_w = to_long_wide(&candidate);
    // SAFETY: `dest_w` é UTF-16 terminado em NUL e `persist` é uma interface válida.
    unsafe { persist.Save(PCWSTR(dest_w.as_ptr()), true) }
        .map_err(|e| map_io_err(&candidate, win_err_to_io(e)))?;

    Ok(candidate)
}

/// Remove prefixo verbatim `\\?\` para compatibilidade com `IShellLinkW::SetPath`.
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let unc_prefix = [
        b'\\' as u16,
        b'\\' as u16,
        b'?' as u16,
        b'\\' as u16,
        b'U' as u16,
        b'N' as u16,
        b'C' as u16,
        b'\\' as u16,
    ];
    let verbatim_prefix = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    if units.starts_with(&unc_prefix) {
        let mut out = vec![b'\\' as u16, b'\\' as u16];
        out.extend_from_slice(&units[unc_prefix.len()..]);
        PathBuf::from(OsString::from_wide(&out))
    } else if units.starts_with(&verbatim_prefix) {
        PathBuf::from(OsString::from_wide(&units[verbatim_prefix.len()..]))
    } else {
        path.to_path_buf()
    }
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

    (decode_wide_z(&vol_name_buf), decode_wide_z(&fs_name_buf))
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

/// Adiciona prefixo verbatim `\\?\` quando necessário para caminhos Windows >240 UTF-16 code units.
fn to_long_wide(path: &Path) -> Vec<u16> {
    let units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let is_already_verbatim =
        units.starts_with(&[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16]);
    if units.len() > 240 && !is_already_verbatim && path.is_absolute() {
        let mut out = if units.starts_with(&[b'\\' as u16, b'\\' as u16]) {
            let mut unc = vec![
                b'\\' as u16,
                b'\\' as u16,
                b'?' as u16,
                b'\\' as u16,
                b'U' as u16,
                b'N' as u16,
                b'C' as u16,
                b'\\' as u16,
            ];
            unc.extend_from_slice(&units[2..]);
            unc
        } else {
            let mut verbatim = vec![b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
            verbatim.extend(units);
            verbatim
        };
        out.push(0);
        return out;
    }
    to_wide(path)
}

fn to_str_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Converte `windows::core::Error` (`HRESULT_FROM_WIN32`) para `std::io::Error` preservando o código Win32.
fn win_err_to_io(err: windows::core::Error) -> std::io::Error {
    let raw = err.code().0 as u32;
    if (raw & 0xFFFF_0000) == 0x8007_0000 {
        std::io::Error::from_raw_os_error((raw & 0xFFFF) as i32)
    } else {
        std::io::Error::from(err)
    }
}

fn decode_wide_z(buf: &[u16]) -> Option<CompactString> {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if end == 0 {
        return None;
    }
    Some(CompactString::from(String::from_utf16_lossy(&buf[..end])))
}

#[cfg(test)]
mod tests {
    use super::win_err_to_io;
    use std::io::ErrorKind;
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND,
    };
    use windows::core::{Error as WinError, HRESULT};

    #[test]
    fn maps_hresult_win32_codes_to_io_error_kinds() {
        let not_found_file = WinError::from(HRESULT::from_win32(ERROR_FILE_NOT_FOUND.0));
        let not_found_path = WinError::from(HRESULT::from_win32(ERROR_PATH_NOT_FOUND.0));
        let denied = WinError::from(HRESULT::from_win32(ERROR_ACCESS_DENIED.0));

        assert_eq!(win_err_to_io(not_found_file).kind(), ErrorKind::NotFound);
        assert_eq!(win_err_to_io(not_found_path).kind(), ErrorKind::NotFound);
        assert_eq!(win_err_to_io(denied).kind(), ErrorKind::PermissionDenied);
    }

    #[test]
    fn extracts_drive_letters_from_masks_and_paths() {
        assert_eq!(super::first_drive_letter_from_mask(0), None);
        assert_eq!(super::first_drive_letter_from_mask(1 << 2), Some('C'));
        assert_eq!(super::first_drive_letter_from_mask(1 << 4), Some('E'));
        assert_eq!(
            super::drive_letter_from_path(std::path::Path::new(r"e:\")),
            Some('E')
        );
        assert_eq!(
            super::drive_letter_from_path(std::path::Path::new(r"\\?\D:\isos")),
            Some('D')
        );
    }
}
