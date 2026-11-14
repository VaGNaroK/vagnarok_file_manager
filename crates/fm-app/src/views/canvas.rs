use fm_core::{
    FileEntry, SelectionModifiers, SortDirection, SortField, Strings, TabState, ViewMode,
    format_bytes,
};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles,
};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, responsive, row, scrollable, space, text,
};
use iced::{Color, Element, Length, Padding};
use lucide_icons::iced::{
    icon_arrow_up_down, icon_chevron_right, icon_eye, icon_eye_off, icon_info, icon_plus,
    icon_terminal,
};

use crate::app::{App, Message, selected_id_set_contains};
use crate::views::entry_icon;

pub fn view<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let Some(tab) = tab else {
        return container(text("Nenhuma aba ativa"))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let mut content = column![action_bar(app, tab, palette, strings)].spacing(16);

    if app.config.show_hero_card {
        content = content.push(hero_card(tab, palette, strings));
    }

    let body: Element<'a, Message> = match tab.view_mode {
        ViewMode::Grid => grid_view(app, tab, palette, strings),
        ViewMode::DetailTable => detail_table_view(app, tab, palette, strings),
        ViewMode::Columns => columns_view(app, tab, palette, strings),
    };

    content = content.push(body);

    container(scrollable(content).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::from([14, 20]))
        .style(styles::canvas_container(palette))
        .into()
}

fn action_bar<'a>(
    app: &'a App,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mut left_info = row![
        text(tab.title.as_str())
            .size(14)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.text_primary),
    ]
    .spacing(10)
    .align_y(Center);

    if let Some(branch) = &tab.telemetry.git_branch {
        left_info = left_info.push(
            container(
                row![
                    container(Space::new().width(6).height(6)).style(move |_| {
                        styles::badge_container(
                            palette.status_success,
                            palette.status_success,
                            Color::TRANSPARENT,
                            9999.0,
                        )(&iced::Theme::Light)
                    }),
                    text(format!("git:({branch})"))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.status_success),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([3, 10])
            .style(styles::badge_container(
                palette.status_success_bg,
                palette.status_success,
                palette.status_success,
                9999.0,
            )),
        );
    }

    let sort_name = match tab.sort.field {
        SortField::Name => strings.col_name,
        SortField::Size => strings.col_size,
        SortField::Modified => strings.col_modified,
        SortField::Kind => strings.col_type,
    };
    let dir_arrow = match tab.sort.direction {
        SortDirection::Ascending => "↑",
        SortDirection::Descending => "↓",
    };

    let new_btn = button(
        row![icon_plus().size(14), text(strings.new_item).size(12).font(FONT_UI_SEMIBOLD)]
            .spacing(4)
            .align_y(Center),
    )
    .padding([6, 14])
    .style(styles::pill_primary_button(palette))
    .on_press(Message::Noop);

    let sort_btn = button(
        row![
            icon_arrow_up_down().size(13),
            text(format!("{}: {sort_name}", strings.sort)).size(12),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([6, 12])
    .style(styles::pill_secondary_button(palette))
    .on_press(Message::CycleSortField);

    let sort_dir_btn = button(text(dir_arrow).size(12).font(FONT_MONO_BOLD))
        .padding([6, 10])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::ToggleSortDirection);

    let hidden_icon = if tab.show_hidden {
        icon_eye().size(13)
    } else {
        icon_eye_off().size(13)
    };
    let hidden_btn = button(
        row![hidden_icon, text(strings.hidden).size(12)]
            .spacing(6)
            .align_y(Center),
    )
    .padding([6, 12])
    .style(styles::pill_secondary_button(palette))
    .on_press(Message::ToggleHidden);

    let info_btn = button(icon_info().size(16).color(if app.config.show_inspector {
        palette.accent_primary
    } else {
        palette.text_secondary
    }))
    .padding([6, 8])
    .style(styles::ghost_button(palette, 9999.0))
    .on_press(Message::ToggleInspector);

    row![
        left_info,
        space::horizontal(),
        row![new_btn, sort_btn, sort_dir_btn, hidden_btn, info_btn]
            .spacing(8)
            .align_y(Center),
    ]
    .align_y(Center)
    .into()
}

fn hero_card<'a>(
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let stats = tab.telemetry.folder_stats;
    let folder_name = tab
        .current_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| tab.current_path.to_string_lossy().into_owned());

    let left = row![
        container(icon_terminal().size(24).color(palette.accent_primary))
            .width(46)
            .height(46)
            .align_x(Center)
            .align_y(Center)
            .style(move |_| styles::panel_surface(
                palette.accent_surface,
                palette.accent_border,
                10.0
            )),
        column![
            row![
                text(folder_name)
                    .size(18)
                    .font(FONT_UI_BOLD)
                    .color(palette.text_primary),
                container(
                    text("ext4")
                        .size(11)
                        .font(FONT_MONO_BOLD)
                        .color(palette.accent_primary)
                )
                .padding([2, 8])
                .style(styles::badge_container(
                    palette.accent_surface,
                    palette.accent_primary,
                    palette.accent_border,
                    6.0
                )),
            ]
            .spacing(8)
            .align_y(Center),
            text(tab.current_path.to_string_lossy().into_owned())
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        ]
        .spacing(3),
    ]
    .spacing(14)
    .align_y(Center);

    let metrics = row![
        metric_block(
            strings.folders,
            format!("{} {}", stats.dir_count, strings.folders_unit),
            palette.text_primary,
            palette,
        ),
        metric_block(
            strings.files,
            format!("{} {}", stats.file_count, strings.files_unit),
            palette.text_primary,
            palette,
        ),
        metric_block(
            strings.total_size,
            format_bytes(stats.total_direct_bytes).to_string(),
            palette.accent_primary,
            palette,
        ),
    ]
    .spacing(28)
    .align_y(Center);

    container(row![left, space::horizontal(), metrics].align_y(Center))
        .width(Length::Fill)
        .padding([16, 20])
        .style(styles::hero_card(palette))
        .into()
}

fn metric_block<'a>(
    label: &'a str,
    value: String,
    value_color: Color,
    palette: ThemePalette,
) -> Element<'a, Message> {
    column![
        text(label)
            .size(10)
            .font(FONT_MONO)
            .color(palette.text_muted),
        text(value)
            .size(13)
            .font(FONT_MONO_BOLD)
            .color(value_color),
    ]
    .spacing(2)
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}

fn grid_view<'a>(
    app: &'a App,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let dirs: Vec<(usize, &FileEntry)> = tab
        .visible_entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind.is_dir_like())
        .collect();
    let files: Vec<(usize, &FileEntry)> = tab
        .visible_entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.kind.is_dir_like())
        .collect();

    let modifiers = SelectionModifiers {
        ctrl: app.current_modifiers.control(),
        shift: app.current_modifiers.shift(),
    };

    responsive(move |size| {
        let cols = ((size.width / 150.0).floor() as usize).clamp(2, 8);
        let mut sections = column![].spacing(22).width(Length::Fill);

        if !dirs.is_empty() {
            sections = sections.push(section_title_bar(
                strings.folders,
                dirs.len(),
                "drwxr-xr-x",
                palette,
            ));
            sections = sections.push(build_grid_rows(&dirs, cols, tab, modifiers, palette, strings));
        }

        if !files.is_empty() {
            sections = sections.push(section_title_bar(
                strings.files,
                files.len(),
                "-rw-r--r--",
                palette,
            ));
            sections =
                sections.push(build_grid_rows(&files, cols, tab, modifiers, palette, strings));
        }

        sections.into()
    })
    .into()
}

fn section_title_bar<'a>(
    title: &'a str,
    count: usize,
    perms_hint: &'a str,
    palette: ThemePalette,
) -> Element<'a, Message> {
    row![
        text(format!("{title} • {count}"))
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(palette.text_muted),
        space::horizontal(),
        text(perms_hint)
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_muted),
    ]
    .align_y(Center)
    .into()
}

fn build_grid_rows<'a>(
    items: &[(usize, &'a FileEntry)],
    cols: usize,
    tab: &'a TabState,
    modifiers: SelectionModifiers,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mut rows_col = column![].spacing(16).width(Length::Fill);

    for chunk in items.chunks(cols) {
        let mut r = row![].spacing(12).width(Length::Fill);
        for &(idx, entry) in chunk {
            r = r.push(grid_item_card(
                idx,
                entry,
                selected_id_set_contains(tab, entry.id),
                modifiers,
                palette,
                strings,
            ));
        }
        // Preenche colunas faltantes para manter alinhamento uniforme da grade.
        for _ in chunk.len()..cols {
            r = r.push(Space::new().width(Length::Fill));
        }
        rows_col = rows_col.push(r);
    }

    rows_col.into()
}

fn grid_item_card<'a>(
    index: usize,
    entry: &'a FileEntry,
    selected: bool,
    modifiers: SelectionModifiers,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (icon_widget, _color) = entry_icon(entry, palette, 36.0);

    let mut icon_box_col = column![icon_widget].align_x(Center);
    if let Some(badge) = &entry.mime_badge {
        let badge_bg = if selected {
            palette.accent_primary
        } else {
            palette.surface_low
        };
        let badge_fg = if selected {
            palette.on_accent
        } else {
            palette.text_secondary
        };
        icon_box_col = icon_box_col.push(
            container(text(badge.as_str()).size(9).font(FONT_MONO_BOLD).color(badge_fg))
                .padding([1, 5])
                .style(styles::badge_container(
                    badge_bg,
                    badge_fg,
                    palette.border,
                    4.0,
                )),
        );
    }

    let sub_label = if entry.kind.is_dir_like() {
        format!(
            "{} {}",
            entry.child_count.unwrap_or(0),
            strings.items_unit
        )
    } else {
        format_bytes(entry.size_bytes).to_string()
    };

    let name_widget = if selected {
        container(
            text(entry.name.as_str())
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.accent_primary),
        )
        .padding([2, 8])
        .style(styles::badge_container(
            palette.accent_surface,
            palette.accent_primary,
            palette.accent_border,
            9999.0,
        ))
        .into()
    } else {
        Element::from(
            text(entry.name.as_str())
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary),
        )
    };

    let card_content = column![
        container(icon_box_col)
            .width(68)
            .height(68)
            .align_x(Center)
            .align_y(Center),
        name_widget,
        text(sub_label)
            .size(11)
            .font(FONT_MONO)
            .color(if selected {
                palette.accent_primary
            } else {
                palette.text_secondary
            }),
    ]
    .spacing(4)
    .align_x(Center)
    .width(Length::Fill);

    button(card_content)
        .width(Length::Fill)
        .padding([8, 6])
        .style(styles::entry_card_button(palette, selected, 14.0))
        .on_press(Message::SelectEntry { index, modifiers })
        .into()
}

fn detail_table_view<'a>(
    app: &'a App,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let modifiers = SelectionModifiers {
        ctrl: app.current_modifiers.control(),
        shift: app.current_modifiers.shift(),
    };

    let header = container(
        row![
            text(strings.col_name)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted)
                .width(Length::FillPortion(4)),
            text(strings.col_size)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted)
                .width(Length::FillPortion(1)),
            text(strings.col_type)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted)
                .width(Length::FillPortion(1)),
            text(strings.col_perms)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted)
                .width(Length::FillPortion(2)),
        ]
        .padding([6, 12]),
    )
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0));

    let mut list = column![header].spacing(4).width(Length::Fill);

    for (idx, entry) in tab.visible_entries.iter().enumerate() {
        let selected = selected_id_set_contains(tab, entry.id);
        let (icon_widget, _) = entry_icon(entry, palette, 16.0);
        let size_txt = if entry.kind.is_dir_like() {
            "—".to_owned()
        } else {
            format_bytes(entry.size_bytes).to_string()
        };
        let kind_txt = if entry.kind.is_dir_like() {
            "DIR"
        } else {
            entry.extension.as_deref().unwrap_or("FILE")
        };
        let perms = entry.permissions.format_compact(entry.kind.is_dir_like());

        let row_content = row![
            row![
                icon_widget,
                text(entry.name.as_str())
                    .size(13)
                    .font(if selected { FONT_UI_SEMIBOLD } else { FONT_UI })
                    .color(palette.text_primary),
            ]
            .spacing(10)
            .align_y(Center)
            .width(Length::FillPortion(4)),
            text(size_txt)
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_secondary)
                .width(Length::FillPortion(1)),
            text(kind_txt)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_muted)
                .width(Length::FillPortion(1)),
            text(perms.as_str().to_owned())
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_secondary)
                .width(Length::FillPortion(2)),
        ]
        .align_y(Center);

        list = list.push(
            button(row_content)
                .width(Length::Fill)
                .padding([7, 12])
                .style(styles::entry_card_button(palette, selected, 8.0))
                .on_press(Message::SelectEntry {
                    index: idx,
                    modifiers,
                }),
        );
    }

    list.into()
}

fn columns_view<'a>(
    app: &'a App,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let modifiers = SelectionModifiers {
        ctrl: app.current_modifiers.control(),
        shift: app.current_modifiers.shift(),
    };

    let parent_name = tab
        .current_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_owned());

    let col1 = container(
        column![
            text(parent_name)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted),
            row![
                text(tab.title.as_str())
                    .size(13)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.accent_primary),
                space::horizontal(),
                icon_chevron_right().size(14).color(palette.accent_primary),
            ]
            .align_y(Center),
        ]
        .spacing(10),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fixed(380.0))
    .padding(12)
    .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0));

    let mut active_col = column![
        text(tab.title.as_str())
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(palette.text_muted)
    ]
    .spacing(4);

    for (idx, entry) in tab.visible_entries.iter().enumerate() {
        let selected = selected_id_set_contains(tab, entry.id);
        let (icon_widget, _) = entry_icon(entry, palette, 15.0);
        let mut r = row![
            icon_widget,
            text(entry.name.as_str()).size(13),
            space::horizontal(),
        ]
        .spacing(8)
        .align_y(Center);
        if entry.kind.is_dir_like() {
            r = r.push(icon_chevron_right().size(13).color(palette.text_muted));
        }
        active_col = active_col.push(
            button(r)
                .width(Length::Fill)
                .padding([5, 8])
                .style(styles::entry_card_button(palette, selected, 6.0))
                .on_press(Message::SelectEntry {
                    index: idx,
                    modifiers,
                }),
        );
    }

    let col2 = container(scrollable(active_col))
        .width(Length::FillPortion(2))
        .height(Length::Fixed(380.0))
        .padding(12)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0));

    let preview_col = if let Some(selected) = tab.selection.single_selected(&tab.visible_entries) {
        let (icon_widget, _) = entry_icon(selected, palette, 42.0);
        column![
            icon_widget,
            text(selected.name.as_str())
                .size(14)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary),
            text(format_bytes(selected.size_bytes).to_string())
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_secondary),
            text(
                selected
                    .permissions
                    .format_compact(selected.kind.is_dir_like())
                    .as_str()
                    .to_owned(),
            )
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_muted),
        ]
        .spacing(8)
        .align_x(Center)
    } else {
        column![
            text(strings.inspector_empty)
                .size(12)
                .color(palette.text_muted)
        ]
    };

    let col3 = container(preview_col)
        .width(Length::FillPortion(2))
        .height(Length::Fixed(380.0))
        .padding(16)
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0));

    row![col1, col2, col3].spacing(12).width(Length::Fill).into()
}
