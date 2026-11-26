use fm_core::{Strings, ViewMode};

use crate::app::{App, Message, SidebarTarget};
use crate::archive_jobs::ArchiveUiMsg;
use crate::disk_analyzer::DiskAnalyzerMsg;
use crate::duplicate_finder::DuplicateFinderMsg;

/// Entrada individual exibida na Command Palette (`Ctrl+Shift+P`).
#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub keywords: &'static str,
    pub shortcut: &'static str,
    pub enabled: bool,
    pub message: Message,
}

/// Constrói o catálogo completo de ações do aplicativo com base no estado atual.
pub fn build_catalog(app: &App, strings: Strings) -> Vec<CommandEntry> {
    let tab = app.current_tab();
    let has_selection = tab.is_some_and(|t| t.selection.count() > 0);
    let active_path = tab
        .map(|t| t.current_path.clone())
        .unwrap_or_default();
    let selected_or_current_path = tab
        .and_then(|t| {
            t.visible_entries
                .iter()
                .find(|e| t.selection.selected_ids.contains(&e.id))
                .map(|e| e.path.clone())
        })
        .unwrap_or_else(|| active_path.clone());
    let can_close_tab = app.tabs.len() > 1;
    let can_paste = app.clipboard.is_some();
    let can_undo = !app.undo_stack.is_empty();
    let has_filter = tab.is_some_and(|t| {
        !t.filter_query.trim().is_empty() || t.extension_filter.is_some()
    });

    vec![
        // Navegação & Abas
        CommandEntry {
            id: "nav.new_tab",
            category: strings.cmd_cat_nav,
            title: strings.tip_new_tab,
            keywords: "tab aba nova",
            shortcut: "Ctrl+T",
            enabled: true,
            message: Message::TabNew,
        },
        CommandEntry {
            id: "nav.close_tab",
            category: strings.cmd_cat_nav,
            title: strings.tip_close_tab,
            keywords: "tab aba fechar close",
            shortcut: "Ctrl+W",
            enabled: can_close_tab,
            message: Message::TabClose(app.active_tab),
        },
        CommandEntry {
            id: "nav.next_tab",
            category: strings.cmd_cat_nav,
            title: "Next tab / Próxima aba",
            keywords: "tab aba proxima next",
            shortcut: "Ctrl+Tab",
            enabled: can_close_tab,
            message: Message::TabNext(false),
        },
        CommandEntry {
            id: "nav.split_view",
            category: strings.cmd_cat_nav,
            title: strings.tip_split_view,
            keywords: "split dual pane painel f3",
            shortcut: "F3",
            enabled: true,
            message: Message::ToggleSplitView,
        },
        CommandEntry {
            id: "nav.back",
            category: strings.cmd_cat_nav,
            title: strings.tip_back,
            keywords: "back voltar",
            shortcut: "Alt+←",
            enabled: true,
            message: Message::NavigateBack,
        },
        CommandEntry {
            id: "nav.forward",
            category: strings.cmd_cat_nav,
            title: strings.tip_forward,
            keywords: "forward avancar",
            shortcut: "Alt+→",
            enabled: true,
            message: Message::NavigateForward,
        },
        CommandEntry {
            id: "nav.up",
            category: strings.cmd_cat_nav,
            title: strings.tip_up,
            keywords: "up subir pai parent",
            shortcut: "Alt+↑",
            enabled: true,
            message: Message::NavigateUp,
        },
        CommandEntry {
            id: "nav.home",
            category: strings.cmd_cat_nav,
            title: strings.home,
            keywords: "home inicio",
            shortcut: "",
            enabled: true,
            message: Message::SelectSidebar(SidebarTarget::Home),
        },
        CommandEntry {
            id: "nav.recent",
            category: strings.cmd_cat_nav,
            title: strings.recent_files,
            keywords: "recent recentes",
            shortcut: "",
            enabled: true,
            message: Message::SelectSidebar(SidebarTarget::Recent),
        },
        CommandEntry {
            id: "nav.trash",
            category: strings.cmd_cat_nav,
            title: strings.trash,
            keywords: "trash lixeira",
            shortcut: "",
            enabled: true,
            message: Message::SelectSidebar(SidebarTarget::Trash),
        },
        CommandEntry {
            id: "nav.edit_path",
            category: strings.cmd_cat_nav,
            title: strings.tip_edit_path,
            keywords: "path caminho barra endereco",
            shortcut: "Ctrl+L",
            enabled: true,
            message: Message::TogglePathEdit,
        },
        CommandEntry {
            id: "nav.quick_jump",
            category: strings.cmd_cat_nav,
            title: strings.quick_jump_title,
            keywords: "quick jump ir pasta folder goto",
            shortcut: "Ctrl+P",
            enabled: true,
            message: Message::OpenQuickJump,
        },
        CommandEntry {
            id: "nav.refresh",
            category: strings.cmd_cat_nav,
            title: strings.refresh_drives,
            keywords: "refresh atualizar recarregar",
            shortcut: "F5",
            enabled: true,
            message: Message::RefreshActiveTab,
        },
        // Busca & Filtros
        CommandEntry {
            id: "search.deep",
            category: strings.cmd_cat_search,
            title: strings.deep_search_title,
            keywords: "busca avancada profunda regex tamanho data",
            shortcut: "Ctrl+Shift+F",
            enabled: true,
            message: Message::OpenDeepSearch,
        },
        CommandEntry {
            id: "search.containing",
            category: strings.cmd_cat_search,
            title: strings.cmd_search_containing,
            keywords: "contendo content grep texto dentro",
            shortcut: "",
            enabled: true,
            message: Message::OpenDeepSearchIn(active_path.clone()),
        },
        CommandEntry {
            id: "search.quick_filter",
            category: strings.cmd_cat_search,
            title: strings.cmd_focus_quick_filter,
            keywords: "filtro filter quick ext",
            shortcut: "Ctrl+K",
            enabled: true,
            message: Message::FocusQuickFilter,
        },
        CommandEntry {
            id: "search.clear_filters",
            category: strings.cmd_cat_search,
            title: strings.cmd_clear_filters,
            keywords: "limpar clear filter",
            shortcut: "",
            enabled: has_filter,
            message: Message::ClearTabFilters,
        },
        // Arquivos & Operações
        CommandEntry {
            id: "files.new_folder",
            category: strings.cmd_cat_files,
            title: strings.new_folder,
            keywords: "nova pasta mkdir folder",
            shortcut: "Ctrl+Shift+N",
            enabled: true,
            message: Message::OpenCreateDialog(true),
        },
        CommandEntry {
            id: "files.new_file",
            category: strings.cmd_cat_files,
            title: strings.new_file,
            keywords: "novo arquivo touch file",
            shortcut: "",
            enabled: true,
            message: Message::OpenCreateDialog(false),
        },
        CommandEntry {
            id: "files.rename",
            category: strings.cmd_cat_files,
            title: strings.rename,
            keywords: "renomear rename f2",
            shortcut: "F2",
            enabled: has_selection,
            message: Message::OpenRenameDialog,
        },
        CommandEntry {
            id: "files.bulk_rename",
            category: strings.cmd_cat_files,
            title: strings.br_title,
            keywords: "renomeacao lote batch bulk regex exif id3",
            shortcut: "",
            enabled: has_selection,
            message: Message::OpenBulkRenameDialog,
        },
        CommandEntry {
            id: "files.copy",
            category: strings.cmd_cat_files,
            title: strings.copy,
            keywords: "copiar copy",
            shortcut: "Ctrl+C",
            enabled: has_selection,
            message: Message::CopySelected,
        },
        CommandEntry {
            id: "files.cut",
            category: strings.cmd_cat_files,
            title: strings.cut,
            keywords: "recortar mover cut",
            shortcut: "Ctrl+X",
            enabled: has_selection,
            message: Message::CutSelected,
        },
        CommandEntry {
            id: "files.paste",
            category: strings.cmd_cat_files,
            title: strings.paste,
            keywords: "colar paste",
            shortcut: "Ctrl+V",
            enabled: can_paste,
            message: Message::PasteClipboard,
        },
        CommandEntry {
            id: "files.undo",
            category: strings.cmd_cat_files,
            title: strings.undo_action,
            keywords: "desfazer undo",
            shortcut: "Ctrl+Z",
            enabled: can_undo,
            message: Message::UndoLastAction,
        },
        CommandEntry {
            id: "files.select_all",
            category: strings.cmd_cat_files,
            title: strings.select_all,
            keywords: "selecionar tudo all",
            shortcut: "Ctrl+A",
            enabled: true,
            message: Message::SelectAll,
        },
        CommandEntry {
            id: "files.trash",
            category: strings.cmd_cat_files,
            title: strings.move_to_trash,
            keywords: "lixeira trash apagar",
            shortcut: "Del",
            enabled: has_selection,
            message: Message::RequestDelete(false),
        },
        CommandEntry {
            id: "files.delete",
            category: strings.cmd_cat_files,
            title: strings.delete_permanent,
            keywords: "excluir permanente delete",
            shortcut: "Shift+Del",
            enabled: has_selection,
            message: Message::RequestDelete(true),
        },
        CommandEntry {
            id: "files.compress",
            category: strings.cmd_cat_files,
            title: strings.arch_compress_as,
            keywords: "compactar zip 7z tar archive",
            shortcut: "",
            enabled: has_selection,
            message: Message::Archive(ArchiveUiMsg::OpenCompressDialog),
        },
        CommandEntry {
            id: "files.terminal",
            category: strings.cmd_cat_files,
            title: strings.tip_terminal,
            keywords: "terminal shell console cmd",
            shortcut: "Ctrl+Shift+T",
            enabled: true,
            message: Message::OpenTerminal,
        },
        CommandEntry {
            id: "files.properties",
            category: strings.cmd_cat_files,
            title: strings.properties,
            keywords: "propriedades permissoes chmod acl",
            shortcut: "",
            enabled: true,
            message: Message::OpenProperties(selected_or_current_path),
        },
        // Visualização
        CommandEntry {
            id: "view.grid",
            category: strings.cmd_cat_view,
            title: strings.cmd_view_grid,
            keywords: "grade grid icones",
            shortcut: "",
            enabled: true,
            message: Message::SetViewMode(ViewMode::Grid),
        },
        CommandEntry {
            id: "view.list",
            category: strings.cmd_cat_view,
            title: strings.cmd_view_list,
            keywords: "lista tabela detalhes list",
            shortcut: "",
            enabled: true,
            message: Message::SetViewMode(ViewMode::DetailTable),
        },
        CommandEntry {
            id: "view.hidden",
            category: strings.cmd_cat_view,
            title: strings.tip_hidden,
            keywords: "ocultos hidden dotfiles",
            shortcut: "Ctrl+H",
            enabled: true,
            message: Message::ToggleHidden,
        },
        CommandEntry {
            id: "view.sidebar",
            category: strings.cmd_cat_view,
            title: strings.tip_toggle_sidebar,
            keywords: "sidebar barra lateral",
            shortcut: "Ctrl+B",
            enabled: true,
            message: Message::ToggleSidebar,
        },
        CommandEntry {
            id: "view.inspector",
            category: strings.cmd_cat_view,
            title: strings.tip_inspector,
            keywords: "inspector inspetor painel",
            shortcut: "Alt+I",
            enabled: true,
            message: Message::ToggleInspector,
        },
        CommandEntry {
            id: "view.theme",
            category: strings.cmd_cat_view,
            title: strings.cmd_toggle_theme,
            keywords: "tema theme dark light obsidian aether",
            shortcut: "",
            enabled: true,
            message: Message::ToggleTheme,
        },
        // Ferramentas
        CommandEntry {
            id: "tools.disk_analyzer",
            category: strings.cmd_cat_tools,
            title: strings.analyze_space,
            keywords: "disco espaco disk analyzer treemap grandes",
            shortcut: "",
            enabled: true,
            message: Message::DiskAnalyzer(DiskAnalyzerMsg::OpenForPaths(vec![active_path])),
        },
        CommandEntry {
            id: "tools.duplicates",
            category: strings.cmd_cat_tools,
            title: strings.dup_modal_title,
            keywords: "duplicados duplicates md5 hash",
            shortcut: "",
            enabled: true,
            message: Message::DuplicateFinder(DuplicateFinderMsg::OpenForCurrentDir),
        },
        CommandEntry {
            id: "tools.quicklook",
            category: strings.cmd_cat_tools,
            title: strings.quicklook_title,
            keywords: "quicklook preview visualizar espaco",
            shortcut: "Space",
            enabled: has_selection,
            message: Message::ToggleQuickLook,
        },
        CommandEntry {
            id: "tools.preferences",
            category: strings.cmd_cat_tools,
            title: strings.tip_preferences,
            keywords: "preferencias configuracoes settings",
            shortcut: "",
            enabled: true,
            message: Message::OpenPreferences,
        },
        CommandEntry {
            id: "tools.updates",
            category: strings.cmd_cat_tools,
            title: strings.cmd_check_updates,
            keywords: "atualizacoes updates versao",
            shortcut: "",
            enabled: true,
            message: Message::CheckForUpdates { manual: true },
        },
    ]
}

/// Filtra e ordena comandos por query fuzzy (ou prioriza recentes quando a query está vazia).
pub fn filter_commands(
    mut entries: Vec<CommandEntry>,
    query: &str,
    recent_ids: &[&'static str],
) -> Vec<CommandEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        entries.sort_by_key(|entry| {
            let recent_pos = recent_ids
                .iter()
                .position(|&id| id == entry.id)
                .unwrap_or(usize::MAX);
            (!entry.enabled, recent_pos)
        });
        return entries;
    }

    let search_strings: Vec<String> = entries
        .iter()
        .map(|e| format!("{} {} {} {}", e.title, e.category, e.keywords, e.shortcut))
        .collect();
    let refs: Vec<&str> = search_strings.iter().map(String::as_str).collect();
    let ranked_indices = fm_index::fuzzy_rank_indices(&refs, trimmed);

    let mut enabled_hits = Vec::new();
    let mut disabled_hits = Vec::new();
    for idx in ranked_indices {
        let entry = entries[idx].clone();
        if entry.enabled {
            enabled_hits.push(entry);
        } else {
            disabled_hits.push(entry);
        }
    }
    enabled_hits.extend(disabled_hits);
    enabled_hits
}

/// Move o índice selecionado com clamping seguro dentro da lista visível.
pub fn move_selection(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let max_idx = len - 1;
    let clamped_current = current.min(max_idx) as isize;
    (clamped_current + delta).clamp(0, max_idx as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &'static str, title: &'static str, enabled: bool) -> CommandEntry {
        CommandEntry {
            id,
            category: "Tools",
            title,
            keywords: "test",
            shortcut: "",
            enabled,
            message: Message::OpenPreferences,
        }
    }

    #[test]
    fn filter_commands_prioritizes_recent_and_fuzzy_matches() {
        let entries = vec![
            sample_entry("a", "Preferences", true),
            sample_entry("b", "Disk Space Analyzer", true),
            sample_entry("c", "Delete Permanently", false),
        ];

        let empty = filter_commands(entries.clone(), "", &["b"]);
        assert_eq!(empty[0].id, "b");
        assert_eq!(empty[2].id, "c");

        let fuzzy = filter_commands(entries, "disk", &[]);
        assert_eq!(fuzzy.len(), 1);
        assert_eq!(fuzzy[0].id, "b");
    }

    #[test]
    fn move_selection_clamps_within_bounds() {
        assert_eq!(move_selection(0, -1, 5), 0);
        assert_eq!(move_selection(0, 2, 5), 2);
        assert_eq!(move_selection(4, 3, 5), 4);
        assert_eq!(move_selection(3, 1, 0), 0);
    }
}
