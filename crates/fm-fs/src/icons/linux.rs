//! Descoberta de ícones do tema Freedesktop instalado no Linux (nativo e Flatpak).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use directories::BaseDirs;
use fm_core::{DriveKind, SystemIconBucket, SystemIconKey, SystemIconPayload, SystemPlaceKind};

use crate::linux::host_command;

#[derive(Debug, Clone)]
struct ThemeDirMeta {
    rel_path: String,
    effective_px: u32,
    min_px: u32,
    max_px: u32,
    is_scalable: bool,
}

#[derive(Debug)]
struct ThemeEntry {
    root_dirs: Vec<PathBuf>,
    dirs: Vec<ThemeDirMeta>,
}

#[derive(Debug, Default)]
struct MimeTable {
    ext_to_mime: HashMap<String, (u32, String)>,
    mime_to_generic: HashMap<String, String>,
}

struct LinuxIconIndex {
    themes: Vec<ThemeEntry>,
    pixmaps_dirs: Vec<PathBuf>,
    mime: MimeTable,
}

static ICON_INDEX: OnceLock<LinuxIconIndex> = OnceLock::new();

pub(super) fn resolve_system_icon(
    key: &SystemIconKey,
    bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    if let SystemIconKey::FilePath(path) = key {
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("desktop"))
            && let Some(icon_name) = parse_desktop_icon_name(path)
            && let Some(payload) = resolve_icon_by_hint(&icon_name, bucket)
        {
            return Some(payload);
        }
        let fallback_key = path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.is_empty())
            .map(|e| SystemIconKey::Extension(e.to_ascii_lowercase().into()))
            .unwrap_or(SystemIconKey::GenericFile);
        return resolve_system_icon(&fallback_key, bucket);
    }
    let index = ICON_INDEX.get_or_init(build_icon_index);
    let target_px = bucket.target_px();
    let names = candidate_icon_names(key, &index.mime);
    lookup_icon_file(index, &names, target_px).map(SystemIconPayload::File)
}

fn parse_desktop_icon_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_desktop_entry = trimmed == "[Desktop Entry]";
            continue;
        }
        if in_desktop_entry && let Some(icon) = trimmed.strip_prefix("Icon=") {
            let val = icon.trim();
            if !val.is_empty() {
                return Some(val.to_owned());
            }
        }
    }
    None
}

/// Resolve ícone por nome FreeDesktop ou caminho absoluto para um aplicativo instalado.
pub(super) fn resolve_icon_by_hint(
    icon_hint: &str,
    bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    let candidate = Path::new(icon_hint);
    if candidate.is_absolute() {
        if candidate.is_file() {
            return Some(SystemIconPayload::File(candidate.to_path_buf()));
        }
        if std::env::var_os("FLATPAK_ID").is_some()
            && let Ok(rel) = candidate.strip_prefix("/")
        {
            let host_candidate = Path::new("/run/host").join(rel);
            if host_candidate.is_file() {
                return Some(SystemIconPayload::File(host_candidate));
            }
        }
    }

    let index = ICON_INDEX.get_or_init(build_icon_index);
    let mut names = vec![icon_hint.to_owned()];
    if let Some(stem) = Path::new(icon_hint)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| *s != icon_hint && !s.is_empty())
    {
        names.push(stem.to_owned());
    }
    lookup_icon_file(index, &names, bucket.target_px()).map(SystemIconPayload::File)
}

/// Resolve o MIME type conhecido pela tabela `globs2` do sistema para um arquivo.
pub(crate) fn lookup_mime_for_path(path: &Path) -> Option<String> {
    let index = ICON_INDEX.get_or_init(build_icon_index);
    let file_name = path.file_name()?.to_str()?;
    let lower = file_name.to_ascii_lowercase();
    let mut best: Option<(u32, String)> = None;
    for (idx, _) in lower.match_indices('.') {
        let ext = &lower[idx + 1..];
        if ext.is_empty() {
            continue;
        }
        if let Some((weight, mime)) = index.mime.ext_to_mime.get(ext) {
            match &best {
                Some((best_w, _)) if *best_w >= *weight => {}
                _ => best = Some((*weight, mime.clone())),
            }
        }
    }
    best.map(|(_, mime)| mime)
}

fn build_icon_index() -> LinuxIconIndex {
    let search_roots = icon_theme_search_roots();
    let active_theme = detect_active_icon_theme();
    let theme_names = collect_theme_chain(&active_theme, &search_roots);

    let mut themes = Vec::with_capacity(theme_names.len());
    for theme_name in theme_names {
        let mut root_dirs = Vec::new();
        let mut parsed_dirs = Vec::new();
        for base in &search_roots {
            let candidate = base.join(&theme_name);
            if candidate.is_dir() {
                if parsed_dirs.is_empty() {
                    let index_file = candidate.join("index.theme");
                    if let Ok(raw) = std::fs::read_to_string(&index_file) {
                        parsed_dirs = parse_theme_dirs(&raw);
                    }
                }
                root_dirs.push(candidate);
            }
        }
        if !root_dirs.is_empty() {
            themes.push(ThemeEntry {
                root_dirs,
                dirs: parsed_dirs,
            });
        }
    }

    let pixmaps_dirs = pixmap_search_roots();
    let mime = load_mime_table();

    LinuxIconIndex {
        themes,
        pixmaps_dirs,
        mime,
    }
}

fn icon_theme_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let in_flatpak = std::env::var_os("FLATPAK_ID").is_some();

    if in_flatpak {
        roots.push(PathBuf::from("/run/host/user-share/icons"));
    }
    if let Some(base) = BaseDirs::new() {
        roots.push(base.data_local_dir().join("icons"));
        roots.push(base.home_dir().join(".icons"));
    }
    if in_flatpak {
        roots.push(PathBuf::from("/run/host/share/icons"));
    }
    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in std::env::split_paths(&xdg_data_dirs) {
            let candidate = dir.join("icons");
            if !roots.contains(&candidate) {
                roots.push(candidate);
            }
        }
    }
    for sys in ["/usr/local/share/icons", "/usr/share/icons"] {
        let p = PathBuf::from(sys);
        if !roots.contains(&p) {
            roots.push(p);
        }
    }
    roots
}

fn pixmap_search_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if std::env::var_os("FLATPAK_ID").is_some() {
        out.push(PathBuf::from("/run/host/share/pixmaps"));
    }
    out.push(PathBuf::from("/usr/share/pixmaps"));
    out
}

fn detect_active_icon_theme() -> String {
    if let Some(base) = BaseDirs::new() {
        let cfg = base.config_dir();
        for rel in ["gtk-4.0/settings.ini", "gtk-3.0/settings.ini"] {
            if let Ok(raw) = std::fs::read_to_string(cfg.join(rel))
                && let Some(theme) = parse_icon_theme_from_gtk_ini(&raw)
            {
                return theme;
            }
        }
        if let Ok(raw) = std::fs::read_to_string(cfg.join("kdeglobals"))
            && let Some(theme) = parse_icon_theme_from_kdeglobals(&raw)
        {
            return theme;
        }
    }

    for schema in [
        "org.cinnamon.desktop.interface",
        "org.gnome.desktop.interface",
        "org.mate.interface",
    ] {
        if let Ok(out) = host_command("gsettings")
            .args(["get", schema, "icon-theme"])
            .output()
            && out.status.success()
        {
            let val = String::from_utf8_lossy(&out.stdout)
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .trim()
                .to_owned();
            if !val.is_empty() {
                return val;
            }
        }
    }

    "Adwaita".to_owned()
}

fn parse_icon_theme_from_gtk_ini(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once('=')
            && key.trim() == "gtk-icon-theme-name"
        {
            let cleaned = val.trim().trim_matches('"').trim_matches('\'').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_owned());
            }
        }
    }
    None
}

fn parse_icon_theme_from_kdeglobals(raw: &str) -> Option<String> {
    let mut in_icons = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_icons = trimmed == "[Icons]";
            continue;
        }
        if in_icons
            && let Some((key, val)) = trimmed.split_once('=')
            && key.trim() == "Theme"
        {
            let cleaned = val.trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_owned());
            }
        }
    }
    None
}

fn collect_theme_chain(primary: &str, roots: &[PathBuf]) -> Vec<String> {
    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(primary.to_owned());

    while let Some(name) = queue.pop_front() {
        if !visited.insert(name.clone()) {
            continue;
        }
        chain.push(name.clone());

        for base in roots {
            let idx = base.join(&name).join("index.theme");
            if let Ok(raw) = std::fs::read_to_string(idx) {
                for parent in parse_theme_inherits(&raw) {
                    if !visited.contains(&parent) {
                        queue.push_back(parent);
                    }
                }
                break;
            }
        }
    }

    for fallback in ["Adwaita", "hicolor"] {
        if visited.insert(fallback.to_owned()) {
            chain.push(fallback.to_owned());
        }
    }
    chain
}

fn parse_theme_inherits(raw: &str) -> Vec<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some((key, val)) = trimmed.split_once('=')
            && key.trim() == "Inherits"
        {
            return val
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
    }
    Vec::new()
}

fn parse_theme_dirs(raw: &str) -> Vec<ThemeDirMeta> {
    let mut out = Vec::new();
    let mut current_section: Option<String> = None;
    let mut size: Option<u32> = None;
    let mut scale: u32 = 1;
    let mut min_size: Option<u32> = None;
    let mut max_size: Option<u32> = None;
    let mut dir_type = String::new();
    let mut context = String::new();

    let flush = |out: &mut Vec<ThemeDirMeta>,
                 sec: Option<String>,
                 size: Option<u32>,
                 scale: u32,
                 min_size: Option<u32>,
                 max_size: Option<u32>,
                 dir_type: &str,
                 context: &str| {
        let Some(rel_path) = sec else {
            return;
        };
        if rel_path == "Icon Theme"
            || rel_path.contains("symbolic")
            || matches!(
                context,
                "Actions" | "Animations" | "Emotes" | "Status" | "International"
            )
        {
            return;
        }
        let Some(sz) = size else {
            return;
        };
        let eff = sz.saturating_mul(scale.max(1));
        let is_scalable = dir_type.eq_ignore_ascii_case("Scalable");
        out.push(ThemeDirMeta {
            rel_path,
            effective_px: eff,
            min_px: min_size.unwrap_or(sz).saturating_mul(scale.max(1)),
            max_px: max_size.unwrap_or(sz).saturating_mul(scale.max(1)),
            is_scalable,
        });
    };

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            flush(
                &mut out,
                current_section.take(),
                size.take(),
                scale,
                min_size.take(),
                max_size.take(),
                &dir_type,
                &context,
            );
            current_section = Some(trimmed[1..trimmed.len() - 1].to_owned());
            scale = 1;
            dir_type.clear();
            context.clear();
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "Size" => size = val.parse::<u32>().ok(),
                "Scale" => scale = val.parse::<u32>().unwrap_or(1),
                "MinSize" => min_size = val.parse::<u32>().ok(),
                "MaxSize" => max_size = val.parse::<u32>().ok(),
                "Type" => dir_type = val.to_owned(),
                "Context" => context = val.to_owned(),
                _ => {}
            }
        }
    }
    flush(
        &mut out,
        current_section,
        size,
        scale,
        min_size,
        max_size,
        &dir_type,
        &context,
    );
    out
}

fn load_mime_table() -> MimeTable {
    let mut table = MimeTable::default();
    let mut mime_roots = Vec::new();
    if std::env::var_os("FLATPAK_ID").is_some() {
        mime_roots.push(PathBuf::from("/run/host/usr/share/mime"));
    }
    if let Some(base) = BaseDirs::new() {
        mime_roots.push(base.data_local_dir().join("mime"));
    }
    mime_roots.push(PathBuf::from("/usr/share/mime"));

    for root in mime_roots {
        if let Ok(raw) = std::fs::read_to_string(root.join("globs2")) {
            parse_globs2_into(&raw, &mut table.ext_to_mime);
        }
        if let Ok(raw) = std::fs::read_to_string(root.join("generic-icons")) {
            parse_generic_icons_into(&raw, &mut table.mime_to_generic);
        }
        if !table.ext_to_mime.is_empty() {
            break;
        }
    }
    table
}

fn parse_globs2_into(raw: &str, out: &mut HashMap<String, (u32, String)>) {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(4, ':');
        let Some(weight_str) = parts.next() else {
            continue;
        };
        let Some(mime) = parts.next() else {
            continue;
        };
        let Some(pattern) = parts.next() else {
            continue;
        };
        let Ok(weight) = weight_str.parse::<u32>() else {
            continue;
        };
        let Some(ext) = pattern.strip_prefix("*.") else {
            continue;
        };
        // Ignora globs compostos com outros wildcards e mantém só extensões diretas (incluindo `.tar.gz`).
        if ext.is_empty() || ext.contains(['*', '?', '[']) {
            continue;
        }
        let ext_lower = ext.to_ascii_lowercase();
        match out.get(&ext_lower) {
            Some((prev_w, _)) if *prev_w >= weight => {}
            _ => {
                out.insert(ext_lower, (weight, mime.to_owned()));
            }
        }
    }
}

fn parse_generic_icons_into(raw: &str, out: &mut HashMap<String, String>) {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((mime, icon)) = trimmed.split_once(':') {
            let m = mime.trim();
            let ic = icon.trim();
            if !m.is_empty() && !ic.is_empty() {
                out.entry(m.to_owned()).or_insert_with(|| ic.to_owned());
            }
        }
    }
}

fn candidate_icon_names(key: &SystemIconKey, mime: &MimeTable) -> Vec<String> {
    let mut names = Vec::new();
    match key {
        SystemIconKey::Directory => {
            names.push("folder".to_owned());
            names.push("inode-directory".to_owned());
        }
        SystemIconKey::Archive => {
            names.push("package-x-generic".to_owned());
            names.push("application-x-archive".to_owned());
            names.push("application-zip".to_owned());
        }
        SystemIconKey::SpecialPlace(place) => {
            let primary = match place {
                SystemPlaceKind::Home => "user-home",
                SystemPlaceKind::Desktop => "user-desktop",
                SystemPlaceKind::Documents => "folder-documents",
                SystemPlaceKind::Downloads => "folder-download",
                SystemPlaceKind::Music => "folder-music",
                SystemPlaceKind::Pictures => "folder-pictures",
                SystemPlaceKind::Videos => "folder-videos",
                SystemPlaceKind::Public => "folder-publicshare",
                SystemPlaceKind::Templates => "folder-templates",
                SystemPlaceKind::Trash => "user-trash",
            };
            names.push(primary.to_owned());
            if matches!(place, SystemPlaceKind::Trash) {
                names.push("user-trash-full".to_owned());
            } else {
                names.push("folder".to_owned());
            }
        }
        SystemIconKey::Drive(kind) => {
            match kind {
                DriveKind::InternalSsd => {
                    names.push("drive-harddisk-solidstate".to_owned());
                    names.push("drive-harddisk".to_owned());
                }
                DriveKind::SecondaryDisk => names.push("drive-harddisk".to_owned()),
                DriveKind::Removable => names.push("drive-removable-media".to_owned()),
                DriveKind::OpticalOrIso => {
                    names.push("media-optical".to_owned());
                    names.push("drive-optical".to_owned());
                }
                DriveKind::Network => names.push("network-server".to_owned()),
            }
            names.push("drive-harddisk".to_owned());
        }
        SystemIconKey::GenericFile | SystemIconKey::FilePath(_) => {
            names.push("text-x-generic".to_owned());
            names.push("application-octet-stream".to_owned());
        }
        SystemIconKey::Extension(ext) => {
            let lower = ext.to_ascii_lowercase();
            if let Some((_, mime_str)) = mime.ext_to_mime.get(lower.as_str()) {
                let exact = mime_str.replace('/', "-");
                names.push(exact.clone());
                names.push(format!("gnome-mime-{exact}"));
                if let Some(generic) = mime.mime_to_generic.get(mime_str) {
                    names.push(generic.clone());
                }
                if let Some((top, _)) = mime_str.split_once('/') {
                    let top_generic = match top {
                        "image" => "image-x-generic",
                        "audio" => "audio-x-generic",
                        "video" => "video-x-generic",
                        "text" => "text-x-generic",
                        "font" => "font-x-generic",
                        _ => "application-x-generic",
                    };
                    names.push(top_generic.to_owned());
                }
            }
            match lower.as_str() {
                "rs" | "py" | "js" | "ts" | "sh" | "bash" => names.push("text-x-script".to_owned()),
                "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" => {
                    names.push("package-x-generic".to_owned());
                }
                "pdf" => names.push("application-pdf".to_owned()),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => {
                    names.push("image-x-generic".to_owned());
                }
                "mp3" | "flac" | "ogg" | "wav" => names.push("audio-x-generic".to_owned()),
                "mp4" | "mkv" | "webm" | "avi" => names.push("video-x-generic".to_owned()),
                _ => {}
            }
            names.push("text-x-generic".to_owned());
        }
    }
    names
}

fn lookup_icon_file(index: &LinuxIconIndex, names: &[String], target_px: u32) -> Option<PathBuf> {
    for name in names {
        for theme in &index.themes {
            let mut ranked_dirs: Vec<(&ThemeDirMeta, u32)> = theme
                .dirs
                .iter()
                .map(|d| (d, dir_distance_score(d, target_px)))
                .collect();
            ranked_dirs.sort_by_key(|(_, score)| *score);

            for (dir_meta, _) in ranked_dirs {
                for root in &theme.root_dirs {
                    let dir = root.join(&dir_meta.rel_path);
                    for ext in ["svg", "png"] {
                        let candidate = dir.join(format!("{name}.{ext}"));
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }

    for name in names {
        for pix in &index.pixmaps_dirs {
            for ext in ["svg", "png"] {
                let candidate = pix.join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn dir_distance_score(dir: &ThemeDirMeta, target_px: u32) -> u32 {
    if dir.is_scalable && target_px >= dir.min_px && target_px <= dir.max_px {
        return 0;
    }
    if dir.effective_px >= target_px {
        dir.effective_px - target_px
    } else {
        // Penaliza ícones menores que o alvo para evitar serrilhado ao ampliar.
        (target_px - dir.effective_px).saturating_mul(3) + 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gtk_settings_and_kdeglobals_icon_themes() {
        let gtk_ini = "[Settings]\ngtk-theme-name=Mint-Y\ngtk-icon-theme-name = 'Mint-Y-Blue'\n";
        assert_eq!(
            parse_icon_theme_from_gtk_ini(gtk_ini).as_deref(),
            Some("Mint-Y-Blue")
        );

        let kde = "[General]\nName=Breeze\n[Icons]\nTheme=breeze-dark\n";
        assert_eq!(
            parse_icon_theme_from_kdeglobals(kde).as_deref(),
            Some("breeze-dark")
        );
    }

    #[test]
    fn parses_index_theme_inherits_and_filters_symbolic_dirs() {
        let index_raw = r#"
[Icon Theme]
Name=Mint-Y-Blue
Inherits=Mint-Y,Adwaita,hicolor

[places/48]
Size=48
Context=Places
Type=Fixed

[places/symbolic]
Size=16
Context=Places
Type=Scalable
"#;
        assert_eq!(
            parse_theme_inherits(index_raw),
            vec!["Mint-Y", "Adwaita", "hicolor"]
        );
        let dirs = parse_theme_dirs(index_raw);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].rel_path, "places/48");
    }

    #[test]
    fn parses_globs2_and_generic_icons_for_mime_candidates() {
        let globs = "50:application/pdf:*.pdf\n60:text/x-python:*.py\n";
        let generic = "application/pdf:x-office-document\n";
        let mut mime = MimeTable::default();
        parse_globs2_into(globs, &mut mime.ext_to_mime);
        parse_generic_icons_into(generic, &mut mime.mime_to_generic);

        let names = candidate_icon_names(&SystemIconKey::Extension("pdf".into()), &mime);
        assert!(names.contains(&"application-pdf".to_owned()));
        assert!(names.contains(&"x-office-document".to_owned()));
    }
}
