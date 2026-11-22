//! Descoberta de aplicativos `.desktop`, leitura/gravação de `mimeapps.list` e execução no Linux / Flatpak.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use compact_str::CompactString;
use directories::BaseDirs;
use fm_core::{FsError, OpenWithApp, OpenWithAssociation, OpenWithCatalog};
use tokio_util::sync::CancellationToken;

use super::split_posix_command_tokens;
use crate::linux::{host_command, spawn_detached};

pub(super) fn discover_open_with_catalog(
    path: &Path,
    mime_hint: Option<&str>,
    cancel: &CancellationToken,
) -> Result<OpenWithCatalog, FsError> {
    let type_key = super::open_with_type_key_for_path(path, mime_hint);
    let mime_for_match = if type_key.contains('/') {
        Some(type_key.as_str())
    } else {
        None
    };

    let type_label = mime_for_match
        .and_then(lookup_mime_description)
        .unwrap_or_else(|| fallback_type_label(path, &type_key));

    let desktop_entries = scan_desktop_entries(cancel)?;
    let (default_ids, added_ids) = mime_for_match
        .map(read_mimeapps_associations)
        .unwrap_or_default();

    let default_id = default_ids
        .into_iter()
        .find(|id| desktop_entries.contains_key(id));

    let mut system_default: Option<OpenWithApp> = None;
    let mut recommended = Vec::new();
    let mut others = Vec::new();

    for (desktop_id, entry) in desktop_entries {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let is_default = default_id.as_deref() == Some(desktop_id.as_str());
        let matches_mime = mime_for_match.is_some_and(|mime| {
            entry.mime_types.iter().any(|m| m == mime)
                || added_ids.iter().any(|id| id == &desktop_id)
        });

        let app = OpenWithApp {
            id: CompactString::from(desktop_id.as_str()),
            display_name: CompactString::from(entry.name.as_str()),
            exec: entry.exec,
            icon_hint: entry.icon,
            is_recommended: matches_mime || is_default,
            is_default,
            is_custom: false,
            runs_in_terminal: entry.terminal,
        };

        if is_default {
            system_default = Some(app);
        } else if matches_mime {
            recommended.push(app);
        } else {
            others.push(app);
        }
    }

    recommended.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    others.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

    Ok(OpenWithCatalog {
        type_key: CompactString::from(type_key),
        type_label: CompactString::from(type_label),
        system_default,
        recommended,
        others,
    })
}

pub(super) fn save_system_default(
    type_key: &str,
    assoc: &OpenWithAssociation,
) -> Result<(), FsError> {
    if !type_key.contains('/') || !assoc.app_id.ends_with(".desktop") {
        return Ok(());
    }
    let Some(path) = user_mimeapps_write_path() else {
        return Ok(());
    };
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let updated = update_mimeapps_default(&existing, type_key, Some(&assoc.app_id));
    atomic_write_text(&path, &updated)?;
    // Sincronização best-effort com GLib/XDG no host (inclui Flatpak) para refletir de imediato.
    let _ = host_command("gio")
        .args(["mime", type_key, &assoc.app_id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

pub(super) fn reset_system_default(type_key: &str) -> Result<(), FsError> {
    if !type_key.contains('/') {
        return Ok(());
    }
    let Some(path) = user_mimeapps_write_path() else {
        return Ok(());
    };
    let Ok(existing) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let updated = update_mimeapps_default(&existing, type_key, None);
    atomic_write_text(&path, &updated)
}

pub(super) fn launch_open_with(path: &Path, target: &OpenWithAssociation) -> Result<(), FsError> {
    let raw_exec = target.exec.trim();
    if raw_exec.is_empty() {
        return Err(FsError::Io {
            path: path.to_path_buf(),
            message: "comando de aplicativo vazio".to_owned(),
        });
    }

    let raw_tokens = split_posix_command_tokens(raw_exec);
    let expanded = expand_freedesktop_exec(&raw_tokens, path, &target.display_name);
    let Some((program, args)) = expanded.split_first() else {
        return Err(FsError::Io {
            path: path.to_path_buf(),
            message: "não foi possível resolver o executável do aplicativo".to_owned(),
        });
    };

    if target.runs_in_terminal {
        return launch_in_host_terminal(path, program, args);
    }

    let mut cmd = host_command(program);
    cmd.args(args);
    spawn_detached(&mut cmd).map_err(|err| FsError::Io {
        path: path.to_path_buf(),
        message: format!("falha ao abrir com '{}': {err}", target.display_name),
    })
}

fn launch_in_host_terminal(path: &Path, program: &str, args: &[String]) -> Result<(), FsError> {
    let mut full = vec![program.to_owned()];
    full.extend_from_slice(args);
    for (term, flag) in [
        ("xdg-terminal-exec", None),
        ("x-terminal-emulator", Some("-e")),
        ("gnome-terminal", Some("--")),
        ("konsole", Some("-e")),
        ("xfce4-terminal", Some("-x")),
        ("kitty", None),
        ("alacritty", Some("-e")),
        ("xterm", Some("-e")),
    ] {
        let mut cmd = host_command(term);
        if let Some(f) = flag {
            cmd.arg(f);
        }
        cmd.args(&full);
        if spawn_detached(&mut cmd).is_ok() {
            return Ok(());
        }
    }
    Err(FsError::Io {
        path: path.to_path_buf(),
        message: "nenhum terminal encontrado no sistema para executar o aplicativo".to_owned(),
    })
}

#[derive(Debug, Clone)]
struct ParsedDesktopEntry {
    name: String,
    exec: String,
    icon: Option<String>,
    mime_types: Vec<String>,
    terminal: bool,
}

fn desktop_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let in_flatpak = std::env::var_os("FLATPAK_ID").is_some();
    if let Some(base) = BaseDirs::new() {
        if in_flatpak {
            dirs.push(base.home_dir().join(".local/share/applications"));
            dirs.push(
                base.home_dir()
                    .join(".local/share/flatpak/exports/share/applications"),
            );
        } else {
            dirs.push(base.data_local_dir().join("applications"));
            dirs.push(
                base.data_local_dir()
                    .join("flatpak/exports/share/applications"),
            );
        }
    }
    if in_flatpak {
        dirs.push(PathBuf::from(
            "/run/host/var/lib/flatpak/exports/share/applications",
        ));
        dirs.push(PathBuf::from("/run/host/usr/local/share/applications"));
        dirs.push(PathBuf::from("/run/host/usr/share/applications"));
    }
    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for base in std::env::split_paths(&xdg_data_dirs) {
            dirs.push(base.join("applications"));
        }
    }
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/usr/share/applications"));

    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for dir in dirs {
        if seen.insert(dir.clone()) && dir.is_dir() {
            unique.push(dir);
        }
    }
    unique
}

fn scan_desktop_entries(
    cancel: &CancellationToken,
) -> Result<HashMap<String, ParsedDesktopEntry>, FsError> {
    let mut map = HashMap::new();
    let locale_prefs = preferred_locale_suffixes();

    for root in desktop_search_dirs() {
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            if cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for item in entries.flatten() {
                let path = item.path();
                let Ok(ft) = item.file_type() else {
                    continue;
                };
                if ft.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(&root) else {
                    continue;
                };
                let desktop_id = desktop_id_from_relative(rel);
                if map.contains_key(&desktop_id) {
                    continue;
                }
                let Ok(raw) = fs::read_to_string(&path) else {
                    continue;
                };
                if let Some(parsed) = parse_desktop_entry(&raw, &locale_prefs) {
                    map.insert(desktop_id, parsed);
                }
            }
        }
    }

    Ok(map)
}

fn desktop_id_from_relative(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

fn preferred_locale_suffixes() -> Vec<String> {
    let mut out = Vec::new();
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let clean = val.split('.').next().unwrap_or(&val).trim();
            if !clean.is_empty() && clean != "C" && clean != "POSIX" {
                out.push(clean.to_owned());
                if let Some((lang, _)) = clean.split_once('_') {
                    out.push(lang.to_owned());
                }
                break;
            }
        }
    }
    out
}

fn parse_desktop_entry(raw: &str, locale_prefs: &[String]) -> Option<ParsedDesktopEntry> {
    let mut in_desktop_entry = false;
    let mut seen_entry_header = false;
    let mut entry_type: Option<String> = None;
    let mut no_display = false;
    let mut hidden = false;
    let mut terminal = false;
    let mut exec: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut name_default: Option<String> = None;
    let mut name_localized: Vec<(String, String)> = Vec::new();
    let mut mime_types = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if trimmed == "[Desktop Entry]" {
                in_desktop_entry = true;
                seen_entry_header = true;
            } else if seen_entry_header {
                break;
            }
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match key {
            "Type" => entry_type = Some(value.to_owned()),
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
            "Exec" if !value.is_empty() => exec = Some(value.to_owned()),
            "Icon" if !value.is_empty() => icon = Some(value.to_owned()),
            "Name" if !value.is_empty() => name_default = Some(value.to_owned()),
            "MimeType" => {
                for mime in value.split(';') {
                    let m = mime.trim().to_ascii_lowercase();
                    if !m.is_empty() {
                        mime_types.push(m);
                    }
                }
            }
            _ => {
                if let Some(loc) = key.strip_prefix("Name[").and_then(|s| s.strip_suffix(']'))
                    && !value.is_empty()
                {
                    name_localized.push((loc.to_owned(), value.to_owned()));
                }
            }
        }
    }

    if entry_type.as_deref() != Some("Application") || no_display || hidden {
        return None;
    }

    let mut chosen_name = None;
    for pref in locale_prefs {
        if let Some((_, val)) = name_localized.iter().find(|(loc, _)| loc == pref) {
            chosen_name = Some(val.clone());
            break;
        }
    }
    let name = chosen_name.or(name_default)?;
    let exec = exec?;

    Some(ParsedDesktopEntry {
        name,
        exec,
        icon,
        mime_types,
        terminal,
    })
}

fn mimeapps_read_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let in_flatpak = std::env::var_os("FLATPAK_ID").is_some();
    if let Some(base) = BaseDirs::new() {
        if in_flatpak {
            paths.push(base.home_dir().join(".config/mimeapps.list"));
            paths.push(
                base.home_dir()
                    .join(".local/share/applications/mimeapps.list"),
            );
        } else {
            paths.push(base.config_dir().join("mimeapps.list"));
            paths.push(base.data_local_dir().join("applications/mimeapps.list"));
        }
    }
    if in_flatpak {
        paths.push(PathBuf::from("/run/host/etc/xdg/mimeapps.list"));
        paths.push(PathBuf::from(
            "/run/host/usr/share/applications/mimeapps.list",
        ));
        paths.push(PathBuf::from(
            "/run/host/usr/share/applications/defaults.list",
        ));
    }
    paths.push(PathBuf::from("/etc/xdg/mimeapps.list"));
    paths.push(PathBuf::from("/usr/share/applications/mimeapps.list"));
    paths.push(PathBuf::from("/usr/share/applications/defaults.list"));
    paths
}

fn user_mimeapps_write_path() -> Option<PathBuf> {
    let base = BaseDirs::new()?;
    if std::env::var_os("FLATPAK_ID").is_some() {
        Some(base.home_dir().join(".config/mimeapps.list"))
    } else {
        Some(base.config_dir().join("mimeapps.list"))
    }
}

fn read_mimeapps_associations(mime: &str) -> (Vec<String>, Vec<String>) {
    let mut defaults = Vec::new();
    let mut added = Vec::new();
    let mut removed = HashSet::new();

    for path in mimeapps_read_paths() {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let (d, a, r) = parse_mimeapps_for_mime(&raw, mime);
        for item in r {
            removed.insert(item);
        }
        for item in d {
            if !removed.contains(&item) && !defaults.contains(&item) {
                defaults.push(item);
            }
        }
        for item in a {
            if !removed.contains(&item) && !added.contains(&item) {
                added.push(item);
            }
        }
    }
    (defaults, added)
}

fn parse_mimeapps_for_mime(
    raw: &str,
    target_mime: &str,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut defaults = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut section = "";

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed;
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case(target_mime) {
            continue;
        }
        let target_vec = match section {
            "[Default Applications]" => &mut defaults,
            "[Added Associations]" => &mut added,
            "[Removed Associations]" => &mut removed,
            _ => continue,
        };
        for item in value.split(';') {
            let id = item.trim();
            if !id.is_empty() && !target_vec.iter().any(|existing| existing == id) {
                target_vec.push(id.to_owned());
            }
        }
    }
    (defaults, added, removed)
}

fn update_mimeapps_default(existing: &str, mime: &str, desktop_id: Option<&str>) -> String {
    let mut sections: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    let mut order: HashMap<String, usize> = HashMap::new();
    let mut current_sec: Option<usize> = None;

    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec_name = trimmed.to_owned();
            let idx = *order.entry(sec_name.clone()).or_insert_with(|| {
                sections.push((sec_name, BTreeMap::new()));
                sections.len() - 1
            });
            current_sec = Some(idx);
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let idx = match current_sec {
                Some(i) => i,
                None => {
                    let i = *order
                        .entry("[Default Applications]".to_owned())
                        .or_insert_with(|| {
                            sections.push(("[Default Applications]".to_owned(), BTreeMap::new()));
                            sections.len() - 1
                        });
                    current_sec = Some(i);
                    i
                }
            };
            if let Some((_, map)) = sections.get_mut(idx) {
                map.insert(k.trim().to_owned(), v.trim().to_owned());
            }
        }
    }

    let def_idx = *order
        .entry("[Default Applications]".to_owned())
        .or_insert_with(|| {
            sections.push(("[Default Applications]".to_owned(), BTreeMap::new()));
            sections.len() - 1
        });

    if let Some(id) = desktop_id {
        sections[def_idx]
            .1
            .insert(mime.to_owned(), format!("{id};"));

        let add_idx = *order
            .entry("[Added Associations]".to_owned())
            .or_insert_with(|| {
                sections.push(("[Added Associations]".to_owned(), BTreeMap::new()));
                sections.len() - 1
            });
        let entry = sections[add_idx].1.entry(mime.to_owned()).or_default();
        let mut items: Vec<String> = entry
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if !items.iter().any(|existing_id| existing_id == id) {
            items.insert(0, id.to_owned());
        }
        *entry = format!("{};", items.join(";"));

        if let Some(&rem_idx) = order.get("[Removed Associations]")
            && let Some(rem_val) = sections[rem_idx].1.get_mut(mime)
        {
            let kept: Vec<&str> = rem_val
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty() && *s != id)
                .collect();
            if kept.is_empty() {
                sections[rem_idx].1.remove(mime);
            } else {
                *rem_val = format!("{};", kept.join(";"));
            }
        }
    } else {
        sections[def_idx].1.remove(mime);
    }

    let mut out = String::new();
    for (sec_name, map) in sections {
        if map.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&sec_name);
        out.push('\n');
        for (k, v) in map {
            out.push_str(&format!("{k}={v}\n"));
        }
    }
    out
}

fn atomic_write_text(path: &Path, content: &str) -> Result<(), FsError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| crate::map_io_err(parent, e))?;
    }
    let tmp = path.with_extension("list.tmp");
    fs::write(&tmp, content).map_err(|e| crate::map_io_err(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        crate::map_io_err(path, e)
    })
}

fn lookup_mime_description(mime: &str) -> Option<String> {
    let (top, sub) = mime.split_once('/')?;
    let mut roots = Vec::new();
    if std::env::var_os("FLATPAK_ID").is_some() {
        roots.push(PathBuf::from("/run/host/usr/share/mime"));
    }
    if let Some(base) = BaseDirs::new() {
        roots.push(base.data_local_dir().join("mime"));
    }
    roots.push(PathBuf::from("/usr/share/mime"));

    let locale_prefs = preferred_locale_suffixes();
    for root in roots {
        let xml_path = root.join(top).join(format!("{sub}.xml"));
        let Ok(xml) = fs::read_to_string(&xml_path) else {
            continue;
        };
        if let Some(comment) = extract_xml_comment(&xml, &locale_prefs) {
            return Some(comment);
        }
    }
    None
}

fn extract_xml_comment(xml: &str, locale_prefs: &[String]) -> Option<String> {
    let mut default_comment: Option<String> = None;
    let mut localized: Vec<(String, String)> = Vec::new();
    let mut rest = xml;

    while let Some(start) = rest.find("<comment") {
        let after_tag = &rest[start + "<comment".len()..];
        let Some(gt) = after_tag.find('>') else {
            break;
        };
        let attrs = &after_tag[..gt];
        let content_and_tail = &after_tag[gt + 1..];
        let Some(end) = content_and_tail.find("</comment>") else {
            break;
        };
        let text = content_and_tail[..end].trim();
        if !text.is_empty() {
            if let Some(lang_attr) = attrs
                .split("xml:lang=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
            {
                localized.push((lang_attr.to_owned(), text.to_owned()));
            } else if attrs.trim().is_empty() && default_comment.is_none() {
                default_comment = Some(text.to_owned());
            }
        }
        rest = &content_and_tail[end + "</comment>".len()..];
    }

    for pref in locale_prefs {
        let hyphen = pref.replace('_', "-");
        if let Some((_, val)) = localized
            .iter()
            .find(|(lang, _)| lang.eq_ignore_ascii_case(pref) || lang.eq_ignore_ascii_case(&hyphen))
        {
            return Some(val.clone());
        }
    }
    default_comment
}

fn fallback_type_label(path: &Path, type_key: &str) -> String {
    if let Some(ext) = path.extension().and_then(|e| e.to_str())
        && !ext.is_empty()
    {
        return format!("{} ({type_key})", ext.to_ascii_uppercase());
    }
    type_key.to_owned()
}

fn expand_freedesktop_exec(tokens: &[String], file_path: &Path, app_name: &str) -> Vec<String> {
    let file_str = file_path.to_string_lossy().into_owned();
    let file_uri = path_to_file_uri(file_path);
    let mut out = Vec::with_capacity(tokens.len() + 1);
    let mut injected_target = false;

    for token in tokens {
        match token.as_str() {
            "%f" | "%F" => {
                out.push(file_str.clone());
                injected_target = true;
            }
            "%u" | "%U" => {
                out.push(file_uri.clone());
                injected_target = true;
            }
            "%i" | "%k" => {}
            "%c" => out.push(app_name.to_owned()),
            _ if token.contains('%') => {
                let mut replaced = token.replace("%%", "%");
                if replaced.contains("%f") {
                    replaced = replaced.replace("%f", &file_str);
                    injected_target = true;
                }
                if replaced.contains("%F") {
                    replaced = replaced.replace("%F", &file_str);
                    injected_target = true;
                }
                if replaced.contains("%u") {
                    replaced = replaced.replace("%u", &file_uri);
                    injected_target = true;
                }
                if replaced.contains("%U") {
                    replaced = replaced.replace("%U", &file_uri);
                    injected_target = true;
                }
                if replaced.contains("%c") {
                    replaced = replaced.replace("%c", app_name);
                }
                out.push(replaced);
            }
            _ => out.push(token.clone()),
        }
    }

    if !injected_target {
        out.push(file_str);
    }
    out
}

fn path_to_file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        let keep = matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/'
        );
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktop_entry_with_localized_name_and_mimes() {
        let raw = r#"
[Desktop Entry]
Type=Application
Name=VLC media player
Name[pt_BR]=Reprodutor de Mídias VLC
Icon=vlc
Exec=/usr/bin/vlc --started-from-file %U
MimeType=video/mp4;video/x-matroska;
Terminal=false

[Desktop Action NewWindow]
Name=New Window
Exec=/usr/bin/vlc --new
"#;
        let parsed = parse_desktop_entry(raw, &["pt_BR".to_owned(), "pt".to_owned()])
            .expect("desktop entry");
        assert_eq!(parsed.name, "Reprodutor de Mídias VLC");
        assert_eq!(parsed.exec, "/usr/bin/vlc --started-from-file %U");
        assert_eq!(parsed.icon.as_deref(), Some("vlc"));
        assert!(parsed.mime_types.contains(&"video/mp4".to_owned()));
        assert!(!parsed.terminal);
    }

    #[test]
    fn updates_and_resets_mimeapps_default_preserving_other_entries() {
        let initial = "[Default Applications]\ntext/plain=gedit.desktop;\n\n[Added Associations]\nvideo/mp4=mpv.desktop;\n";
        let updated = update_mimeapps_default(initial, "video/mp4", Some("vlc.desktop"));
        let (defs, added, _) = parse_mimeapps_for_mime(&updated, "video/mp4");
        assert_eq!(defs, vec!["vlc.desktop"]);
        assert_eq!(added, vec!["vlc.desktop", "mpv.desktop"]);

        let reset = update_mimeapps_default(&updated, "video/mp4", None);
        let (defs_after, _, _) = parse_mimeapps_for_mime(&reset, "video/mp4");
        assert!(defs_after.is_empty());
        let (text_defs, _, _) = parse_mimeapps_for_mime(&reset, "text/plain");
        assert_eq!(text_defs, vec!["gedit.desktop"]);

        let with_removed = "[Default Applications]\napplication/pdf=xreader.desktop;\n\n[Removed Associations]\napplication/pdf=ai.storyteller.pdfcraft.desktop;evince.desktop;\n";
        let restored = update_mimeapps_default(
            with_removed,
            "application/pdf",
            Some("ai.storyteller.pdfcraft.desktop"),
        );
        let (pdf_defs, pdf_added, pdf_rem) = parse_mimeapps_for_mime(&restored, "application/pdf");
        assert_eq!(pdf_defs, vec!["ai.storyteller.pdfcraft.desktop"]);
        assert_eq!(pdf_added, vec!["ai.storyteller.pdfcraft.desktop"]);
        assert_eq!(pdf_rem, vec!["evince.desktop"]);
    }

    #[test]
    fn expands_exec_field_codes_and_appends_when_missing() {
        let p = Path::new("/home/user/My Video.mp4");
        let with_uri = expand_freedesktop_exec(
            &[
                "/usr/bin/vlc".into(),
                "--started-from-file".into(),
                "%U".into(),
                "%i".into(),
            ],
            p,
            "VLC",
        );
        assert_eq!(
            with_uri,
            vec![
                "/usr/bin/vlc",
                "--started-from-file",
                "file:///home/user/My%20Video.mp4"
            ]
        );

        let custom = expand_freedesktop_exec(&["mpv".into(), "--fs".into()], p, "mpv");
        assert_eq!(custom, vec!["mpv", "--fs", "/home/user/My Video.mp4"]);
    }
}
