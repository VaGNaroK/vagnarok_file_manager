use std::ffi::OsString;
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
use windows::Win32::Foundation::{ERROR_NO_MORE_FILES, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    FIND_FIRST_EX_LARGE_FETCH, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW,
    GetVolumeInformationW, WIN32_FIND_DATAW,
};
use windows::core::PCWSTR;

use crate::{make_file_entry, map_io_err};

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
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
    let wide = to_wide(&pattern);
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
            return Err(map_io_err(path, std::io::Error::from(err)));
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

            let size_bytes = (u64::from(find_data.nFileSizeHigh) << 32) | u64::from(find_data.nFileSizeLow);
            let modified_at = filetime_to_system_time(
                find_data.ftLastWriteTime.dwHighDateTime,
                find_data.ftLastWriteTime.dwLowDateTime,
            );

            entries.push(make_file_entry(
                name,
                entry_path,
                kind,
                size_bytes,
                modified_at,
                PermissionsMeta::WinAttributes(attrs),
                is_hidden,
            ));
        }

        // SAFETY: `handle.0` é válido e `find_data` pertence ao frame atual.
        match unsafe { FindNextFileW(handle.0, &raw mut find_data) } {
            Ok(()) => {}
            Err(err) if err.code() == ERROR_NO_MORE_FILES.into() => break,
            Err(err) => return Err(map_io_err(path, std::io::Error::from(err))),
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

/// Abre Windows Terminal (`wt.exe`) ou fallback `cmd.exe` no diretório especificado.
pub fn open_terminal_at(path: &Path) -> Result<(), FsError> {
    if Command::new("wt.exe").arg("-d").arg(path).spawn().is_ok() {
        return Ok(());
    }
    Command::new("cmd.exe")
        .args(["/C", "start", "cmd.exe"])
        .current_dir(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| map_io_err(path, e))
}

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
