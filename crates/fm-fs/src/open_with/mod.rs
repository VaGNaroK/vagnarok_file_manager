//! Descoberta de aplicativos instalados, associação padrão e execução personalizada ("Abrir com...").
//!
//! Código específico de SO fica confinado em `linux`, `windows` e `fallback`.

use std::path::Path;

use fm_core::{FsError, OpenWithAssociation, OpenWithCatalog, open_with_type_key};
use tokio_util::sync::CancellationToken;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use fallback as platform;

/// Resolve a chave normalizada de tipo (`mime` específico ou `.ext`) usada para salvar e buscar preferências.
#[must_use]
pub fn open_with_type_key_for_path(path: &Path, mime_hint: Option<&str>) -> String {
    if let Some(hint) = mime_hint
        && !hint.trim().is_empty()
        && !hint.eq_ignore_ascii_case("application/octet-stream")
    {
        return open_with_type_key(path, Some(hint));
    }

    #[cfg(target_os = "linux")]
    if let Some(detected) = crate::icons::lookup_mime_for_path(path) {
        return open_with_type_key(path, Some(&detected));
    }

    open_with_type_key(path, mime_hint)
}

/// Descobre aplicativos instalados e categoriza padrão / recomendados / outros em thread de worker.
pub async fn discover_open_with_catalog(
    path: std::path::PathBuf,
    mime_hint: Option<String>,
    cancel: CancellationToken,
) -> Result<OpenWithCatalog, FsError> {
    let err_path = path.clone();
    tokio::task::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        platform::discover_open_with_catalog(&path, mime_hint.as_deref(), &cancel)
    })
    .await
    .map_err(|err| FsError::Io {
        path: err_path,
        message: err.to_string(),
    })?
}

/// Persiste o aplicativo padrão no banco nativo do sistema quando suportado (ex.: `mimeapps.list` no Linux).
pub async fn save_open_with_system_default(
    type_key: String,
    assoc: OpenWithAssociation,
) -> Result<(), FsError> {
    tokio::task::spawn_blocking(move || platform::save_system_default(&type_key, &assoc))
        .await
        .map_err(|err| FsError::Io {
            path: std::path::PathBuf::new(),
            message: err.to_string(),
        })?
}

/// Remove a associação padrão do usuário no sistema quando aplicável.
pub async fn reset_open_with_system_default(type_key: String) -> Result<(), FsError> {
    tokio::task::spawn_blocking(move || platform::reset_system_default(&type_key))
        .await
        .map_err(|err| FsError::Io {
            path: std::path::PathBuf::new(),
            message: err.to_string(),
        })?
}

pub(crate) fn launch_open_with_sync(
    path: &Path,
    target: &OpenWithAssociation,
) -> Result<(), FsError> {
    platform::launch_open_with(path, target)
}

/// Tokeniza uma linha de comando respeitando aspas simples/duplas sem invocar shell.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn split_posix_command_tokens(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut quote: Option<char> = None;
    let mut escape = false;

    for ch in raw.chars() {
        if escape {
            current.push(ch);
            escape = false;
            in_token = true;
            continue;
        }
        match quote {
            Some(q) => {
                if ch == '\\' && q == '"' {
                    escape = true;
                } else if ch == q {
                    quote = None;
                } else {
                    current.push(ch);
                }
                in_token = true;
            }
            None => {
                if ch == '\\' {
                    escape = true;
                    in_token = true;
                } else if ch == '\'' || ch == '"' {
                    quote = Some(ch);
                    in_token = true;
                } else if ch.is_whitespace() {
                    if in_token {
                        tokens.push(std::mem::take(&mut current));
                        in_token = false;
                    }
                } else {
                    current.push(ch);
                    in_token = true;
                }
            }
        }
    }
    if escape {
        current.push('\\');
    }
    if in_token || !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Tokeniza comandos Windows mantendo contrabarras (`\`) literais como separadores de caminho.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn split_windows_command_tokens(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut in_quotes = false;

    for ch in raw.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            in_token = true;
        } else if !in_quotes && ch.is_whitespace() {
            if in_token {
                tokens.push(std::mem::take(&mut current));
                in_token = false;
            }
        } else {
            current.push(ch);
            in_token = true;
        }
    }
    if in_token || !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_posix_exec_preserving_quotes_and_escapes() {
        let tokens =
            split_posix_command_tokens(r#"/usr/bin/vlc --started-from-file "My\ Video.mp4" %U"#);
        assert_eq!(
            tokens,
            vec!["/usr/bin/vlc", "--started-from-file", "My Video.mp4", "%U"]
        );
    }

    #[test]
    fn tokenizes_windows_command_preserving_backslashes() {
        let tokens = split_windows_command_tokens(
            r#""C:\Program Files\VideoLAN\VLC\vlc.exe" --started-from-file "%1""#,
        );
        assert_eq!(
            tokens,
            vec![
                r"C:\Program Files\VideoLAN\VLC\vlc.exe",
                "--started-from-file",
                "%1"
            ]
        );
    }
}
