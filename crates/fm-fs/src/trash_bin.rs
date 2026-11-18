//! Suporte unificado à Lixeira (FreeDesktop no Linux/Flatpak e Recycle Bin no Windows).

use std::path::{Path, PathBuf};

use compact_str::CompactString;
use fm_core::{DirectorySnapshot, EntryKind, FsError, VIRTUAL_TRASH_PATH, is_virtual_trash_path};
use tokio_util::sync::CancellationToken;

use crate::{default_permissions_meta, make_file_entry};

/// Retorna o caminho lógico da Lixeira exibido na barra lateral.
pub(crate) fn resolve_trash_dir() -> Option<PathBuf> {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        Some(PathBuf::from(VIRTUAL_TRASH_PATH))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Move os caminhos indicados para a Lixeira do sistema.
pub(crate) fn move_to_trash(targets: &[PathBuf]) -> Result<(), FsError> {
    if targets.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        freedesktop_host::move_to_trash(targets)
    }
    #[cfg(not(target_os = "linux"))]
    {
        trash::delete_all(targets).map_err(|err| FsError::Io {
            path: targets.first().cloned().unwrap_or_default(),
            message: format!("falha ao enviar para a lixeira: {err}"),
        })
    }
}

/// Lista os itens atualmente presentes na Lixeira virtual (`trash://`).
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn list_virtual_trash(
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let items = list_all_trash_items().map_err(|err| FsError::Io {
        path: PathBuf::from(VIRTUAL_TRASH_PATH),
        message: format!("falha ao listar lixeira: {err}"),
    })?;

    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }

        let id_path = PathBuf::from(&item.id);
        let entry_path = resolve_item_backing_path(&id_path, &item.name);
        let meta = std::fs::symlink_metadata(&entry_path).ok();
        let (is_dir, size_bytes) = if let Some(m) = &meta {
            let dir = m.is_dir();
            (dir, if dir { 0 } else { m.len() })
        } else {
            fallback_trash_item_size(&item)
        };

        let modified_at = if item.time_deleted >= 0 {
            Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(item.time_deleted as u64))
        } else {
            meta.as_ref().and_then(|m| m.modified().ok())
        };
        let created_at = meta.as_ref().and_then(|m| m.created().ok());
        let name = CompactString::from(item.name.to_string_lossy().as_ref());
        let kind = if is_dir {
            EntryKind::Directory
        } else {
            EntryKind::File
        };

        entries.push(make_file_entry(
            name,
            entry_path,
            kind,
            size_bytes,
            modified_at,
            created_at,
            default_permissions_meta(),
            false,
        ));
    }

    Ok(DirectorySnapshot {
        path: PathBuf::from(VIRTUAL_TRASH_PATH),
        entries,
        fs_type: None,
        request_id,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn list_virtual_trash(
    request_id: u64,
    _cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    Ok(DirectorySnapshot {
        path: PathBuf::from(VIRTUAL_TRASH_PATH),
        entries: Vec::new(),
        fs_type: None,
        request_id,
    })
}

/// Exclui permanentemente os alvos que residem dentro da Lixeira e retorna quaisquer outros caminhos intactos.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn purge_matching_trash_items(targets: &[PathBuf]) -> Result<Vec<PathBuf>, FsError> {
    let any_trash_id = targets.iter().any(|p| is_potential_trash_target(p));
    if !any_trash_id {
        return Ok(targets.to_vec());
    }

    let items = list_all_trash_items().map_err(|err| FsError::Io {
        path: targets.first().cloned().unwrap_or_default(),
        message: format!("falha ao listar lixeira: {err}"),
    })?;

    let mut matched = Vec::new();
    let mut matched_paths = Vec::new();
    for item in items {
        if let Some(p) = targets.iter().find(|p| trash_item_matches_exact(&item, p)) {
            matched_paths.push(p.clone());
            matched.push(item);
        }
    }

    if !matched.is_empty() {
        trash::os_limited::purge_all(matched).map_err(|err| FsError::Io {
            path: targets.first().cloned().unwrap_or_default(),
            message: format!("falha ao excluir da lixeira: {err}"),
        })?;
    }

    Ok(targets
        .iter()
        .filter(|p| !matched_paths.contains(p))
        .cloned()
        .collect())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn purge_matching_trash_items(targets: &[PathBuf]) -> Result<Vec<PathBuf>, FsError> {
    Ok(targets.to_vec())
}

/// Restaura para o local original os itens selecionados na Lixeira.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn restore_trash_items(paths: &[PathBuf]) -> Result<usize, FsError> {
    let items = list_all_trash_items().map_err(|err| FsError::Io {
        path: paths.first().cloned().unwrap_or_default(),
        message: format!("falha ao listar lixeira: {err}"),
    })?;

    let mut matched = Vec::new();
    for item in items {
        let exact = paths.iter().any(|p| trash_item_matches_exact(&item, p));
        let orig_match = !exact
            && paths
                .iter()
                .any(|p| !is_potential_trash_target(p) && p == &item.original_path());
        if exact || orig_match {
            matched.push(item);
        }
    }

    let count = matched.len();
    if count == 0 {
        return Ok(0);
    }

    trash::os_limited::restore_all(matched).map_err(|err| FsError::Io {
        path: paths.first().cloned().unwrap_or_default(),
        message: format!("falha ao restaurar da lixeira: {err}"),
    })?;
    Ok(count)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn restore_trash_items(_paths: &[PathBuf]) -> Result<usize, FsError> {
    Ok(0)
}

/// Esvazia todos os itens presentes na Lixeira.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn purge_all_trash_items() -> Result<(), FsError> {
    let items = list_all_trash_items().map_err(|err| FsError::Io {
        path: PathBuf::new(),
        message: format!("falha ao listar lixeira: {err}"),
    })?;
    if items.is_empty() {
        return Ok(());
    }
    trash::os_limited::purge_all(items).map_err(|err| FsError::Io {
        path: PathBuf::new(),
        message: format!("falha ao esvaziar lixeira: {err}"),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn purge_all_trash_items() -> Result<(), FsError> {
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn list_all_trash_items() -> Result<Vec<trash::TrashItem>, trash::Error> {
    #[cfg(target_os = "linux")]
    {
        freedesktop_host::list_all_trash_items()
    }
    #[cfg(not(target_os = "linux"))]
    {
        trash::os_limited::list()
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn resolve_item_backing_path(id_path: &Path, item_name: &std::ffi::OsStr) -> PathBuf {
    #[cfg(target_os = "linux")]
    if let Some(files_path) = freedesktop_host::resolve_trash_file_path(id_path) {
        return files_path;
    }

    if id_path.as_os_str().is_empty() {
        PathBuf::from(VIRTUAL_TRASH_PATH).join(item_name)
    } else {
        id_path.to_path_buf()
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn fallback_trash_item_size(item: &trash::TrashItem) -> (bool, u64) {
    #[cfg(target_os = "windows")]
    {
        match trash::os_limited::metadata(item) {
            Ok(meta) => match meta.size {
                trash::TrashItemSize::Entries(_) => (true, 0),
                trash::TrashItemSize::Bytes(bytes) => (false, bytes),
            },
            Err(_) => (false, 0),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = item;
        (false, 0)
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn is_potential_trash_target(path: &Path) -> bool {
    if is_virtual_trash_path(path) {
        return true;
    }
    let lossy = path.to_string_lossy();
    if lossy.starts_with(VIRTUAL_TRASH_PATH) {
        return true;
    }
    let lower = lossy.to_ascii_lowercase();
    lower.contains("$recycle.bin")
        || lower.contains("{645ff040-5081-101b-9f08-00aa002f954e}")
        || lower.contains("/trash/files/")
        || lower.contains("/trash/info/")
        || lower.contains("/.trash-")
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn trash_item_matches_exact(item: &trash::TrashItem, candidate: &Path) -> bool {
    if candidate.as_os_str() == item.id {
        return true;
    }
    #[cfg(target_os = "linux")]
    if freedesktop_host::resolve_trash_file_path(Path::new(&item.id))
        .is_some_and(|backing| backing == candidate)
    {
        return true;
    }
    Path::new(VIRTUAL_TRASH_PATH).join(&item.name) == candidate
}

#[cfg(target_os = "linux")]
mod freedesktop_host {
    use std::collections::HashSet;
    use std::ffi::{OsStr, OsString};
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    use std::path::{Component, Path, PathBuf};

    use chrono::{Local, NaiveDateTime, TimeZone};
    use directories::BaseDirs;
    use fm_core::FsError;

    /// Dado `.../Trash/info/<stem>.trashinfo`, retorna `.../Trash/files/<stem>`.
    pub(super) fn resolve_trash_file_path(info_path: &Path) -> Option<PathBuf> {
        if info_path.extension() != Some(OsStr::new("trashinfo")) {
            return None;
        }
        let info_dir = info_path.parent()?;
        if info_dir.file_name() != Some(OsStr::new("info")) {
            return None;
        }
        let trash_dir = info_dir.parent()?;
        let stem = info_path.file_stem()?;
        Some(trash_dir.join("files").join(stem))
    }

    /// No Flatpak, `XDG_DATA_HOME` aponta para o sandbox (`~/.var/app/<id>/data`), então a Lixeira real do
    /// desktop (`~/.local/share/Trash`) precisa ser integrada diretamente quando `--filesystem=host` está ativo.
    fn flatpak_host_home_trash_dir() -> Option<PathBuf> {
        std::env::var_os("FLATPAK_ID")?;
        let base = BaseDirs::new()?;
        let host_trash = base.home_dir().join(".local").join("share").join("Trash");
        let sandbox_trash = base.data_local_dir().join("Trash");
        (host_trash != sandbox_trash).then_some(host_trash)
    }

    pub(super) fn list_all_trash_items() -> Result<Vec<trash::TrashItem>, trash::Error> {
        let host_dir = flatpak_host_home_trash_dir();
        let mut items = match trash::os_limited::list() {
            Ok(list) => list,
            Err(err) if host_dir.is_some() => {
                tracing::debug!("sandbox_trash_list_ignored_in_flatpak: {err}");
                Vec::new()
            }
            Err(err) => return Err(err),
        };

        if let Some(host_trash) = host_dir {
            let mut seen_ids: HashSet<OsString> =
                items.iter().map(|item| item.id.clone()).collect();
            for item in scan_trash_dir(&host_trash) {
                if seen_ids.insert(item.id.clone()) {
                    items.push(item);
                }
            }
        }
        Ok(items)
    }

    pub(super) fn move_to_trash(targets: &[PathBuf]) -> Result<(), FsError> {
        let Some(host_trash) = flatpak_host_home_trash_dir() else {
            return trash::delete_all(targets).map_err(|err| FsError::Io {
                path: targets.first().cloned().unwrap_or_default(),
                message: format!("falha ao enviar para a lixeira: {err}"),
            });
        };

        let home_dev = host_home_dev(&host_trash);
        let mut external_targets = Vec::new();

        for target in targets {
            let resolved = canonicalize_preserving_leaf(target)?;
            let target_dev = fs::symlink_metadata(&resolved)
                .map(|m| m.dev())
                .map_err(|err| FsError::Io {
                    path: target.clone(),
                    message: format!("falha ao inspecionar item: {err}"),
                })?;

            if home_dev == Some(target_dev) {
                match move_item_to_trash_dir(&resolved, &host_trash)? {
                    MoveOutcome::Moved => {}
                    MoveOutcome::CrossDevice => external_targets.push(target.clone()),
                }
            } else {
                external_targets.push(target.clone());
            }
        }

        if !external_targets.is_empty() {
            trash::delete_all(&external_targets).map_err(|err| FsError::Io {
                path: external_targets.first().cloned().unwrap_or_default(),
                message: format!("falha ao enviar para a lixeira: {err}"),
            })?;
        }
        Ok(())
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum MoveOutcome {
        Moved,
        CrossDevice,
    }

    fn host_home_dev(host_trash: &Path) -> Option<u64> {
        for candidate in [
            Some(host_trash),
            host_trash.parent(),
            host_trash.parent().and_then(Path::parent),
            host_trash
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent),
        ]
        .into_iter()
        .flatten()
        {
            if let Ok(meta) = fs::metadata(candidate) {
                return Some(meta.dev());
            }
        }
        None
    }

    pub(super) fn scan_trash_dir(trash_dir: &Path) -> Vec<trash::TrashItem> {
        let info_dir = trash_dir.join("info");
        let Ok(read_dir) = fs::read_dir(&info_dir) else {
            return Vec::new();
        };
        let top_dir = trash_dir.parent().unwrap_or_else(|| Path::new("/"));
        let mut items = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("trashinfo")) {
                continue;
            }
            if !entry.file_type().is_ok_and(|ft| ft.is_file()) {
                continue;
            }
            if let Some(item) = parse_trashinfo_file(&path, top_dir) {
                items.push(item);
            }
        }
        items
    }

    pub(super) fn parse_trashinfo_file(
        info_path: &Path,
        top_dir: &Path,
    ) -> Option<trash::TrashItem> {
        let bytes = fs::read(info_path).ok()?;
        let content = String::from_utf8_lossy(&bytes);
        let mut lines = content.lines();
        if !matches!(lines.next(), Some(header) if header.trim() == "[Trash Info]") {
            return None;
        }

        let mut name: Option<OsString> = None;
        let mut original_parent: Option<PathBuf> = None;
        let mut time_deleted: Option<i64> = None;
        let mut seen_path = false;
        let mut seen_date = false;

        for line in lines {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "Path" if !seen_path => {
                    seen_path = true;
                    let decoded = decode_trashinfo_path(value.trim());
                    if decoded.file_name().is_none()
                        || decoded.as_os_str().as_bytes().contains(&0)
                        || (decoded.is_relative()
                            && decoded
                                .components()
                                .any(|c| matches!(c, Component::ParentDir)))
                    {
                        return None;
                    }
                    let full = if decoded.is_relative() {
                        top_dir.join(&decoded)
                    } else {
                        decoded
                    };
                    name = full.file_name().map(OsStr::to_os_string);
                    original_parent = full.parent().map(Path::to_path_buf);
                }
                "DeletionDate" if !seen_date => {
                    seen_date = true;
                    if let Ok(naive) =
                        NaiveDateTime::parse_from_str(value.trim(), "%Y-%m-%dT%H:%M:%S")
                        && let Some(local_dt) = Local.from_local_datetime(&naive).earliest()
                    {
                        time_deleted = Some(local_dt.timestamp());
                    }
                }
                _ => {}
            }
        }

        Some(trash::TrashItem {
            id: info_path.as_os_str().to_os_string(),
            name: name?,
            original_parent: original_parent?,
            time_deleted: time_deleted.unwrap_or(-1),
        })
    }

    pub(super) fn move_item_to_trash_dir(
        abs_target: &Path,
        trash_dir: &Path,
    ) -> Result<MoveOutcome, FsError> {
        let files_dir = trash_dir.join("files");
        let info_dir = trash_dir.join("info");
        ensure_private_dir(trash_dir)?;
        ensure_private_dir(&files_dir)?;
        ensure_private_dir(&info_dir)?;

        let original_name = abs_target.file_name().ok_or_else(|| FsError::Io {
            path: abs_target.to_path_buf(),
            message: "caminho inválido para a lixeira".to_owned(),
        })?;
        let now_str = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let encoded_path = encode_trashinfo_path(abs_target);
        let trashinfo_body = format!("[Trash Info]\nPath={encoded_path}\nDeletionDate={now_str}\n");

        let (dest_file, info_path, mut info_file) =
            reserve_trash_slot(original_name, &files_dir, &info_dir)?;

        if let Err(err) = info_file.write_all(trashinfo_body.as_bytes()) {
            drop(info_file);
            let _ = fs::remove_file(&info_path);
            return Err(FsError::Io {
                path: info_path,
                message: format!("falha ao gravar metadados da lixeira: {err}"),
            });
        }
        drop(info_file);

        match fs::rename(abs_target, &dest_file) {
            Ok(()) => Ok(MoveOutcome::Moved),
            Err(err) if err.kind() == std::io::ErrorKind::CrossesDevices => {
                let _ = fs::remove_file(&info_path);
                Ok(MoveOutcome::CrossDevice)
            }
            Err(err) => {
                let _ = fs::remove_file(&info_path);
                Err(FsError::Io {
                    path: abs_target.to_path_buf(),
                    message: format!("falha ao mover item para a lixeira: {err}"),
                })
            }
        }
    }

    fn ensure_private_dir(dir: &Path) -> Result<(), FsError> {
        if dir.is_dir() {
            return Ok(());
        }
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|err| FsError::Io {
                path: dir.to_path_buf(),
                message: format!("falha ao preparar diretório da lixeira: {err}"),
            })
    }

    fn reserve_trash_slot(
        original_name: &OsStr,
        files_dir: &Path,
        info_dir: &Path,
    ) -> Result<(PathBuf, PathBuf, fs::File), FsError> {
        let name_path = Path::new(original_name);
        let stem = name_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "item".to_owned());
        let ext = name_path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

        for idx in 1u32..=10_000 {
            let candidate_name: OsString = if idx == 1 {
                original_name.to_os_string()
            } else {
                OsString::from(format!("{stem}.{idx}{ext}"))
            };
            let dest_file = files_dir.join(&candidate_name);
            if fs::symlink_metadata(&dest_file).is_ok() {
                continue;
            }
            let mut info_name = candidate_name.clone();
            info_name.push(".trashinfo");
            let info_path = info_dir.join(info_name);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&info_path)
            {
                Ok(file) => return Ok((dest_file, info_path, file)),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => {
                    return Err(FsError::Io {
                        path: info_path,
                        message: format!("falha ao criar arquivo .trashinfo: {err}"),
                    });
                }
            }
        }

        Err(FsError::Io {
            path: info_dir.to_path_buf(),
            message: "não foi possível reservar nome único na lixeira".to_owned(),
        })
    }

    fn canonicalize_preserving_leaf(target: &Path) -> Result<PathBuf, FsError> {
        let abs = if target.is_relative() {
            std::env::current_dir()
                .map_err(|err| FsError::Io {
                    path: target.to_path_buf(),
                    message: err.to_string(),
                })?
                .join(target)
        } else {
            target.to_path_buf()
        };
        let parent = abs.parent().ok_or_else(|| FsError::Io {
            path: target.to_path_buf(),
            message: "não é possível enviar a raiz para a lixeira".to_owned(),
        })?;
        let leaf = abs.file_name().ok_or_else(|| FsError::Io {
            path: target.to_path_buf(),
            message: "nome de item inválido para a lixeira".to_owned(),
        })?;
        let canonical_parent = parent.canonicalize().map_err(|err| FsError::Io {
            path: parent.to_path_buf(),
            message: format!("falha ao resolver caminho original: {err}"),
        })?;
        Ok(canonical_parent.join(leaf))
    }

    pub(super) fn encode_trashinfo_path(path: &Path) -> String {
        let mut out = String::new();
        for &byte in path.as_os_str().as_bytes() {
            let unreserved = matches!(
                byte,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/'
            );
            if unreserved {
                out.push(byte as char);
            } else {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
        out
    }

    pub(super) fn decode_trashinfo_path(raw: &str) -> PathBuf {
        let bytes = raw.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%'
                && i + 2 < bytes.len()
                && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
            {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        }
        PathBuf::from(OsString::from_vec(out))
    }

    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_trash_dir_always_returns_virtual_trash_on_supported_os() {
        let resolved = resolve_trash_dir();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            assert_eq!(resolved, Some(PathBuf::from(VIRTUAL_TRASH_PATH)));
            assert!(is_virtual_trash_path(resolved.as_deref().unwrap()));
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        assert!(resolved.is_none());
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn detects_virtual_windows_and_linux_trash_paths() {
        assert!(is_potential_trash_target(Path::new(VIRTUAL_TRASH_PATH)));
        assert!(is_potential_trash_target(Path::new("trash://note.txt")));
        assert!(is_potential_trash_target(Path::new(
            r"C:\$Recycle.Bin\S-1-5-21-100\$R123ABC.txt"
        )));
        assert!(is_potential_trash_target(Path::new(
            r"D:\$RECYCLE.BIN\S-1-5-21-100\$R999.dir"
        )));
        assert!(is_potential_trash_target(Path::new(
            "::{645FF040-5081-101B-9F08-00AA002F954E}\\item"
        )));
        assert!(is_potential_trash_target(Path::new(
            "/home/user/.local/share/Trash/files/doc.pdf"
        )));
        assert!(is_potential_trash_target(Path::new(
            "/mnt/dados/.Trash-1000/files/archive.zip"
        )));
        assert!(!is_potential_trash_target(Path::new(
            "/home/user/Documents/doc.pdf"
        )));
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn exact_trash_item_matching_prevents_twin_collisions() {
        let item_a = trash::TrashItem {
            id: std::ffi::OsString::from(if cfg!(target_os = "linux") {
                "/home/user/.local/share/Trash/info/report.txt.trashinfo"
            } else {
                r"C:\$Recycle.Bin\S-1-5-21\$RAAA.txt"
            }),
            name: std::ffi::OsString::from("report.txt"),
            original_parent: PathBuf::from(if cfg!(target_os = "linux") {
                "/home/user/Docs"
            } else {
                r"C:\Users\Docs"
            }),
            time_deleted: 1_700_000_000,
        };
        let item_b = trash::TrashItem {
            id: std::ffi::OsString::from(if cfg!(target_os = "linux") {
                "/home/user/.local/share/Trash/info/report.2.txt.trashinfo"
            } else {
                r"C:\$Recycle.Bin\S-1-5-21\$RBBB.txt"
            }),
            name: std::ffi::OsString::from("report.txt"),
            original_parent: PathBuf::from(if cfg!(target_os = "linux") {
                "/home/user/Docs"
            } else {
                r"C:\Users\Docs"
            }),
            time_deleted: 1_700_000_100,
        };

        #[cfg(target_os = "linux")]
        let selected = Path::new("/home/user/.local/share/Trash/files/report.2.txt");
        #[cfg(target_os = "windows")]
        let selected = Path::new(r"C:\$Recycle.Bin\S-1-5-21\$RBBB.txt");

        assert!(!trash_item_matches_exact(&item_a, selected));
        assert!(trash_item_matches_exact(&item_b, selected));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn freedesktop_trash_roundtrip_handles_files_and_folders() {
        let root = std::env::temp_dir().join(format!(
            "vagnarok_trash_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let work_dir = root.join("work");
        let trash_dir = root.join("host_trash");
        std::fs::create_dir_all(&work_dir).expect("create work dir");

        let file_one = work_dir.join("hello world (1).txt");
        let file_two = work_dir.join("hello world (1).txt");
        let sub_folder = work_dir.join("my folder");
        std::fs::create_dir_all(&sub_folder).expect("create subfolder");
        std::fs::write(sub_folder.join("inside.bin"), b"123").expect("write nested file");
        std::fs::write(&file_one, b"first").expect("write first");

        assert_eq!(
            freedesktop_host::move_item_to_trash_dir(&file_one, &trash_dir)
                .expect("trash first file"),
            freedesktop_host::MoveOutcome::Moved
        );
        std::fs::write(&file_two, b"second").expect("write second");
        assert_eq!(
            freedesktop_host::move_item_to_trash_dir(&file_two, &trash_dir)
                .expect("trash twin file"),
            freedesktop_host::MoveOutcome::Moved
        );
        assert_eq!(
            freedesktop_host::move_item_to_trash_dir(&sub_folder, &trash_dir)
                .expect("trash folder"),
            freedesktop_host::MoveOutcome::Moved
        );

        let items = freedesktop_host::scan_trash_dir(&trash_dir);
        assert_eq!(items.len(), 3);

        let folder_item = items
            .iter()
            .find(|i| i.name == std::ffi::OsStr::new("my folder"))
            .expect("folder trash item");
        let backing_folder = freedesktop_host::resolve_trash_file_path(Path::new(&folder_item.id))
            .expect("resolve backing folder path");
        assert!(backing_folder.is_dir());

        let second_item = items
            .iter()
            .find(|i| i.id.to_string_lossy().contains(".2."))
            .expect("collision-renamed trash item");
        let backing_second = freedesktop_host::resolve_trash_file_path(Path::new(&second_item.id))
            .expect("resolve second backing file");
        assert_eq!(
            std::fs::read(&backing_second).expect("read second trashed content"),
            b"second"
        );

        // Restaura apenas o folder e verifica que o conteúdo voltou.
        trash::os_limited::restore_all(vec![folder_item.clone()]).expect("restore folder");
        assert!(sub_folder.join("inside.bin").is_file());

        // Expurga permanentemente os demais itens diretamente pelo id do trashinfo.
        let remaining = freedesktop_host::scan_trash_dir(&trash_dir);
        assert_eq!(remaining.len(), 2);
        trash::os_limited::purge_all(remaining).expect("purge remaining");
        assert!(freedesktop_host::scan_trash_dir(&trash_dir).is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
