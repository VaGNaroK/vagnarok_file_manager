//! Detecção multiplataforma de identidade física de arquivos (hardlinks) e carimbo de integridade para cache.
//!
//! `#[cfg]` específicos de SO ficam confinados neste submódulo.

use std::fs::Metadata;
use std::path::Path;

/// Identidade física estável de um arquivo no sistema de arquivos:
/// - Linux/Unix: `(dev, ino)`
/// - Windows: `(dwVolumeSerialNumber, file_index_64)`
pub type FileIdentity = (u64, u64);

/// Obtém a identidade física do arquivo para detectar hardlinks que compartilham o mesmo dado em disco.
pub fn file_identity(path: &Path, meta: &Metadata) -> Option<FileIdentity> {
    #[cfg(unix)]
    {
        let _ = path;
        use std::os::unix::fs::MetadataExt;
        Some((meta.dev(), meta.ino()))
    }
    #[cfg(windows)]
    {
        let _ = meta;
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };

        let file = crate::open_shared::open_shared_read(path).ok()?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        let ok =
            unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info).is_ok() };
        if !ok {
            return None;
        }
        let vol = info.dwVolumeSerialNumber as u64;
        let idx = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        Some((vol, idx))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, meta);
        None
    }
}

/// Carimbo auxiliar de integridade para validação do cache `redb` (`md5_meta_v1`):
/// - Linux/Unix: combina `(ctime, ctime_nsec)` completos (detecta escritas mesmo se `mtime` for restaurado);
/// - Windows: `file_attributes`.
pub fn cache_integrity_stamp(meta: &Metadata) -> u128 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let combined = (meta.ctime() as i128)
            .saturating_mul(1_000_000_000)
            .saturating_add(meta.ctime_nsec() as i128);
        combined as u128
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() as u128
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = meta;
        0
    }
}
