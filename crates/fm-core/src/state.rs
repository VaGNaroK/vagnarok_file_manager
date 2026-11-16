use std::collections::HashSet;
use std::path::PathBuf;

use compact_str::CompactString;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::{
    DirectoryTelemetry, EntryId, FileEntry, FolderSummary, SelectedFileTelemetry, SortSpec,
    ViewMode,
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
                Some(DIR_FILTER_SENTINEL) => entry.kind.is_dir_like(),
                Some(ext) => entry
                    .extension
                    .as_deref()
                    .is_some_and(|e| e.eq_ignore_ascii_case(ext)),
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

fn default_true() -> bool {
    true
}

fn default_grid_card_width() -> f32 {
    140.0
}

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
    pub window_decorations: WindowDecorationsMode,
    #[serde(default)]
    pub default_view_mode: ViewMode,
    #[serde(default = "default_true")]
    pub grid_group_folders: bool,
    #[serde(default = "default_grid_card_width")]
    pub grid_card_width: f32,
    #[serde(default = "default_true")]
    pub dirs_first: bool,
    #[serde(default)]
    pub favorites: Vec<PathBuf>,
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
            window_decorations: WindowDecorationsMode::NativeWm,
            default_view_mode: ViewMode::Grid,
            grid_group_folders: true,
            grid_card_width: default_grid_card_width(),
            dirs_first: true,
            favorites: Vec::new(),
        }
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
}
