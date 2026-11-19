//! Resolução multiplataforma de ícones do tema ou shell do sistema hospedeiro.
//!
//! Código específico de SO fica confinado aos submódulos `linux` e `windows`.

use fm_core::{SystemIconBucket, SystemIconKey, SystemIconPayload};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "linux")]
pub(crate) use linux::lookup_mime_for_path;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use fallback as platform;

/// Resolve o payload de ícone do sistema operacional para a chave e bucket informados.
pub fn resolve_system_icon(
    key: &SystemIconKey,
    bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    platform::resolve_system_icon(key, bucket)
}

/// Resolve o ícone de um aplicativo instalado a partir do `Icon=` (`.desktop`) ou caminho `.exe`/`.ico`.
pub fn resolve_app_icon(icon_hint: &str, bucket: SystemIconBucket) -> Option<SystemIconPayload> {
    let trimmed = icon_hint.trim();
    if trimmed.is_empty() {
        return None;
    }
    platform::resolve_icon_by_hint(trimmed, bucket)
}
