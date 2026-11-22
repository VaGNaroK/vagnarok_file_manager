use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use compact_str::CompactString;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::{
    DirectoryTelemetry, EntryId, FileEntry, FolderSummary, OpenWithAssociation,
    SelectedFileTelemetry, SortSpec, ViewMode,
};

/// Identificador único de uma aba aberta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

/// Modificadores relevantes para regras de seleção de itens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectionModifiers {
    pub ctrl: bool,
    pub shift: bool,
}

/// Fonte única de verdade para seleção de itens dentro de uma aba.
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub selected_ids: HashSet<EntryId>,
    pub anchor: Option<usize>,
    pub focused: Option<usize>,
}

impl SelectionState {
    /// Limpa toda a seleção e foco.
    pub fn clear(&mut self) {
        self.selected_ids.clear();
        self.anchor = None;
        self.focused = None;
    }

    /// Seleciona todas as entradas visíveis (`Ctrl+A`).
    pub fn select_all(&mut self, entries: &[FileEntry]) {
        self.selected_ids.clear();
        for entry in entries {
            self.selected_ids.insert(entry.id);
        }
        self.anchor = if entries.is_empty() { None } else { Some(0) };
        self.focused = entries.len().checked_sub(1);
    }

    /// Seleciona exatamente um índice e atualiza âncora/foco.
    pub fn select_single(&mut self, index: usize, entries: &[FileEntry]) {
        self.selected_ids.clear();
        if let Some(entry) = entries.get(index) {
            self.selected_ids.insert(entry.id);
            self.anchor = Some(index);
            self.focused = Some(index);
        }
    }

    /// Seleciona o intervalo contínuo entre a âncora atual e `target_index` seguindo `visual_order`.
    ///
    /// - Mantém `anchor` fixa (inicializando com `focused` ou `target_index` se ausente) e move `focused`.
    /// - Quando `visual_order` é vazio, usa diretamente a ordem de índices `0..entries.len()`.
    /// - Quando `additive` é `false` (`Shift+Setas`), substitui a seleção pelo intervalo (permitindo
    ///   estender e contrair ao inverter a direção); quando `true` (`Ctrl+Shift+Setas`), acumula.
    pub fn select_range_in_order(
        &mut self,
        target_index: usize,
        visual_order: &[usize],
        additive: bool,
        entries: &[FileEntry],
    ) {
        if entries.get(target_index).is_none() {
            return;
        }
        let anchor_idx = self.anchor.or(self.focused).unwrap_or(target_index);
        self.anchor = Some(anchor_idx);
        self.focused = Some(target_index);

        if !additive {
            self.selected_ids.clear();
        }

        if let (Some(pos_a), Some(pos_b)) = (
            visual_order.iter().position(|&idx| idx == anchor_idx),
            visual_order.iter().position(|&idx| idx == target_index),
        ) {
            let (lo, hi) = if pos_a <= pos_b {
                (pos_a, pos_b)
            } else {
                (pos_b, pos_a)
            };
            for &entry_idx in &visual_order[lo..=hi] {
                if let Some(entry) = entries.get(entry_idx) {
                    self.selected_ids.insert(entry.id);
                }
            }
        } else {
            let lo = anchor_idx.min(target_index);
            let hi = anchor_idx.max(target_index);
            for entry in entries.iter().take(hi + 1).skip(lo) {
                self.selected_ids.insert(entry.id);
            }
        }
    }

    /// Aplica seleção respeitando `Ctrl` (toggle) e `Shift` (intervalo contínuo).
    pub fn apply_click(
        &mut self,
        index: usize,
        modifiers: SelectionModifiers,
        entries: &[FileEntry],
    ) {
        let Some(target) = entries.get(index) else {
            return;
        };

        if modifiers.shift {
            let start = self.anchor.unwrap_or(index).min(index);
            let end = self.anchor.unwrap_or(index).max(index);
            if !modifiers.ctrl {
                self.selected_ids.clear();
            }
            for item in entries.iter().take(end + 1).skip(start) {
                self.selected_ids.insert(item.id);
            }
            self.focused = Some(index);
            if self.anchor.is_none() {
                self.anchor = Some(index);
            }
        } else if modifiers.ctrl {
            if !self.selected_ids.remove(&target.id) {
                self.selected_ids.insert(target.id);
            }
            self.anchor = Some(index);
            self.focused = Some(index);
        } else {
            self.select_single(index, entries);
        }
    }

    /// Retorna quantidade de itens selecionados presentes no slice atual.
    pub fn count(&self) -> usize {
        self.selected_ids.len()
    }

    /// Soma os tamanhos em bytes dos itens selecionados.
    pub fn selected_bytes(&self, entries: &[FileEntry]) -> u64 {
        entries
            .iter()
            .filter(|e| self.selected_ids.contains(&e.id))
            .map(|e| e.size_bytes)
            .fold(0u64, u64::saturating_add)
    }

    /// Retorna o único item selecionado, caso haja exatamente 1 seleção válida.
    pub fn single_selected<'a>(&self, entries: &'a [FileEntry]) -> Option<&'a FileEntry> {
        if self.selected_ids.len() != 1 {
            return None;
        }
        let id = self.selected_ids.iter().next()?;
        entries.iter().find(|e| e.id == *id)
    }
}

/// Estado completo de uma aba de navegação.
#[derive(Debug, Clone)]
pub struct TabState {
    pub id: TabId,
    pub title: CompactString,
    pub current_path: PathBuf,
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,
    pub view_mode: ViewMode,
    pub sort: SortSpec,
    pub show_hidden: bool,
    pub filter_query: String,
    pub extension_filter: Option<CompactString>,
    pub raw_entries: Vec<FileEntry>,
    pub visible_entries: Vec<FileEntry>,
    pub fs_type: Option<CompactString>,
    pub selection: SelectionState,
    pub telemetry: DirectoryTelemetry,
    pub selected_telemetry: Option<SelectedFileTelemetry>,
    pub active_request_id: u64,
    pub is_loading: bool,
    pub last_error: Option<String>,
}

impl TabState {
    /// Cria uma nova aba apontando para `initial_path`.
    pub fn new(id: TabId, initial_path: PathBuf) -> Self {
        let title = initial_path
            .file_name()
            .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| CompactString::from(initial_path.to_string_lossy().as_ref()));

        Self {
            id,
            title,
            current_path: initial_path,
            history_back: Vec::new(),
            history_forward: Vec::new(),
            view_mode: ViewMode::Grid,
            sort: SortSpec::default(),
            show_hidden: false,
            filter_query: String::new(),
            extension_filter: None,
            raw_entries: Vec::new(),
            visible_entries: Vec::new(),
            fs_type: None,
            selection: SelectionState::default(),
            telemetry: DirectoryTelemetry::default(),
            selected_telemetry: None,
            active_request_id: 0,
            is_loading: false,
            last_error: None,
        }
    }

    /// Reaplica filtro, visibilidade de ocultos e ordenação sobre `raw_entries`.
    pub fn recompute_visible(&mut self) {
        let query = self.filter_query.trim().to_ascii_lowercase();
        let ext_filter = self.extension_filter.as_deref();
        let mut filtered: Vec<FileEntry> = self
            .raw_entries
            .iter()
            .filter(|entry| self.show_hidden || !entry.is_hidden)
            .filter(|entry| query.is_empty() || entry.name.to_ascii_lowercase().contains(&query))
            .filter(|entry| match ext_filter {
                None => true,
                Some(filter) => matches_extension_filter(entry, filter),
            })
            .cloned()
            .collect();

        if filtered.len() >= 2_000 {
            filtered.par_sort_by(|a, b| self.sort.compare(a, b));
        } else {
            filtered.sort_by(|a, b| self.sort.compare(a, b));
        }
        self.telemetry.folder_stats = FolderSummary::from_entries(&filtered);
        self.visible_entries = filtered;

        // Remove seleções que deixaram de existir após filtro ou reload.
        let valid_ids: HashSet<EntryId> = self.visible_entries.iter().map(|e| e.id).collect();
        self.selection
            .selected_ids
            .retain(|id| valid_ids.contains(id));
    }

    /// Atualiza `child_count` nas entradas já carregadas quando a telemetria rasa retorna.
    pub fn apply_child_counts(&mut self, counts: &[(PathBuf, u32)]) {
        use std::collections::HashMap;
        let map: HashMap<&PathBuf, u32> = counts.iter().map(|(p, c)| (p, *c)).collect();
        for entry in &mut self.raw_entries {
            if let Some(&count) = map.get(&entry.path) {
                entry.child_count = Some(count);
            }
        }
        for entry in &mut self.visible_entries {
            if let Some(&count) = map.get(&entry.path) {
                entry.child_count = Some(count);
            }
        }
    }

    /// Navega para `next_path`, registrando o atual no histórico de retorno.
    pub fn push_navigation(&mut self, next_path: PathBuf) {
        if self.current_path != next_path {
            let previous = std::mem::replace(&mut self.current_path, next_path);
            self.history_back.push(previous);
            self.history_forward.clear();
        }
        self.title = self
            .current_path
            .file_name()
            .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| CompactString::from(self.current_path.to_string_lossy().as_ref()));
        self.selection.clear();
        self.selected_telemetry = None;
        self.telemetry.recursive_size = None;
        self.filter_query.clear();
        self.extension_filter = None;
        self.last_error = None;
    }

    /// Lista ordenada das extensões presentes em `raw_entries` (respeitando `show_hidden`).
    pub fn available_extensions(&self) -> Vec<CompactString> {
        use std::collections::BTreeSet;
        let mut set = BTreeSet::new();
        for entry in &self.raw_entries {
            if (self.show_hidden || !entry.is_hidden)
                && !entry.kind.is_dir_like()
                && let Some(ext) = &entry.extension
            {
                set.insert(ext.to_ascii_lowercase());
            }
        }
        set.into_iter().collect()
    }

    pub fn can_go_back(&self) -> bool {
        !self.history_back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.history_forward.is_empty()
    }
}

/// Presets oficiais de tema conforme Spec 02.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreset {
    #[default]
    AetherLight,
    ObsidianFlow,
}

impl ThemePreset {
    pub fn toggled(self) -> Self {
        match self {
            Self::AetherLight => Self::ObsidianFlow,
            Self::ObsidianFlow => Self::AetherLight,
        }
    }
}

/// Idiomas suportados pela camada de UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    PtBr,
    EnUs,
    EsEs,
    FrFr,
    DeDe,
    ItIt,
}

impl Language {
    pub const ALL: [Self; 6] = [
        Self::PtBr,
        Self::EnUs,
        Self::EsEs,
        Self::FrFr,
        Self::DeDe,
        Self::ItIt,
    ];

    pub const fn bcp47(self) -> &'static str {
        match self {
            Self::PtBr => "pt-BR",
            Self::EnUs => "en-US",
            Self::EsEs => "es-ES",
            Self::FrFr => "fr-FR",
            Self::DeDe => "de-DE",
            Self::ItIt => "it-IT",
        }
    }

    pub const fn short_code(self) -> &'static str {
        match self {
            Self::PtBr => "PT",
            Self::EnUs => "EN",
            Self::EsEs => "ES",
            Self::FrFr => "FR",
            Self::DeDe => "DE",
            Self::ItIt => "IT",
        }
    }

    pub const fn native_name(self) -> &'static str {
        match self {
            Self::PtBr => "Português (BR)",
            Self::EnUs => "English (US)",
            Self::EsEs => "Español",
            Self::FrFr => "Français",
            Self::DeDe => "Deutsch",
            Self::ItIt => "Italiano",
        }
    }

    /// Resolve um tag de locale (BCP-47 ou POSIX, ex.: `pt_BR.UTF-8`, `es-419`, `de`).
    pub fn from_locale_tag(tag: &str) -> Option<Self> {
        let base = tag.split('.').next().unwrap_or(tag).trim();
        let norm = base.replace('_', "-").to_ascii_lowercase();
        let primary = norm.split('-').next().unwrap_or("");
        match primary {
            "pt" => Some(Self::PtBr),
            "en" => Some(Self::EnUs),
            "es" => Some(Self::EsEs),
            "fr" => Some(Self::FrFr),
            "de" => Some(Self::DeDe),
            "it" => Some(Self::ItIt),
            _ => None,
        }
    }

    /// Detecta o idioma do sistema operacional com fallback para inglês (`EnUs`).
    pub fn detect_system() -> Self {
        sys_locale::get_locale()
            .as_deref()
            .and_then(Self::from_locale_tag)
            .unwrap_or(Self::EnUs)
    }

    pub fn toggled(self) -> Self {
        let idx = Self::ALL.iter().position(|&lang| lang == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

/// Modo de decoração de janela (especialmente relevante para tiling WMs no Linux).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WindowDecorationsMode {
    #[default]
    NativeWm,
    IntegratedCsd,
}

pub const DIR_FILTER_SENTINEL: &str = "__DIR__";

/// Normaliza a entrada digitada pelo usuário no filtro de extensões (ex.: `".exe, .zip"` -> `Some("exe, zip")`).
/// Aceita separadores por vírgula, ponto-e-vírgula ou espaço, com ou sem `.` / `*` inicial.
#[must_use]
pub fn normalize_extension_filter_input(raw: &str) -> Option<CompactString> {
    let mut tokens: Vec<String> = Vec::new();
    for part in raw.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let cleaned = part.trim().trim_start_matches('*').trim_start_matches('.');
        if cleaned.is_empty() {
            continue;
        }
        let lower = cleaned.to_ascii_lowercase();
        if !tokens.contains(&lower) {
            tokens.push(lower);
        }
    }
    if tokens.is_empty() {
        None
    } else {
        Some(CompactString::from(tokens.join(", ")))
    }
}

/// Formata o filtro normalizado de extensões para exibição na UI (`"exe, zip"` -> `".exe, .zip"`).
#[must_use]
pub fn format_extension_filter_badge(filter: &str) -> String {
    let parts: Vec<String> = filter
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .map(|p| p.trim().trim_start_matches('*').trim_start_matches('.'))
        .filter(|p| !p.is_empty())
        .map(|p| format!(".{}", p.to_ascii_lowercase()))
        .collect();
    parts.join(", ")
}

/// Avalia se `entry` satisfaz o filtro de extensão ativo (sentinela de pastas ou uma ou mais extensões).
#[must_use]
pub fn matches_extension_filter(entry: &FileEntry, filter: &str) -> bool {
    if filter == DIR_FILTER_SENTINEL {
        return entry.kind.is_dir_like();
    }
    let mut has_token = false;
    let name_lower = if entry.kind.is_dir_like() {
        None
    } else {
        Some(entry.name.to_ascii_lowercase())
    };
    for part in filter.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let cleaned = part.trim().trim_start_matches('*').trim_start_matches('.');
        if cleaned.is_empty() {
            continue;
        }
        has_token = true;
        if entry.kind.is_dir_like() {
            continue;
        }
        if cleaned.contains('.') {
            let suffix = format!(".{}", cleaned.to_ascii_lowercase());
            if let Some(ref nl) = name_lower
                && nl.len() > suffix.len()
                && nl.ends_with(&suffix)
            {
                return true;
            }
        } else if entry
            .extension
            .as_deref()
            .is_some_and(|e| e.eq_ignore_ascii_case(cleaned))
        {
            return true;
        }
    }
    !has_token
}

fn default_true() -> bool {
    true
}

fn default_grid_card_width() -> f32 {
    140.0
}

/// Pasta aberta por padrão ao iniciar o aplicativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StartupLocation {
    #[default]
    Home,
    LastVisited,
}

impl StartupLocation {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Home => Self::LastVisited,
            Self::LastVisited => Self::Home,
        }
    }
}

/// Modo de ícones exibidos para pastas, arquivos, locais e unidades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IconThemeMode {
    #[default]
    Bundled,
    HostSystem,
}

/// Capacidade máxima do histórico MRU de arquivos recentes.
pub const MAX_RECENT_FILES: usize = 100;

/// Preferências persistíveis da aplicação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: ThemePreset,
    #[serde(default)]
    pub theme_customized: bool,
    pub language: Language,
    #[serde(default)]
    pub language_customized: bool,
    pub show_hero_card: bool,
    pub show_inspector: bool,
    pub sidebar_collapsed: bool,
    #[serde(default)]
    pub sidebar_width: Option<f32>,
    pub window_decorations: WindowDecorationsMode,
    #[serde(default)]
    pub default_view_mode: ViewMode,
    #[serde(default = "default_true")]
    pub grid_group_folders: bool,
    #[serde(default)]
    pub grid_center_items: bool,
    #[serde(default = "default_grid_card_width")]
    pub grid_card_width: f32,
    #[serde(default = "default_true")]
    pub dirs_first: bool,
    #[serde(default)]
    pub icon_theme_mode: IconThemeMode,
    #[serde(default)]
    pub favorites: Vec<PathBuf>,
    #[serde(default)]
    pub open_with_defaults: BTreeMap<String, OpenWithAssociation>,
    #[serde(default)]
    pub open_with_custom_apps: BTreeMap<String, Vec<OpenWithAssociation>>,
    #[serde(default = "default_true")]
    pub check_updates_on_startup: bool,
    #[serde(default)]
    pub dismissed_update_tag: Option<String>,
    #[serde(default)]
    pub last_update_check_unix: Option<u64>,
    #[serde(default = "default_true")]
    pub show_recent_files: bool,
    #[serde(default)]
    pub recent_files: Vec<PathBuf>,
    #[serde(default)]
    pub startup_location: StartupLocation,
    #[serde(default)]
    pub last_visited_dir: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemePreset::AetherLight,
            theme_customized: false,
            language: Language::detect_system(),
            language_customized: false,
            show_hero_card: true,
            show_inspector: false,
            sidebar_collapsed: false,
            sidebar_width: None,
            window_decorations: WindowDecorationsMode::NativeWm,
            default_view_mode: ViewMode::Grid,
            grid_group_folders: true,
            grid_center_items: false,
            grid_card_width: default_grid_card_width(),
            dirs_first: true,
            icon_theme_mode: IconThemeMode::Bundled,
            favorites: Vec::new(),
            open_with_defaults: BTreeMap::new(),
            open_with_custom_apps: BTreeMap::new(),
            check_updates_on_startup: true,
            dismissed_update_tag: None,
            last_update_check_unix: None,
            show_recent_files: true,
            recent_files: Vec::new(),
            startup_location: StartupLocation::Home,
            last_visited_dir: None,
        }
    }
}

impl AppConfig {
    /// Registra um arquivo aberto no topo da lista MRU quando `show_recent_files` está ativo.
    pub fn record_recent_file(&mut self, path: PathBuf) -> bool {
        if !self.show_recent_files
            || path.as_os_str().is_empty()
            || crate::domain::is_virtual_readonly_dir(&path)
        {
            return false;
        }
        if self.recent_files.first().is_some_and(|p| p == &path) {
            return false;
        }
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        if self.recent_files.len() > MAX_RECENT_FILES {
            self.recent_files.truncate(MAX_RECENT_FILES);
        }
        true
    }

    /// Remove um caminho específico do histórico de arquivos recentes.
    pub fn remove_recent_file(&mut self, path: &std::path::Path) -> bool {
        let prev_len = self.recent_files.len();
        self.recent_files.retain(|p| p != path);
        self.recent_files.len() != prev_len
    }

    /// Limpa todo o histórico de arquivos recentes.
    pub fn clear_recent_files(&mut self) -> bool {
        if self.recent_files.is_empty() {
            return false;
        }
        self.recent_files.clear();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EntryKind, PermissionsMeta};

    fn make_entries(count: usize) -> Vec<FileEntry> {
        (0..count)
            .map(|i| FileEntry {
                id: EntryId(i as u64 + 1),
                name: CompactString::from(format!("item_{i}.txt")),
                path: PathBuf::from(format!("/tmp/item_{i}.txt")),
                extension: Some(CompactString::new("txt")),
                kind: EntryKind::File,
                size_bytes: 100 * (i as u64 + 1),
                created_at: None,
                modified_at: None,
                is_hidden: false,
                permissions: PermissionsMeta::PosixMode(0o644),
                child_count: None,
                mime_badge: None,
            })
            .collect()
    }

    #[test]
    fn range_and_ctrl_selection_work_consistently() {
        let entries = make_entries(5);
        let mut sel = SelectionState::default();

        sel.apply_click(1, SelectionModifiers::default(), &entries);
        assert_eq!(sel.count(), 1);
        assert_eq!(sel.selected_bytes(&entries), 200);

        sel.apply_click(
            3,
            SelectionModifiers {
                ctrl: false,
                shift: true,
            },
            &entries,
        );
        assert_eq!(sel.count(), 3);
        assert_eq!(sel.selected_bytes(&entries), 200 + 300 + 400);

        sel.apply_click(
            2,
            SelectionModifiers {
                ctrl: true,
                shift: false,
            },
            &entries,
        );
        assert_eq!(sel.count(), 2);
        assert_eq!(sel.selected_bytes(&entries), 200 + 400);
    }

    #[test]
    fn hidden_filter_updates_visible_entries_and_clears_hidden_selection() {
        let mut tab = TabState::new(TabId(1), PathBuf::from("/tmp"));
        let mut entries = make_entries(2);
        entries[1].is_hidden = true;
        tab.raw_entries = entries;
        tab.show_hidden = true;
        tab.recompute_visible();
        tab.selection.select_single(1, &tab.visible_entries);
        assert_eq!(tab.selection.count(), 1);

        tab.show_hidden = false;
        tab.recompute_visible();
        assert_eq!(tab.visible_entries.len(), 1);
        assert_eq!(tab.selection.count(), 0);
    }

    #[test]
    fn select_all_selects_every_visible_entry_and_handles_empty() {
        let mut sel = SelectionState::default();
        let entries = make_entries(4);
        sel.select_all(&entries);
        assert_eq!(sel.count(), 4);
        assert_eq!(sel.anchor, Some(0));
        assert_eq!(sel.focused, Some(3));
        assert_eq!(sel.selected_bytes(&entries), 100 + 200 + 300 + 400);

        sel.select_all(&[]);
        assert_eq!(sel.count(), 0);
        assert!(sel.anchor.is_none());
        assert!(sel.focused.is_none());
    }

    #[test]
    fn extension_filter_matches_single_multiple_and_compound_extensions() {
        let mut entries = make_entries(4);
        entries[0].name = CompactString::new("setup.EXE");
        entries[0].extension = Some(CompactString::new("EXE"));
        entries[1].name = CompactString::new("backup.tar.gz");
        entries[1].extension = Some(CompactString::new("gz"));
        entries[2].name = CompactString::new("archive.zip");
        entries[2].extension = Some(CompactString::new("zip"));
        entries[3].name = CompactString::new("Folder");
        entries[3].extension = None;
        entries[3].kind = EntryKind::Directory;

        let norm = normalize_extension_filter_input(" .EXE, *.zip ; .exe ");
        assert_eq!(norm.as_deref(), Some("exe, zip"));
        assert_eq!(format_extension_filter_badge("exe, zip"), ".exe, .zip");

        assert!(matches_extension_filter(&entries[0], "exe, zip"));
        assert!(!matches_extension_filter(&entries[1], "exe, zip"));
        assert!(matches_extension_filter(&entries[2], "exe, zip"));
        assert!(!matches_extension_filter(&entries[3], "exe, zip"));

        assert!(matches_extension_filter(&entries[1], ".tar.gz"));
        assert!(!matches_extension_filter(&entries[2], ".tar.gz"));

        assert!(matches_extension_filter(&entries[3], DIR_FILTER_SENTINEL));
        assert!(!matches_extension_filter(&entries[0], DIR_FILTER_SENTINEL));

        assert_eq!(normalize_extension_filter_input("   ... , "), None);
    }

    #[test]
    fn shift_arrow_range_extends_contracts_and_respects_visual_order() {
        let entries = make_entries(6);
        let mut sel = SelectionState::default();

        // Começa no item 2 e estende para 4 (Shift+Down/Right).
        sel.select_single(2, &entries);
        sel.select_range_in_order(3, &[], false, &entries);
        sel.select_range_in_order(4, &[], false, &entries);
        assert_eq!(sel.anchor, Some(2));
        assert_eq!(sel.focused, Some(4));
        assert_eq!(sel.count(), 3);
        assert!(sel.selected_ids.contains(&entries[2].id));
        assert!(sel.selected_ids.contains(&entries[3].id));
        assert!(sel.selected_ids.contains(&entries[4].id));

        // Recua para 3: deve contrair removendo o 4.
        sel.select_range_in_order(3, &[], false, &entries);
        assert_eq!(sel.count(), 2);
        assert!(!sel.selected_ids.contains(&entries[4].id));

        // Cruza a âncora para o lado oposto (índice 1): seleciona [1, 2].
        sel.select_range_in_order(1, &[], false, &entries);
        assert_eq!(sel.anchor, Some(2));
        assert_eq!(sel.focused, Some(1));
        assert_eq!(sel.count(), 2);
        assert!(sel.selected_ids.contains(&entries[1].id));
        assert!(sel.selected_ids.contains(&entries[2].id));
        assert!(!sel.selected_ids.contains(&entries[3].id));

        // Ordem visual agrupada (ex.: Pastas [4, 1], depois Arquivos [0, 2, 3, 5]).
        let visual_order = [4, 1, 0, 2, 3, 5];
        sel.select_single(4, &entries);
        sel.select_range_in_order(0, &visual_order, false, &entries);
        assert_eq!(sel.count(), 3);
        assert!(sel.selected_ids.contains(&entries[4].id));
        assert!(sel.selected_ids.contains(&entries[1].id));
        assert!(sel.selected_ids.contains(&entries[0].id));
        assert!(!sel.selected_ids.contains(&entries[2].id));

        // Ctrl+Shift (additive) mantém os itens anteriores ao estender.
        sel.select_range_in_order(2, &visual_order, true, &entries);
        assert_eq!(sel.count(), 4);
        assert!(sel.selected_ids.contains(&entries[2].id));
    }

    #[test]
    fn recent_files_mru_deduplicates_caps_and_respects_enabled_flag() {
        let mut cfg = AppConfig::default();
        assert!(cfg.show_recent_files);
        assert!(cfg.recent_files.is_empty());

        let a = PathBuf::from("/tmp/a.txt");
        let b = PathBuf::from("/tmp/b.txt");
        assert!(cfg.record_recent_file(a.clone()));
        assert!(cfg.record_recent_file(b.clone()));
        assert_eq!(cfg.recent_files, vec![b.clone(), a.clone()]);

        // Reabrir `a` move para o topo sem duplicar.
        assert!(cfg.record_recent_file(a.clone()));
        assert_eq!(cfg.recent_files, vec![a.clone(), b.clone()]);

        // Caminhos virtuais ou vazios são ignorados.
        assert!(!cfg.record_recent_file(PathBuf::from("trash://")));
        assert!(!cfg.record_recent_file(PathBuf::from("recent://")));

        // Remover individualmente.
        assert!(cfg.remove_recent_file(&a));
        assert_eq!(cfg.recent_files, vec![b]);

        // Desativado: não registra novos arquivos.
        cfg.show_recent_files = false;
        assert!(!cfg.record_recent_file(a));
        assert_eq!(cfg.recent_files.len(), 1);

        cfg.clear_recent_files();
        assert!(cfg.recent_files.is_empty());
    }
}
