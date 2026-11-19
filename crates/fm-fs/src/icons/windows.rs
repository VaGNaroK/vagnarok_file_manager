//! Extração de ícones nativos do Windows Shell (`SHGetFileInfoW`) com guarda RAII para handles GDI.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use fm_core::{DriveKind, SystemIconBucket, SystemIconKey, SystemIconPayload, SystemPlaceKind};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDC, GetDIBits, HBITMAP, HDC, ReleaseDC,
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Pictures,
    FOLDERID_Profile, FOLDERID_Public, FOLDERID_RecycleBinFolder, FOLDERID_Templates,
    FOLDERID_Videos, ILFree, KF_FLAG_DEFAULT, SHFILEINFOW, SHGFI_FLAGS, SHGFI_ICON,
    SHGFI_LARGEICON, SHGFI_PIDL, SHGFI_SMALLICON, SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW,
    SHGetKnownFolderIDList,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::PCWSTR;

struct OwnedHicon(HICON);
impl Drop for OwnedHicon {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }
}

struct OwnedBitmap(HBITMAP);
impl Drop for OwnedBitmap {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }
}

struct ScreenDc(HDC);
impl Drop for ScreenDc {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = ReleaseDC(Some(HWND::default()), self.0);
            }
        }
    }
}

struct MemDc(HDC);
impl Drop for MemDc {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }
}

pub(super) fn resolve_system_icon(
    key: &SystemIconKey,
    bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    let size_flag = match bucket {
        SystemIconBucket::Small24 => SHGFI_SMALLICON,
        SystemIconBucket::Large64 => SHGFI_LARGEICON,
    };

    let hicon = match key {
        SystemIconKey::Directory => sh_icon_for_name("folder", true, size_flag),
        SystemIconKey::Archive => sh_icon_for_name("archive.zip", false, size_flag),
        SystemIconKey::GenericFile => sh_icon_for_name("file", false, size_flag),
        SystemIconKey::Extension(ext) => {
            let synthetic = format!("item.{ext}");
            sh_icon_for_name(&synthetic, false, size_flag)
        }
        SystemIconKey::SpecialPlace(place) => sh_icon_for_known_folder(*place, size_flag)
            .or_else(|| sh_icon_for_name("folder", true, size_flag)),
        SystemIconKey::Drive(kind) => sh_icon_for_drive(*kind, size_flag)
            .or_else(|| sh_icon_for_name("folder", true, size_flag)),
    }?;

    hicon_to_rgba8(&hicon)
}

/// Extrai o ícone embutido de um executável `.exe`/`.ico` para exibição em "Abrir com...".
pub(super) fn resolve_icon_by_hint(
    icon_hint: &str,
    bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    let size_flag = match bucket {
        SystemIconBucket::Small24 => SHGFI_SMALLICON,
        SystemIconBucket::Large64 => SHGFI_LARGEICON,
    };
    let clean_path = strip_windows_icon_index(icon_hint);
    if clean_path.is_empty() {
        return None;
    }
    let wide = to_wide_null(OsStr::new(clean_path));
    let mut info = SHFILEINFOW::default();
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            Default::default(),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | size_flag,
        )
    };
    if ok == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let hicon = OwnedHicon(info.hIcon);
    hicon_to_rgba8(&hicon)
}

fn strip_windows_icon_index(raw: &str) -> &str {
    let trimmed = raw.trim().trim_matches('"');
    if let Some((path_part, idx_part)) = trimmed.rsplit_once(',') {
        let idx_clean = idx_part.trim();
        if !idx_clean.is_empty()
            && idx_clean
                .strip_prefix('-')
                .unwrap_or(idx_clean)
                .chars()
                .all(|c| c.is_ascii_digit())
        {
            return path_part.trim().trim_matches('"');
        }
    }
    trimmed
}

fn to_wide_null(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn sh_icon_for_name(name: &str, is_dir: bool, size_flag: SHGFI_FLAGS) -> Option<OwnedHicon> {
    let wide = to_wide_null(OsStr::new(name));
    let attrs = if is_dir {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        FILE_ATTRIBUTE_NORMAL
    };
    let mut info = SHFILEINFOW::default();
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            attrs,
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_USEFILEATTRIBUTES | size_flag,
        )
    };
    if ok == 0 || info.hIcon.is_invalid() {
        None
    } else {
        Some(OwnedHicon(info.hIcon))
    }
}

fn sh_icon_for_drive(kind: DriveKind, size_flag: SHGFI_FLAGS) -> Option<OwnedHicon> {
    let probe = match kind {
        DriveKind::InternalSsd | DriveKind::SecondaryDisk | DriveKind::Removable => {
            let root = std::env::var_os("SystemDrive")
                .map(|d| format!("{}\\", d.to_string_lossy()))
                .unwrap_or_else(|| "C:\\".to_owned());
            if Path::new(&root).exists() {
                root
            } else {
                "C:\\".to_owned()
            }
        }
        DriveKind::Network => return sh_icon_for_known_folder(SystemPlaceKind::Home, size_flag),
    };
    let wide = to_wide_null(OsStr::new(&probe));
    let mut info = SHFILEINFOW::default();
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_DIRECTORY,
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | size_flag,
        )
    };
    if ok == 0 || info.hIcon.is_invalid() {
        None
    } else {
        Some(OwnedHicon(info.hIcon))
    }
}

fn sh_icon_for_known_folder(place: SystemPlaceKind, size_flag: SHGFI_FLAGS) -> Option<OwnedHicon> {
    let folder_id = match place {
        SystemPlaceKind::Home => &FOLDERID_Profile,
        SystemPlaceKind::Desktop => &FOLDERID_Desktop,
        SystemPlaceKind::Documents => &FOLDERID_Documents,
        SystemPlaceKind::Downloads => &FOLDERID_Downloads,
        SystemPlaceKind::Music => &FOLDERID_Music,
        SystemPlaceKind::Pictures => &FOLDERID_Pictures,
        SystemPlaceKind::Videos => &FOLDERID_Videos,
        SystemPlaceKind::Public => &FOLDERID_Public,
        SystemPlaceKind::Templates => &FOLDERID_Templates,
        SystemPlaceKind::Trash => &FOLDERID_RecycleBinFolder,
    };

    let pidl = unsafe { SHGetKnownFolderIDList(folder_id, KF_FLAG_DEFAULT.0 as u32, None).ok()? };
    if pidl.is_null() {
        return None;
    }

    let mut info = SHFILEINFOW::default();
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(pidl as *const u16),
            Default::default(),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_PIDL | SHGFI_ICON | size_flag,
        )
    };
    unsafe {
        ILFree(Some(pidl));
    }
    if ok == 0 || info.hIcon.is_invalid() {
        None
    } else {
        Some(OwnedHicon(info.hIcon))
    }
}

fn hicon_to_rgba8(hicon: &OwnedHicon) -> Option<SystemIconPayload> {
    let mut icon_info = ICONINFO::default();
    unsafe {
        GetIconInfo(hicon.0, &mut icon_info).ok()?;
    }
    let color_bmp = OwnedBitmap(icon_info.hbmColor);
    let mask_bmp = OwnedBitmap(icon_info.hbmMask);

    if color_bmp.0.is_invalid() {
        return None;
    }

    let screen_dc = ScreenDc(unsafe { GetDC(Some(HWND::default())) });
    if screen_dc.0.is_invalid() {
        return None;
    }
    let mem_dc = MemDc(unsafe { CreateCompatibleDC(Some(screen_dc.0)) });
    if mem_dc.0.is_invalid() {
        return None;
    }

    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    let probe = unsafe {
        GetDIBits(
            mem_dc.0,
            color_bmp.0,
            0,
            0,
            None,
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    if probe == 0 {
        return None;
    }

    let width = header.bmiHeader.biWidth.unsigned_abs();
    let height = header.bmiHeader.biHeight.unsigned_abs();
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }

    header.bmiHeader.biHeight = -(height as i32);
    header.bmiHeader.biPlanes = 1;
    header.bmiHeader.biBitCount = 32;
    header.bmiHeader.biCompression = BI_RGB.0;

    let mut bgra = vec![0u8; (width * height * 4) as usize];
    let lines = unsafe {
        GetDIBits(
            mem_dc.0,
            color_bmp.0,
            0,
            height,
            Some(bgra.as_mut_ptr().cast()),
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    if lines == 0 {
        return None;
    }

    let all_alpha_zero = bgra.as_chunks::<4>().0.iter().all(|px| px[3] == 0);
    if all_alpha_zero && !mask_bmp.0.is_invalid() {
        let mut mask_bgra = vec![0u8; (width * height * 4) as usize];
        let mask_lines = unsafe {
            GetDIBits(
                mem_dc.0,
                mask_bmp.0,
                0,
                height,
                Some(mask_bgra.as_mut_ptr().cast()),
                &mut header,
                DIB_RGB_COLORS,
            )
        };
        if mask_lines > 0 {
            for (px, m) in bgra
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(mask_bgra.as_chunks::<4>().0.iter())
            {
                px[3] = if m[0] == 0 { 255 } else { 0 };
            }
        }
    }

    bgra_to_rgba_in_place(&mut bgra);
    Some(SystemIconPayload::Rgba8 {
        width,
        height,
        pixels: bgra,
    })
}

fn bgra_to_rgba_in_place(buf: &mut [u8]) {
    for px in buf.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
}
