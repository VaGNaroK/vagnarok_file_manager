use std::path::PathBuf;

use fm_core::{
    AppConfig, DriveInfo, EntryId, SelectionModifiers, SortField, Strings, TabId, TabState,
    ViewMode,
};
use fm_theme::ThemePalette;
use iced::keyboard;
use iced::keyboard::key::Named;
use iced::widget::{Id, column, container, operation, row, stack};
use iced::{Element, Event, Length, Subscription, Task, event, window};

use crate::sample;
use crate::views;

pub fn search_input_id() -> Id {
    Id::new("fm_search_input")
}

pub fn path_input_id() -> Id {
    Id::new("fm_path_input")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTarget {
    Documents,
    Home,
    Desktop,
    Downloads,
    Pictures,
    CodeDev,
    Favorites,
    Trash,
}

impl SidebarTarget {
    pub fn path(self) -> PathBuf {
        match self {
            Self::Documents => PathBuf::from("/home/aether/Documents/Workspace"),
            Self::Home => PathBuf::from("/home/aether"),
            Self::Desktop => PathBuf::from("/home/aether/Desktop"),
            Self::Downloads => PathBuf::from("/home/aether/Downloads"),
            Self::Pictures => PathBuf::from("/home/aether/Pictures"),
            Self::CodeDev => PathBuf::from("/home/aether/Documents/Workspace/Projects"),
            Self::Favorites => PathBuf::from("/home/aether/Documents/Workspace/Design System"),
            Self::Trash => PathBuf::from("/home/aether/.local/share/Trash"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabNew,
    TabSelect(TabId),
    TabClose(TabId),
    NavigateTo(PathBuf),
    NavigateBack,
    NavigateForward,
    NavigateUp,
    SelectSidebar(SidebarTarget),
    SelectDrive(PathBuf),
    SelectEntry {
        index: usize,
        modifiers: SelectionModifiers,
    },
    ActivateEntry(usize),
    SetViewMode(ViewMode),
    CycleSortField,
    ToggleSortDirection,
    ToggleHidden,
    FilterChanged(String),
    TogglePathEdit,
    PathDraftChanged(String),
    CommitPathDraft,
    ToggleTheme,
    ToggleLanguage,
    ToggleSidebar,
    ToggleInspector,
    ToggleHeroCard,
    ToggleQuickLook,
    CloseQuickLook,
    WindowMinimize,
    WindowToggleMaximize,
    WindowClose,
    ModifiersChanged(keyboard::Modifiers),
    GlobalKeyPressed {
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    },
    Noop,
}

pub struct App {
    pub config: AppConfig,
    pub tabs: Vec<TabState>,
    pub active_tab: TabId,
    pub next_tab_id: u64,
    pub next_request_id: u64,
    pub drives: Vec<DriveInfo>,
    pub active_sidebar: Option<SidebarTarget>,
    pub path_editing: bool,
    pub path_draft: String,
    pub quicklook_open: bool,
    pub current_modifiers: keyboard::Modifiers,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let drives = sample::sample_drives();
        let tab1_path = sample::default_home_path();
        let tab2_path = PathBuf::from("/home/aether/Documents/Workspace/Projects");

        let mut tab1 = build_sample_tab(TabId(1), tab1_path, 1);
        // Destaca `main.rs` inicialmente para espelhar `screen.png` sem contradição de seleção.
        if let Some(main_rs_idx) = tab1
            .visible_entries
            .iter()
            .position(|e| e.name == "main.rs")
        {
            tab1.selection
                .select_single(main_rs_idx, &tab1.visible_entries);
            tab1.selected_telemetry = tab1
                .selection
                .single_selected(&tab1.visible_entries)
                .map(sample::sample_selected_telemetry);
        }

        let tab2 = build_sample_tab(TabId(2), tab2_path, 2);

        let app = Self {
            config: AppConfig::default(),
            tabs: vec![tab1, tab2],
            active_tab: TabId(1),
            next_tab_id: 3,
            next_request_id: 3,
            drives,
            active_sidebar: Some(SidebarTarget::Documents),
            path_editing: false,
            path_draft: String::new(),
            quicklook_open: false,
            current_modifiers: keyboard::Modifiers::empty(),
        };

        (app, Task::none())
    }

    pub fn title(&self) -> String {
        let tab_title = self
            .current_tab()
            .map(|t| t.title.as_str())
            .unwrap_or("Workspace");
        format!("Vagnarok File Manager — {tab_title}")
    }

    pub fn iced_theme(&self) -> iced::Theme {
        self.palette().to_iced_theme()
    }

    pub fn palette(&self) -> ThemePalette {
        ThemePalette::from_preset(self.config.theme)
    }

    pub fn strings(&self) -> Strings {
        Strings::for_language(self.config.language)
    }

    pub fn current_tab(&self) -> Option<&TabState> {
        self.tabs.iter().find(|t| t.id == self.active_tab)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut TabState> {
        let active = self.active_tab;
        self.tabs.iter_mut().find(|t| t.id == active)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabNew => {
                let id = TabId(self.next_tab_id);
                self.next_tab_id += 1;
                let req = self.alloc_request_id();
                let path = self
                    .current_tab()
                    .map(|t| t.current_path.clone())
                    .unwrap_or_else(sample::default_home_path);
                self.tabs.push(build_sample_tab(id, path, req));
                self.active_tab = id;
                self.path_editing = false;
                Task::none()
            }
            Message::TabSelect(id) => {
                if self.tabs.iter().any(|t| t.id == id) {
                    self.active_tab = id;
                    self.path_editing = false;
                    self.quicklook_open = false;
                }
                Task::none()
            }
            Message::TabClose(id) => {
                if self.tabs.len() <= 1 {
                    return Task::none();
                }
                if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                    self.tabs.remove(pos);
                    if self.active_tab == id {
                        let fallback = pos.saturating_sub(1).min(self.tabs.len() - 1);
                        self.active_tab = self.tabs[fallback].id;
                    }
                }
                Task::none()
            }
            Message::NavigateTo(path) => {
                self.navigate_active_tab(path);
                Task::none()
            }
            Message::NavigateBack => {
                let req = self.alloc_request_id();
                if let Some(tab) = self.current_tab_mut()
                    && let Some(prev) = tab.history_back.pop()
                {
                    let cur = std::mem::replace(&mut tab.current_path, prev);
                    tab.history_forward.push(cur);
                    reload_tab_with_sample(tab, req);
                }
                self.path_editing = false;
                Task::none()
            }
            Message::NavigateForward => {
                let req = self.alloc_request_id();
                if let Some(tab) = self.current_tab_mut()
                    && let Some(next) = tab.history_forward.pop()
                {
                    let cur = std::mem::replace(&mut tab.current_path, next);
                    tab.history_back.push(cur);
                    reload_tab_with_sample(tab, req);
                }
                self.path_editing = false;
                Task::none()
            }
            Message::NavigateUp => {
                let parent = self
                    .current_tab()
                    .and_then(|t| t.current_path.parent().map(PathBuf::from));
                if let Some(p) = parent {
                    self.navigate_active_tab(p);
                }
                Task::none()
            }
            Message::SelectSidebar(target) => {
                self.active_sidebar = Some(target);
                self.navigate_active_tab(target.path());
                Task::none()
            }
            Message::SelectDrive(path) => {
                self.active_sidebar = None;
                self.navigate_active_tab(path);
                Task::none()
            }
            Message::SelectEntry { index, modifiers } => {
                if let Some(tab) = self.current_tab_mut() {
                    let was_selected = tab
                        .visible_entries
                        .get(index)
                        .is_some_and(|e| tab.selection.selected_ids.contains(&e.id));
                    // No protótipo P0, um segundo clique simples no único item selecionado pode abrir/focar;
                    // aqui mantemos clique simples = seleção, e botão dedicado/Enter/duplo clique = ativação.
                    if modifiers == SelectionModifiers::default()
                        && was_selected
                        && tab.selection.count() == 1
                        && let Some(entry) = tab.visible_entries.get(index)
                        && entry.kind.is_dir_like()
                    {
                        let next_path = entry.path.clone();
                        self.navigate_active_tab(next_path);
                        return Task::none();
                    }

                    tab.selection
                        .apply_click(index, modifiers, &tab.visible_entries);
                    refresh_selected_telemetry(tab);
                }
                Task::none()
            }
            Message::ActivateEntry(index) => {
                let target = self
                    .current_tab()
                    .and_then(|t| t.visible_entries.get(index).cloned());
                if let Some(entry) = target {
                    if entry.kind.is_dir_like() {
                        self.navigate_active_tab(entry.path);
                    } else {
                        self.quicklook_open = true;
                    }
                }
                Task::none()
            }
            Message::SetViewMode(mode) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.view_mode = mode;
                }
                Task::none()
            }
            Message::CycleSortField => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.sort.field = match tab.sort.field {
                        SortField::Name => SortField::Size,
                        SortField::Size => SortField::Modified,
                        SortField::Modified => SortField::Kind,
                        SortField::Kind => SortField::Name,
                    };
                    tab.recompute_visible();
                }
                Task::none()
            }
            Message::ToggleSortDirection => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.sort.direction = tab.sort.direction.toggled();
                    tab.recompute_visible();
                }
                Task::none()
            }
            Message::ToggleHidden => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.show_hidden = !tab.show_hidden;
                    tab.recompute_visible();
                    refresh_selected_telemetry(tab);
                }
                Task::none()
            }
            Message::FilterChanged(query) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.filter_query = query;
                    tab.recompute_visible();
                    refresh_selected_telemetry(tab);
                }
                Task::none()
            }
            Message::TogglePathEdit => {
                self.path_editing = !self.path_editing;
                if self.path_editing {
                    self.path_draft = self
                        .current_tab()
                        .map(|t| t.current_path.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    return operation::focus(path_input_id());
                }
                Task::none()
            }
            Message::PathDraftChanged(value) => {
                self.path_draft = value;
                Task::none()
            }
            Message::CommitPathDraft => {
                let raw = self.path_draft.trim().to_owned();
                self.path_editing = false;
                if !raw.is_empty() {
                    self.navigate_active_tab(PathBuf::from(raw));
                }
                Task::none()
            }
            Message::ToggleTheme => {
                self.config.theme = self.config.theme.toggled();
                Task::none()
            }
            Message::ToggleLanguage => {
                self.config.language = self.config.language.toggled();
                Task::none()
            }
            Message::ToggleSidebar => {
                self.config.sidebar_collapsed = !self.config.sidebar_collapsed;
                Task::none()
            }
            Message::ToggleInspector => {
                self.config.show_inspector = !self.config.show_inspector;
                Task::none()
            }
            Message::ToggleHeroCard => {
                self.config.show_hero_card = !self.config.show_hero_card;
                Task::none()
            }
            Message::ToggleQuickLook => {
                let has_selection = self
                    .current_tab()
                    .is_some_and(|t| t.selection.count() > 0);
                if self.quicklook_open || has_selection {
                    self.quicklook_open = !self.quicklook_open;
                }
                Task::none()
            }
            Message::CloseQuickLook => {
                self.quicklook_open = false;
                Task::none()
            }
            Message::WindowMinimize => window::latest().and_then(|id| window::minimize(id, true)),
            Message::WindowToggleMaximize => {
                window::latest().and_then(window::toggle_maximize)
            }
            Message::WindowClose => window::latest().and_then(window::close),
            Message::ModifiersChanged(modifiers) => {
                self.current_modifiers = modifiers;
                Task::none()
            }
            Message::GlobalKeyPressed { key, modifiers } => {
                self.handle_global_key(key, modifiers)
            }
            Message::Noop => Task::none(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _window| match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                Some(Message::ModifiersChanged(modifiers))
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let is_shortcut = modifiers.control()
                    || modifiers.alt()
                    || matches!(key, keyboard::Key::Named(Named::Escape));
                if status == event::Status::Ignored || is_shortcut {
                    Some(Message::GlobalKeyPressed { key, modifiers })
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    pub fn view(&self) -> Element<'_, Message> {
        let palette = self.palette();
        let strings = self.strings();
        let tab = self.current_tab();

        let body_row = row![
            views::sidebar::view(self, palette, strings),
            container(iced::widget::Space::new().width(1).height(Length::Fill))
                .style(fm_theme::styles::divider_line(palette)),
            views::canvas::view(self, tab, palette, strings),
            views::inspector::drawer(self, tab, palette, strings),
        ]
        .height(Length::Fill)
        .width(Length::Fill);

        let main_shell = container(
            column![
                views::header::view(self, tab, palette, strings),
                container(iced::widget::Space::new().width(Length::Fill).height(1))
                    .style(fm_theme::styles::divider_line(palette)),
                body_row,
                container(iced::widget::Space::new().width(Length::Fill).height(1))
                    .style(fm_theme::styles::divider_line(palette)),
                views::footer::view(self, tab, palette, strings),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(fm_theme::styles::canvas_container(palette));

        if self.quicklook_open {
            stack![
                main_shell,
                views::inspector::quicklook_modal(tab, palette, strings),
            ]
            .into()
        } else {
            main_shell.into()
        }
    }

    fn alloc_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    fn navigate_active_tab(&mut self, path: PathBuf) {
        let req = self.alloc_request_id();
        if let Some(tab) = self.current_tab_mut() {
            tab.push_navigation(path);
            reload_tab_with_sample(tab, req);
        }
        self.path_editing = false;
        self.quicklook_open = false;
    }

    fn handle_global_key(
        &mut self,
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        match key.as_ref() {
            keyboard::Key::Character("t") if modifiers.control() => self.update(Message::TabNew),
            keyboard::Key::Character("w") if modifiers.control() => {
                let id = self.active_tab;
                self.update(Message::TabClose(id))
            }
            keyboard::Key::Character("b") if modifiers.control() => {
                self.update(Message::ToggleSidebar)
            }
            keyboard::Key::Character("h") if modifiers.control() => {
                self.update(Message::ToggleHidden)
            }
            keyboard::Key::Character("i") if modifiers.alt() => {
                self.update(Message::ToggleInspector)
            }
            keyboard::Key::Character("k") if modifiers.control() => {
                operation::focus(search_input_id())
            }
            keyboard::Key::Character("l") if modifiers.control() => {
                self.update(Message::TogglePathEdit)
            }
            keyboard::Key::Named(Named::Space) if !modifiers.control() && !modifiers.alt() => {
                self.update(Message::ToggleQuickLook)
            }
            keyboard::Key::Named(Named::Escape) => {
                if self.quicklook_open {
                    self.update(Message::CloseQuickLook)
                } else if self.path_editing {
                    self.path_editing = false;
                    Task::none()
                } else {
                    self.update(Message::FilterChanged(String::new()))
                }
            }
            keyboard::Key::Named(Named::ArrowRight | Named::ArrowDown) => {
                self.move_selection(1);
                Task::none()
            }
            keyboard::Key::Named(Named::ArrowLeft | Named::ArrowUp) => {
                self.move_selection(-1);
                Task::none()
            }
            keyboard::Key::Named(Named::Enter) if !self.path_editing => {
                if let Some(focused) = self.current_tab().and_then(|t| t.selection.focused) {
                    return self.update(Message::ActivateEntry(focused));
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if let Some(tab) = self.current_tab_mut() {
            if tab.visible_entries.is_empty() {
                return;
            }
            let len = tab.visible_entries.len() as isize;
            let current = tab.selection.focused.unwrap_or(0) as isize;
            let next = (current + delta).clamp(0, len - 1) as usize;
            tab.selection.select_single(next, &tab.visible_entries);
            refresh_selected_telemetry(tab);
        }
    }
}

fn build_sample_tab(id: TabId, path: PathBuf, request_id: u64) -> TabState {
    let mut tab = TabState::new(id, path);
    reload_tab_with_sample(&mut tab, request_id);
    tab
}

fn reload_tab_with_sample(tab: &mut TabState, request_id: u64) {
    let snapshot = sample::sample_snapshot(&tab.current_path, request_id);
    tab.active_request_id = snapshot.request_id;
    tab.raw_entries = snapshot.entries;
    tab.recompute_visible();
    tab.telemetry = sample::sample_telemetry(&tab.current_path, &tab.visible_entries);
    tab.title = tab
        .current_path
        .file_name()
        .map(|n| compact_str::CompactString::from(n.to_string_lossy().as_ref()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| compact_str::CompactString::from("/"));
}

fn refresh_selected_telemetry(tab: &mut TabState) {
    tab.selected_telemetry = tab
        .selection
        .single_selected(&tab.visible_entries)
        .map(sample::sample_selected_telemetry);
}

pub fn selected_id_set_contains(tab: &TabState, id: EntryId) -> bool {
    tab.selection.selected_ids.contains(&id)
}
