use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use compact_str::CompactString;
use fm_core::{
    AppConfig, DirectorySnapshot, DriveInfo, EntryId, FsError, SelectedFileTelemetry,
    SelectionModifiers, SortField, Strings, TabId, TabState, UserPlaces, VfsProvider, ViewMode,
    VolumeSpace,
};
use fm_fs::LocalFs;
use fm_theme::ThemePalette;
use iced::keyboard;
use iced::keyboard::key::Named;
use iced::widget::operation::RelativeOffset;
use iced::widget::{Id, column, container, operation, row, stack};
use iced::{Element, Event, Length, Subscription, Task, event, window};
use tokio_util::sync::CancellationToken;

use crate::views;

pub fn search_input_id() -> Id {
    Id::new("fm_search_input")
}

pub fn path_input_id() -> Id {
    Id::new("fm_path_input")
}

pub fn canvas_scroll_id() -> Id {
    Id::new("fm_canvas_scroll")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTarget {
    Documents,
    Home,
    Desktop,
    Downloads,
    Pictures,
    CodeDev,
    Trash,
}

impl SidebarTarget {
    pub fn resolve_path(self, places: &UserPlaces) -> Option<PathBuf> {
        match self {
            Self::Documents => places.documents.clone(),
            Self::Home => Some(places.home.clone()),
            Self::Desktop => places.desktop.clone(),
            Self::Downloads => places.downloads.clone(),
            Self::Pictures => places.pictures.clone(),
            Self::CodeDev => std::env::current_dir().ok(),
            Self::Trash => places.trash.clone(),
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
    RefreshActiveTab,
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
    CanvasScrolled {
        offset_y: f32,
        viewport_height: f32,
    },
    DrivesLoaded(Result<Vec<DriveInfo>, FsError>),
    DirectoryLoaded {
        tab_id: TabId,
        request_id: u64,
        result: Result<DirectorySnapshot, FsError>,
        volume_space: Option<VolumeSpace>,
    },
    TextPreviewLoaded {
        request_id: u64,
        path: PathBuf,
        snippet: Option<String>,
    },
    OpenDefaultFinished(Result<(), FsError>),
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
    pub vfs: Arc<LocalFs>,
    pub places: UserPlaces,
    pub config: AppConfig,
    pub tabs: Vec<TabState>,
    pub active_tab: TabId,
    pub next_tab_id: u64,
    pub next_request_id: u64,
    pub tab_cancels: HashMap<TabId, CancellationToken>,
    pub preview_cancel: Option<CancellationToken>,
    pub preview_request_id: u64,
    pub drives: Vec<DriveInfo>,
    pub active_sidebar: Option<SidebarTarget>,
    pub path_editing: bool,
    pub path_draft: String,
    pub quicklook_open: bool,
    pub current_modifiers: keyboard::Modifiers,
    pub canvas_scroll_y: f32,
    pub canvas_viewport_height: f32,
    pub status_notice: Option<String>,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let vfs = Arc::new(LocalFs::new());
        let places = fm_fs::user_places();

        let tab1_path = places
            .documents
            .clone()
            .filter(|p| p.exists())
            .unwrap_or_else(|| places.home.clone());
        let tab2_path = std::env::current_dir()
            .ok()
            .filter(|p| p.exists())
            .unwrap_or_else(|| places.home.clone());

        let mut tab1 = TabState::new(TabId(1), tab1_path.clone());
        tab1.is_loading = true;
        tab1.active_request_id = 1;

        let mut tab2 = TabState::new(TabId(2), tab2_path.clone());
        tab2.is_loading = true;
        tab2.active_request_id = 2;

        let cancel1 = CancellationToken::new();
        let cancel2 = CancellationToken::new();

        let mut tab_cancels = HashMap::new();
        tab_cancels.insert(TabId(1), cancel1.clone());
        tab_cancels.insert(TabId(2), cancel2.clone());

        let app = Self {
            vfs: Arc::clone(&vfs),
            places,
            config: AppConfig::default(),
            tabs: vec![tab1, tab2],
            active_tab: TabId(1),
            next_tab_id: 3,
            next_request_id: 3,
            tab_cancels,
            preview_cancel: None,
            preview_request_id: 0,
            drives: Vec::new(),
            active_sidebar: Some(SidebarTarget::Documents),
            path_editing: false,
            path_draft: String::new(),
            quicklook_open: false,
            current_modifiers: keyboard::Modifiers::empty(),
            canvas_scroll_y: 0.0,
            canvas_viewport_height: 700.0,
            status_notice: None,
        };

        let drives_task = {
            let vfs = Arc::clone(&vfs);
            Task::perform(async move { vfs.list_drives().await }, Message::DrivesLoaded)
        };
        let load1 = spawn_list_task(Arc::clone(&vfs), TabId(1), tab1_path, 1, cancel1);
        let load2 = spawn_list_task(vfs, TabId(2), tab2_path, 2, cancel2);

        (app, Task::batch([drives_task, load1, load2]))
    }

    pub fn title(&self) -> String {
        let tab_title = self
            .current_tab()
            .map(|t| t.title.as_str())
            .unwrap_or("Vagnarok");
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
                    .unwrap_or_else(|| self.places.home.clone());

                let mut tab = TabState::new(id, path.clone());
                tab.active_request_id = req;
                tab.is_loading = true;
                self.tabs.push(tab);
                self.active_tab = id;
                self.path_editing = false;
                self.quicklook_open = false;

                let cancel = self.reset_tab_cancel(id);
                Task::batch([
                    spawn_list_task(Arc::clone(&self.vfs), id, path, req, cancel),
                    operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
                ])
            }
            Message::TabSelect(id) => {
                if self.tabs.iter().any(|t| t.id == id) {
                    self.active_tab = id;
                    self.path_editing = false;
                    self.quicklook_open = false;
                    self.canvas_scroll_y = 0.0;
                    return operation::snap_to(canvas_scroll_id(), RelativeOffset::START);
                }
                Task::none()
            }
            Message::TabClose(id) => {
                if self.tabs.len() <= 1 {
                    return Task::none();
                }
                if let Some(cancel) = self.tab_cancels.remove(&id) {
                    cancel.cancel();
                }
                if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                    self.tabs.remove(pos);
                    if self.active_tab == id {
                        let fallback = pos.saturating_sub(1).min(self.tabs.len() - 1);
                        self.active_tab = self.tabs[fallback].id;
                        self.canvas_scroll_y = 0.0;
                    }
                }
                Task::none()
            }
            Message::NavigateTo(path) => self.navigate_active_tab(path),
            Message::NavigateBack => {
                let req = self.alloc_request_id();
                let mut target_path = None;
                let active = self.active_tab;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(prev) = tab.history_back.pop()
                {
                    let cur = std::mem::replace(&mut tab.current_path, prev.clone());
                    tab.history_forward.push(cur);
                    prepare_tab_for_load(tab, req);
                    target_path = Some(prev);
                }
                if let Some(path) = target_path {
                    self.finish_navigation_tasks(active, path, req)
                } else {
                    Task::none()
                }
            }
            Message::NavigateForward => {
                let req = self.alloc_request_id();
                let mut target_path = None;
                let active = self.active_tab;
                if let Some(tab) = self.current_tab_mut()
                    && let Some(next) = tab.history_forward.pop()
                {
                    let cur = std::mem::replace(&mut tab.current_path, next.clone());
                    tab.history_back.push(cur);
                    prepare_tab_for_load(tab, req);
                    target_path = Some(next);
                }
                if let Some(path) = target_path {
                    self.finish_navigation_tasks(active, path, req)
                } else {
                    Task::none()
                }
            }
            Message::NavigateUp => {
                let parent = self
                    .current_tab()
                    .and_then(|t| t.current_path.parent().map(PathBuf::from));
                if let Some(p) = parent {
                    self.navigate_active_tab(p)
                } else {
                    Task::none()
                }
            }
            Message::RefreshActiveTab => {
                let req = self.alloc_request_id();
                let active = self.active_tab;
                let target_path = self.current_tab_mut().map(|tab| {
                    prepare_tab_for_load(tab, req);
                    tab.current_path.clone()
                });
                if let Some(path) = target_path {
                    let cancel = self.reset_tab_cancel(active);
                    spawn_list_task(Arc::clone(&self.vfs), active, path, req, cancel)
                } else {
                    Task::none()
                }
            }
            Message::SelectSidebar(target) => {
                if let Some(path) = target.resolve_path(&self.places) {
                    self.active_sidebar = Some(target);
                    self.navigate_active_tab(path)
                } else {
                    Task::none()
                }
            }
            Message::SelectDrive(path) => {
                self.active_sidebar = None;
                self.navigate_active_tab(path)
            }
            Message::SelectEntry { index, modifiers } => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.selection
                        .apply_click(index, modifiers, &tab.visible_entries);
                    let single_entry = tab
                        .selection
                        .single_selected(&tab.visible_entries)
                        .cloned();
                    tab.selected_telemetry = single_entry.as_ref().map(basic_file_telemetry);
                    if self.quicklook_open || self.config.show_inspector {
                        return self.maybe_load_preview(single_entry.as_ref());
                    }
                }
                Task::none()
            }
            Message::ActivateEntry(index) => {
                let target = self
                    .current_tab()
                    .and_then(|t| t.visible_entries.get(index).cloned());
                if let Some(entry) = target {
                    if entry.kind.is_dir_like() {
                        self.active_sidebar = None;
                        return self.navigate_active_tab(entry.path);
                    }
                    let vfs = Arc::clone(&self.vfs);
                    let path = entry.path;
                    return Task::perform(
                        async move { vfs.open_default(&path).await },
                        Message::OpenDefaultFinished,
                    );
                }
                Task::none()
            }
            Message::SetViewMode(mode) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.view_mode = mode;
                }
                self.canvas_scroll_y = 0.0;
                operation::snap_to(canvas_scroll_id(), RelativeOffset::START)
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
                }
                Task::none()
            }
            Message::FilterChanged(query) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.filter_query = query;
                    tab.recompute_visible();
                }
                self.canvas_scroll_y = 0.0;
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
                    self.active_sidebar = None;
                    return self.navigate_active_tab(PathBuf::from(raw));
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
                if self.config.show_inspector {
                    let single = self
                        .current_tab()
                        .and_then(|t| t.selection.single_selected(&t.visible_entries))
                        .cloned();
                    return self.maybe_load_preview(single.as_ref());
                }
                Task::none()
            }
            Message::ToggleHeroCard => {
                self.config.show_hero_card = !self.config.show_hero_card;
                Task::none()
            }
            Message::ToggleQuickLook => {
                let single = self
                    .current_tab()
                    .and_then(|t| t.selection.single_selected(&t.visible_entries))
                    .cloned();
                if self.quicklook_open {
                    self.quicklook_open = false;
                    if let Some(cancel) = self.preview_cancel.take() {
                        cancel.cancel();
                    }
                    return Task::none();
                }
                if let Some(entry) = single {
                    self.quicklook_open = true;
                    return self.maybe_load_preview(Some(&entry));
                }
                Task::none()
            }
            Message::CloseQuickLook => {
                self.quicklook_open = false;
                if let Some(cancel) = self.preview_cancel.take() {
                    cancel.cancel();
                }
                Task::none()
            }
            Message::CanvasScrolled {
                offset_y,
                viewport_height,
            } => {
                self.canvas_scroll_y = offset_y;
                if viewport_height > 0.0 {
                    self.canvas_viewport_height = viewport_height;
                }
                Task::none()
            }
            Message::DrivesLoaded(res) => {
                if let Ok(drives) = res {
                    self.drives = drives;
                }
                Task::none()
            }
            Message::DirectoryLoaded {
                tab_id,
                request_id,
                result,
                volume_space,
            } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    // Descarta respostas obsoletas de navegações canceladas.
                    if tab.active_request_id != request_id {
                        return Task::none();
                    }
                    tab.is_loading = false;
                    tab.telemetry.volume_space = volume_space;

                    match result {
                        Ok(snapshot) => {
                            tab.last_error = None;
                            tab.fs_type = snapshot.fs_type;
                            tab.raw_entries = snapshot.entries;
                            tab.recompute_visible();
                            tab.title = display_tab_title(&tab.current_path);
                        }
                        Err(FsError::Cancelled) => {}
                        Err(err) => {
                            tab.raw_entries.clear();
                            tab.visible_entries.clear();
                            tab.selection.clear();
                            tab.last_error = Some(err.to_string());
                        }
                    }
                }
                Task::none()
            }
            Message::TextPreviewLoaded {
                request_id,
                path,
                snippet,
            } => {
                if request_id != self.preview_request_id {
                    return Task::none();
                }
                if let Some(tab) = self.current_tab_mut()
                    && let Some(selected) = tab.selection.single_selected(&tab.visible_entries)
                    && selected.path == path
                {
                    let mut tel = tab
                        .selected_telemetry
                        .clone()
                        .unwrap_or_else(|| basic_file_telemetry(selected));
                    tel.preview_snippet = snippet;
                    tab.selected_telemetry = Some(tel);
                }
                Task::none()
            }
            Message::OpenDefaultFinished(res) => {
                self.status_notice = res.err().map(|e| e.to_string());
                Task::none()
            }
            Message::WindowMinimize => window::latest().and_then(|id| window::minimize(id, true)),
            Message::WindowToggleMaximize => window::latest().and_then(window::toggle_maximize),
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
                    || matches!(
                        key,
                        keyboard::Key::Named(Named::Escape | Named::F5)
                    );
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

    fn reset_tab_cancel(&mut self, tab_id: TabId) -> CancellationToken {
        if let Some(prev) = self.tab_cancels.remove(&tab_id) {
            prev.cancel();
        }
        let next = CancellationToken::new();
        self.tab_cancels.insert(tab_id, next.clone());
        next
    }

    fn navigate_active_tab(&mut self, path: PathBuf) -> Task<Message> {
        let req = self.alloc_request_id();
        let active = self.active_tab;
        if let Some(tab) = self.current_tab_mut() {
            tab.push_navigation(path.clone());
            prepare_tab_for_load(tab, req);
        }
        self.finish_navigation_tasks(active, path, req)
    }

    fn finish_navigation_tasks(
        &mut self,
        tab_id: TabId,
        path: PathBuf,
        request_id: u64,
    ) -> Task<Message> {
        self.path_editing = false;
        self.quicklook_open = false;
        self.status_notice = None;
        self.canvas_scroll_y = 0.0;
        let cancel = self.reset_tab_cancel(tab_id);
        Task::batch([
            spawn_list_task(Arc::clone(&self.vfs), tab_id, path, request_id, cancel),
            operation::snap_to(canvas_scroll_id(), RelativeOffset::START),
        ])
    }

    fn maybe_load_preview(&mut self, entry: Option<&fm_core::FileEntry>) -> Task<Message> {
        if let Some(prev) = self.preview_cancel.take() {
            prev.cancel();
        }
        let Some(entry) = entry else {
            return Task::none();
        };
        if entry.kind.is_dir_like() {
            return Task::none();
        }
        self.preview_request_id += 1;
        let req_id = self.preview_request_id;
        let cancel = CancellationToken::new();
        self.preview_cancel = Some(cancel.clone());
        let path = entry.path.clone();

        Task::perform(
            async move {
                let snippet = fm_preview::read_text_preview(&path, 64 * 1024, cancel).await;
                (req_id, path, snippet)
            },
            |(request_id, path, snippet)| Message::TextPreviewLoaded {
                request_id,
                path,
                snippet,
            },
        )
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
            keyboard::Key::Character("r") if modifiers.control() => {
                self.update(Message::RefreshActiveTab)
            }
            keyboard::Key::Named(Named::F5) => self.update(Message::RefreshActiveTab),
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
            keyboard::Key::Named(Named::ArrowRight | Named::ArrowDown) => self.move_selection(1),
            keyboard::Key::Named(Named::ArrowLeft | Named::ArrowUp) => self.move_selection(-1),
            keyboard::Key::Named(Named::Enter) if !self.path_editing => {
                if let Some(focused) = self.current_tab().and_then(|t| t.selection.focused) {
                    return self.update(Message::ActivateEntry(focused));
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn move_selection(&mut self, delta: isize) -> Task<Message> {
        let mut selected_entry = None;
        if let Some(tab) = self.current_tab_mut() {
            if tab.visible_entries.is_empty() {
                return Task::none();
            }
            let len = tab.visible_entries.len() as isize;
            let current = tab.selection.focused.unwrap_or(0) as isize;
            let next = (current + delta).clamp(0, len - 1) as usize;
            tab.selection.select_single(next, &tab.visible_entries);
            let single = tab
                .selection
                .single_selected(&tab.visible_entries)
                .cloned();
            tab.selected_telemetry = single.as_ref().map(basic_file_telemetry);
            selected_entry = single;
        }
        if self.quicklook_open || self.config.show_inspector {
            return self.maybe_load_preview(selected_entry.as_ref());
        }
        Task::none()
    }
}

fn spawn_list_task(
    vfs: Arc<LocalFs>,
    tab_id: TabId,
    path: PathBuf,
    request_id: u64,
    cancel: CancellationToken,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = vfs.list_dir(&path, request_id, cancel).await;
            let volume_space = if result.is_ok() {
                vfs.volume_space(&path).await.ok().flatten()
            } else {
                None
            };
            (tab_id, request_id, result, volume_space)
        },
        |(tab_id, request_id, result, volume_space)| Message::DirectoryLoaded {
            tab_id,
            request_id,
            result,
            volume_space,
        },
    )
}

fn prepare_tab_for_load(tab: &mut TabState, request_id: u64) {
    tab.active_request_id = request_id;
    tab.is_loading = true;
    tab.last_error = None;
    tab.title = display_tab_title(&tab.current_path);
}

fn display_tab_title(path: &std::path::Path) -> CompactString {
    path.file_name()
        .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CompactString::from(path.to_string_lossy().as_ref()))
}

fn basic_file_telemetry(entry: &fm_core::FileEntry) -> SelectedFileTelemetry {
    let mime = if entry.kind.is_dir_like() {
        Some(CompactString::new("inode/directory"))
    } else {
        entry
            .extension
            .as_ref()
            .map(|e| CompactString::from(format!("ext/{e}")))
    };
    SelectedFileTelemetry {
        sha256_prefix: None,
        sha256_full: None,
        mime_type: mime,
        preview_snippet: None,
    }
}

pub fn selected_id_set_contains(tab: &TabState, id: EntryId) -> bool {
    tab.selection.selected_ids.contains(&id)
}
