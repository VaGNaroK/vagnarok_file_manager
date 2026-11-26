use std::path::PathBuf;

use fm_core::{Strings, TabState, ThemePreset, ViewMode, WindowDecorationsMode};
use fm_theme::{FONT_MONO, FONT_UI, FONT_UI_SEMIBOLD, Spacing, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{Space, button, container, mouse_area, row, space, text, text_input, tooltip};
use iced::{Element, Length, Padding};
use lucide_icons::iced::{
    icon_arrow_left, icon_arrow_right, icon_arrow_up, icon_chevron_right, icon_columns_2,
    icon_home, icon_layout_grid, icon_list, icon_minus, icon_moon, icon_plus, icon_search,
    icon_square, icon_sun, icon_x,
};

use crate::app::{App, Message, TextInputTarget, path_input_id, search_input_id};
use crate::views::{sanitize_single_line, with_tooltip, wrap_text_input_context_menu};

pub fn view<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let top_row = tab_strip_row(app, palette, strings);
    let nav_row = navigation_row(app, tab, palette, strings);

    container(
        iced::widget::column![top_row, nav_row]
            .spacing(2)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Spacing::TITLEBAR_HEIGHT + Spacing::NAVBAR_HEIGHT)
    .padding(Padding {
        top: 4.0,
        right: 12.0,
        bottom: 4.0,
        left: 12.0,
    })
    .style(styles::header_container(palette))
    .into()
}

fn tab_strip_row<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mut tabs_row = row![].spacing(4).align_y(Center);

    for t in &app.tabs {
        let is_active = t.id == app.active_tab;
        let tab_id = t.id;
        let close_btn = with_tooltip(
            button(icon_x().size(12).color(palette.text_muted))
                .padding([2, 4])
                .style(styles::ghost_button(palette, 4.0))
                .on_press(Message::TabClose(tab_id)),
            strings.tip_close_tab,
            tooltip::Position::Bottom,
            palette,
        );

        let label = text(sanitize_single_line(t.title.as_str()))
            .size(12)
            .font(if is_active { FONT_UI_SEMIBOLD } else { FONT_UI })
            .wrapping(text::Wrapping::None);

        let tab_btn = button(row![label, close_btn].spacing(6).align_y(Center))
            .padding([4, 10])
            .style(styles::tab_button(palette, is_active))
            .on_press(Message::TabSelect(tab_id));

        tabs_row = tabs_row.push(mouse_area(tab_btn).on_middle_press(Message::TabClose(tab_id)));
    }

    let add_tab_btn = with_tooltip(
        button(icon_plus().size(14))
            .padding([5, 7])
            .style(styles::ghost_button(palette, 8.0))
            .on_press(Message::TabNew),
        strings.tip_new_tab,
        tooltip::Position::Bottom,
        palette,
    );
    tabs_row = tabs_row.push(add_tab_btn);

    let theme_icon = match app.config.theme {
        ThemePreset::AetherLight => icon_moon().size(14),
        ThemePreset::ObsidianFlow => icon_sun().size(14),
    };
    let theme_btn = with_tooltip(
        button(theme_icon)
            .padding([5, 8])
            .style(styles::ghost_button(palette, 8.0))
            .on_press(Message::ToggleTheme),
        strings.tip_theme,
        tooltip::Position::Bottom,
        palette,
    );

    let lang_label = app.config.language.short_code();
    let lang_btn = with_tooltip(
        button(text(lang_label).size(11).font(FONT_MONO))
            .padding([4, 8])
            .style(styles::ghost_button(palette, 8.0))
            .on_press(Message::OpenLanguageMenu),
        strings.tip_language,
        tooltip::Position::Bottom,
        palette,
    );

    let mut right_controls = row![].spacing(4).align_y(Center);
    if let Some(update) = app.visible_update() {
        let update_pill = with_tooltip(
            button(
                row![
                    icon_arrow_up().size(12).color(palette.on_accent),
                    text(update.tag_name.as_str())
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.on_accent)
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(4)
                .align_y(Center),
            )
            .padding([3, 9])
            .style(styles::pill_primary_button(palette))
            .on_press(Message::OpenUpdateReleasePage(update.release_url.clone())),
            strings.tip_update_available,
            tooltip::Position::Bottom,
            palette,
        );
        right_controls = right_controls.push(update_pill);
    }
    right_controls = right_controls.push(lang_btn).push(theme_btn);

    let show_csd = app.config.window_decorations == WindowDecorationsMode::IntegratedCsd;
    if show_csd {
        right_controls = right_controls.push(window_controls(palette));
    }

    let tabs_strip = mouse_area(tabs_row)
        .on_scroll(Message::TabStripScrolled)
        .on_exit(Message::TabStripScrollReset);

    row![tabs_strip, space::horizontal(), right_controls]
        .height(Spacing::TITLEBAR_HEIGHT - 4.0)
        .align_y(Center)
        .into()
}

fn window_controls<'a>(palette: ThemePalette) -> Element<'a, Message> {
    row![
        button(icon_minus().size(13))
            .padding([4, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::WindowMinimize),
        button(icon_square().size(11))
            .padding([4, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::WindowToggleMaximize),
        button(icon_x().size(13).color(palette.status_error))
            .padding([4, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::WindowClose),
    ]
    .spacing(2)
    .align_y(Center)
    .into()
}

fn navigation_row<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let can_back = tab.is_some_and(TabState::can_go_back);
    let can_fwd = tab.is_some_and(TabState::can_go_forward);
    let can_up = tab.is_some_and(|t| {
        !fm_core::is_virtual_readonly_dir(&t.current_path) && t.current_path.parent().is_some()
    });

    let mut back_btn = button(icon_arrow_left().size(15))
        .padding([5, 7])
        .style(styles::ghost_button(palette, 8.0));
    if can_back {
        back_btn = back_btn.on_press(Message::NavigateBack);
    }

    let mut fwd_btn = button(icon_arrow_right().size(15))
        .padding([5, 7])
        .style(styles::ghost_button(palette, 8.0));
    if can_fwd {
        fwd_btn = fwd_btn.on_press(Message::NavigateForward);
    }

    let mut up_btn = button(icon_arrow_up().size(15))
        .padding([5, 7])
        .style(styles::ghost_button(palette, 8.0));
    if can_up {
        up_btn = up_btn.on_press(Message::NavigateUp);
    }

    let nav_buttons = row![
        with_tooltip(
            back_btn,
            strings.tip_back,
            tooltip::Position::Bottom,
            palette
        ),
        with_tooltip(
            fwd_btn,
            strings.tip_forward,
            tooltip::Position::Bottom,
            palette
        ),
        with_tooltip(up_btn, strings.tip_up, tooltip::Position::Bottom, palette),
    ]
    .spacing(2)
    .align_y(Center);

    let breadcrumb = unified_breadcrumb(app, tab, palette, strings);

    let query = tab.map(|t| t.filter_query.as_str()).unwrap_or("");
    let deep_btn = with_tooltip(
        button(
            text(strings.deep_search)
                .size(10)
                .font(FONT_MONO)
                .color(palette.accent_primary),
        )
        .padding([2, 6])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::OpenDeepSearch),
        strings.tip_deep_search,
        tooltip::Position::Bottom,
        palette,
    );

    let search_box = container(
        row![
            icon_search().size(14).color(palette.text_muted),
            wrap_text_input_context_menu(
                text_input(strings.search_placeholder, query)
                    .id(search_input_id())
                    .on_input(Message::FilterChanged)
                    .on_submit(Message::FilterSearchSubmit)
                    .size(12)
                    .padding([4, 6])
                    .style(styles::search_input_style(palette))
                    .into(),
                app.main_window_id,
                TextInputTarget::SearchBar,
                query.to_owned(),
            ),
            deep_btn,
            container(
                text("Ctrl+K")
                    .size(10)
                    .font(FONT_MONO)
                    .color(palette.text_secondary),
            )
            .padding([2, 6])
            .style(styles::badge_container(
                palette.surface_high,
                palette.text_secondary,
                palette.border,
                6.0,
            )),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .width(Length::Fixed(310.0))
    .padding([2, 10])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 9999.0));

    let current_mode = tab.map(|t| t.view_mode).unwrap_or(ViewMode::Grid);
    let split_active = app.split_view.is_some();
    let split_btn = with_tooltip(
        button(icon_columns_2().size(14).color(if split_active {
            palette.accent_primary
        } else {
            palette.text_secondary
        }))
        .padding([4, 8])
        .style(styles::tab_button(palette, split_active))
        .on_press(Message::ToggleSplitView),
        strings.tip_split_view,
        tooltip::Position::Bottom,
        palette,
    );
    let view_switcher = container(
        row![
            with_tooltip(
                view_mode_btn(
                    icon_layout_grid().size(14),
                    ViewMode::Grid,
                    current_mode,
                    palette
                ),
                strings.tip_view_grid,
                tooltip::Position::Bottom,
                palette,
            ),
            with_tooltip(
                view_mode_btn(
                    icon_list().size(14),
                    ViewMode::DetailTable,
                    current_mode,
                    palette
                ),
                strings.tip_view_list,
                tooltip::Position::Bottom,
                palette,
            ),
            container(Space::new().width(1).height(16)).style(styles::divider_line(palette)),
            split_btn,
        ]
        .spacing(2)
        .align_y(Center),
    )
    .padding(2)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0));

    row![
        nav_buttons,
        breadcrumb,
        space::horizontal(),
        search_box,
        Space::new().width(8),
        view_switcher,
    ]
    .spacing(8)
    .height(Spacing::NAVBAR_HEIGHT - 4.0)
    .align_y(Center)
    .into()
}

fn unified_breadcrumb<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    if app.path_editing {
        let placeholder = app.places.home.to_string_lossy().into_owned();
        return container(wrap_text_input_context_menu(
            text_input(&placeholder, &app.path_draft)
                .id(path_input_id())
                .on_input(Message::PathDraftChanged)
                .on_submit(Message::CommitPathDraft)
                .size(12)
                .font(FONT_MONO)
                .padding([4, 10])
                .style(styles::search_input_style(palette))
                .into(),
            app.main_window_id,
            TextInputTarget::PathDraft,
            app.path_draft.clone(),
        ))
        .width(Length::Fixed(360.0))
        .into();
    }

    let mut crumbs = row![with_tooltip(
        button(icon_home().size(14).color(palette.accent_primary))
            .padding([3, 5])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::NavigateTo(app.places.home.clone())),
        strings.tip_home,
        tooltip::Position::Bottom,
        palette,
    )]
    .spacing(2)
    .align_y(Center);

    if let Some(tab) = tab {
        if fm_core::is_virtual_trash_path(&tab.current_path) {
            let txt = text(strings.trash)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None);
            crumbs = crumbs.push(icon_chevron_right().size(12).color(palette.text_muted));
            crumbs = crumbs.push(
                button(txt)
                    .padding([3, 6])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::NavigateTo(tab.current_path.clone())),
            );
        } else if fm_core::is_virtual_recent_path(&tab.current_path) {
            let txt = text(strings.recent_files)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None);
            crumbs = crumbs.push(icon_chevron_right().size(12).color(palette.text_muted));
            crumbs = crumbs.push(
                button(txt)
                    .padding([3, 6])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::NavigateTo(tab.current_path.clone())),
            );
        } else {
            let mut acc = PathBuf::new();
            let components: Vec<_> = tab
                .current_path
                .components()
                .filter_map(|c| match c {
                    std::path::Component::RootDir => {
                        acc.push("/");
                        if acc == std::path::Path::new("/") {
                            Some(("/".to_owned(), acc.clone()))
                        } else {
                            None
                        }
                    }
                    std::path::Component::Normal(part) => {
                        acc.push(part);
                        Some((sanitize_single_line(&part.to_string_lossy()), acc.clone()))
                    }
                    std::path::Component::Prefix(prefix) => {
                        let s = sanitize_single_line(&prefix.as_os_str().to_string_lossy());
                        acc.push(prefix.as_os_str());
                        Some((s, acc.clone()))
                    }
                    _ => None,
                })
                .collect();

            let last_idx = components.len().saturating_sub(1);
            for (idx, (label, target)) in components.into_iter().enumerate() {
                let is_last = idx == last_idx;
                let txt = text(label)
                    .size(12)
                    .font(if is_last { FONT_UI_SEMIBOLD } else { FONT_UI })
                    .color(if is_last {
                        palette.text_primary
                    } else {
                        palette.text_secondary
                    })
                    .wrapping(text::Wrapping::None);
                let is_menu_open = app
                    .breadcrumb_menu
                    .as_ref()
                    .is_some_and(|(open_path, _)| open_path == &target);
                crumbs = crumbs.push(
                    button(txt)
                        .padding([3, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::NavigateTo(target.clone())),
                );
                crumbs = crumbs.push(
                    button(icon_chevron_right().size(12).color(if is_menu_open {
                        palette.accent_primary
                    } else {
                        palette.text_muted
                    }))
                    .padding([3, 3])
                    .style(styles::ghost_button(palette, 4.0))
                    .on_press(Message::ToggleBreadcrumbMenu(target)),
                );
            }
        }
    }

    let edit_hint_btn = with_tooltip(
        button(
            text("Ctrl+L")
                .size(10)
                .font(FONT_MONO)
                .color(palette.text_muted),
        )
        .padding([2, 6])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::TogglePathEdit),
        strings.tip_edit_path,
        tooltip::Position::Bottom,
        palette,
    );

    container(row![crumbs, edit_hint_btn].spacing(6).align_y(Center))
        .padding([2, 8])
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0))
        .into()
}

fn view_mode_btn<'a>(
    icon: iced::widget::Text<'a, iced::Theme>,
    mode: ViewMode,
    active: ViewMode,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let is_active = mode == active;
    button(icon.color(if is_active {
        palette.accent_primary
    } else {
        palette.text_secondary
    }))
    .padding([4, 8])
    .style(styles::tab_button(palette, is_active))
    .on_press(Message::SetViewMode(mode))
    .into()
}
