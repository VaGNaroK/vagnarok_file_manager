use std::path::Path;

use fm_core::{
    ConflictPolicy, DIR_FILTER_SENTINEL, FileEntry, Language, PermissionsMeta, Strings,
    ThemePreset, ViewMode, WindowDecorationsMode, format_bytes,
};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles,
};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, mouse_area, pick_list, row, scrollable, space, text,
    text_input,
};
use iced::{Background, Border, Color, Element, Length, Padding, border::Radius, window};
use lucide_icons::iced::{
    icon_check, icon_check_square, icon_clipboard_copy, icon_clipboard_paste, icon_copy,
    icon_external_link, icon_eye, icon_file_plus, icon_file_text, icon_filter, icon_folder,
    icon_folder_plus, icon_hard_drive, icon_info, icon_lock, icon_pencil, icon_rotate_ccw,
    icon_scissors, icon_settings, icon_shield_check, icon_sliders_horizontal, icon_star_off,
    icon_terminal, icon_trash_2, icon_x,
};

use crate::app::{
    ActiveDialog, App, ContextMenuKind, ContextMenuState, Message, PreferencesTab, PropertiesTab,
    PropertiesWindowState, dialog_input_id,
};
use crate::views::{entry_icon, sanitize_single_line};

pub fn dialog_modal<'a>(
    dialog: &'a ActiveDialog,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let body: Element<'a, Message> = match dialog {
        ActiveDialog::CreateItem { is_dir, name } => {
            let folder_pill = button(
                row![
                    icon_folder_plus().size(13),
                    text(strings.new_folder).size(12)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([6, 12])
            .style(styles::tab_button(palette, *is_dir))
            .on_press(Message::DialogSetCreateIsDir(true));

            let file_pill = button(
                row![icon_file_plus().size(13), text(strings.new_file).size(12)]
                    .spacing(6)
                    .align_y(Center),
            )
            .padding([6, 12])
            .style(styles::tab_button(palette, !*is_dir))
            .on_press(Message::DialogSetCreateIsDir(false));

            let input = container(
                text_input(strings.name_label, name)
                    .id(dialog_input_id())
                    .on_input(Message::DialogInputChanged)
                    .on_submit(Message::SubmitDialog)
                    .size(13)
                    .padding([6, 10])
                    .style(styles::search_input_style(palette)),
            )
            .padding([2, 6])
            .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 8.0));

            let actions = row![
                space::horizontal(),
                button(text(strings.cancel).size(12))
                    .padding([6, 14])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CloseDialog),
                button(text(strings.create).size(12).font(FONT_UI_SEMIBOLD))
                    .padding([6, 16])
                    .style(styles::pill_primary_button(palette))
                    .on_press(Message::SubmitDialog),
            ]
            .spacing(8)
            .align_y(Center);

            column![
                row![
                    text(if *is_dir {
                        strings.new_folder
                    } else {
                        strings.new_file
                    })
                    .size(15)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                    space::horizontal(),
                    button(icon_x().size(14))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::CloseDialog),
                ]
                .align_y(Center),
                row![folder_pill, file_pill].spacing(8),
                input,
                actions,
            ]
            .spacing(14)
            .into()
        }
        ActiveDialog::Rename { src: _, name } => {
            let input = container(
                text_input(strings.name_label, name)
                    .id(dialog_input_id())
                    .on_input(Message::DialogInputChanged)
                    .on_submit(Message::SubmitDialog)
                    .size(13)
                    .padding([6, 10])
                    .style(styles::search_input_style(palette)),
            )
            .padding([2, 6])
            .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 8.0));

            let actions = row![
                space::horizontal(),
                button(text(strings.cancel).size(12))
                    .padding([6, 14])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CloseDialog),
                button(text(strings.confirm).size(12).font(FONT_UI_SEMIBOLD))
                    .padding([6, 16])
                    .style(styles::pill_primary_button(palette))
                    .on_press(Message::SubmitDialog),
            ]
            .spacing(8)
            .align_y(Center);

            column![
                row![
                    icon_pencil().size(15).color(palette.accent_primary),
                    text(strings.rename)
                        .size(15)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    space::horizontal(),
                    button(icon_x().size(14))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::CloseDialog),
                ]
                .spacing(8)
                .align_y(Center),
                input,
                actions,
            ]
            .spacing(14)
            .into()
        }
        ActiveDialog::ConfirmDelete { paths, permanent } => {
            let title = if *permanent {
                strings.delete_permanent
            } else {
                strings.move_to_trash
            };
            let warn = if *permanent {
                strings.confirm_delete_msg
            } else {
                strings.confirm_trash_msg
            };

            let mut list_preview = column![].spacing(4);
            for p in paths.iter().take(6) {
                let label = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.to_string_lossy().into_owned());
                list_preview = list_preview.push(
                    text(format!("• {label}"))
                        .size(12)
                        .font(FONT_MONO)
                        .color(palette.text_secondary),
                );
            }
            if paths.len() > 6 {
                list_preview = list_preview.push(
                    text(format!("+ {} ...", paths.len() - 6))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                );
            }

            let actions = row![
                space::horizontal(),
                button(text(strings.cancel).size(12))
                    .padding([6, 14])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CloseDialog),
                button(text(strings.confirm).size(12).font(FONT_UI_SEMIBOLD))
                    .padding([6, 16])
                    .style(styles::pill_primary_button(palette))
                    .on_press(Message::SubmitDialog),
            ]
            .spacing(8)
            .align_y(Center);

            column![
                row![
                    icon_trash_2().size(16).color(if *permanent {
                        palette.status_error
                    } else {
                        palette.accent_primary
                    }),
                    text(title)
                        .size(15)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    space::horizontal(),
                    button(icon_x().size(14))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::CloseDialog),
                ]
                .spacing(8)
                .align_y(Center),
                text(warn).size(13).font(FONT_UI).color(if *permanent {
                    palette.status_error
                } else {
                    palette.text_secondary
                }),
                container(list_preview)
                    .width(Length::Fill)
                    .padding(10)
                    .style(move |_| styles::panel_surface(
                        palette.surface_low,
                        palette.border,
                        8.0
                    )),
                actions,
            ]
            .spacing(12)
            .into()
        }
        ActiveDialog::ConfirmEmptyTrash => {
            let actions = row![
                space::horizontal(),
                button(text(strings.cancel).size(12))
                    .padding([6, 14])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CloseDialog),
                button(text(strings.empty_trash).size(12).font(FONT_UI_SEMIBOLD))
                    .padding([6, 16])
                    .style(styles::pill_primary_button(palette))
                    .on_press(Message::SubmitDialog),
            ]
            .spacing(8)
            .align_y(Center);

            column![
                row![
                    icon_trash_2().size(16).color(palette.status_error),
                    text(strings.empty_trash)
                        .size(15)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    space::horizontal(),
                    button(icon_x().size(14))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::CloseDialog),
                ]
                .spacing(8)
                .align_y(Center),
                text(strings.confirm_empty_trash_msg)
                    .size(13)
                    .font(FONT_UI)
                    .color(palette.status_error),
                actions,
            ]
            .spacing(12)
            .into()
        }
        ActiveDialog::Conflict {
            mode: _,
            sources,
            dest_dir: _,
            conflicts,
        } => {
            let mut list_preview = column![].spacing(4);
            for p in sources.iter().take(5) {
                let label = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.to_string_lossy().into_owned());
                list_preview = list_preview.push(
                    text(format!("• {label}"))
                        .size(12)
                        .font(FONT_MONO)
                        .color(palette.text_secondary),
                );
            }
            if sources.len() > 5 {
                list_preview = list_preview.push(
                    text(format!("+ {} ...", sources.len() - 5))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                );
            }

            let actions = column![
                row![
                    button(
                        text(strings.conflict_rename)
                            .size(12)
                            .font(FONT_UI_SEMIBOLD),
                    )
                    .width(Length::Fill)
                    .padding([8, 12])
                    .style(styles::pill_primary_button(palette))
                    .on_press(Message::ResolveConflict(ConflictPolicy::RenameAuto)),
                ],
                row![
                    button(text(strings.conflict_overwrite).size(12))
                        .width(Length::Fill)
                        .padding([7, 12])
                        .style(styles::pill_secondary_button(palette))
                        .on_press(Message::ResolveConflict(ConflictPolicy::Overwrite)),
                    button(text(strings.conflict_skip).size(12))
                        .width(Length::Fill)
                        .padding([7, 12])
                        .style(styles::pill_secondary_button(palette))
                        .on_press(Message::ResolveConflict(ConflictPolicy::Skip)),
                ]
                .spacing(8),
                row![
                    space::horizontal(),
                    button(text(strings.cancel).size(12))
                        .padding([6, 14])
                        .style(styles::ghost_button(palette, 8.0))
                        .on_press(Message::CloseDialog),
                ],
            ]
            .spacing(8);

            column![
                row![
                    text(format!("{} ({conflicts})", strings.conflict_title))
                        .size(15)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    space::horizontal(),
                    button(icon_x().size(14))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::CloseDialog),
                ]
                .align_y(Center),
                text(strings.conflict_msg)
                    .size(13)
                    .font(FONT_UI)
                    .color(palette.text_secondary),
                container(list_preview)
                    .width(Length::Fill)
                    .padding(10)
                    .style(move |_| styles::panel_surface(
                        palette.surface_low,
                        palette.border,
                        8.0
                    )),
                actions,
            ]
            .spacing(12)
            .into()
        }
    };

    let card = container(column![body].width(Length::Fixed(460.0)))
        .padding(20)
        .style(styles::modal_card(palette));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::modal_backdrop()(&iced::Theme::Light))
        .into()
}

pub fn properties_modal_overlay<'a>(
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let card = container(properties_window_view(win_id, props, palette, strings))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(520.0))
        .style(styles::modal_card(palette));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::modal_backdrop()(&iced::Theme::Light))
        .into()
}

pub fn properties_window_view<'a>(
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let entry = &props.entry;
    let is_dir = entry.kind.is_dir_like();
    let is_archive = props.is_archive();
    let dirty = props.is_dirty();

    // 1. Cabeçalho arrastável da janela flutuante
    let header_title = format!(
        "{}{}",
        strings.properties_header_prefix,
        sanitize_single_line(entry.name.as_str())
    );
    let drag_header = mouse_area(
        row![
            container(icon_info().size(15).color(palette.accent_primary))
                .padding(6)
                .style(move |_| styles::panel_surface(
                    palette.accent_surface,
                    palette.accent_border,
                    8.0
                )),
            container(
                text(header_title)
                    .size(15)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .on_press(Message::DragPropertiesWindow(win_id));

    let header = row![
        container(drag_header).width(Length::Fill),
        button(icon_x().size(14))
            .padding([5, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::ClosePropertiesWindow(win_id)),
    ]
    .spacing(8)
    .align_y(Center);

    // 2. Barra de abas (Geral | Segurança | Histórico | Metadados | Permissões)
    let tab_item = |tab: PropertiesTab, label: &'a str| {
        modal_tab_button(
            label,
            props.active_tab == tab,
            palette,
            Message::PropertiesSelectTab(win_id, tab),
        )
    };

    let tab_bar = column![
        row![
            tab_item(PropertiesTab::General, strings.prop_tab_general),
            tab_item(PropertiesTab::Security, strings.prop_tab_security),
            tab_item(PropertiesTab::History, strings.prop_tab_history),
            tab_item(PropertiesTab::Metadata, strings.prop_tab_metadata),
            tab_item(PropertiesTab::Permissions, strings.prop_tab_permissions),
        ]
        .spacing(4)
        .width(Length::Fill),
        container(Space::new().width(Length::Fill).height(1)).style(styles::divider_line(palette)),
    ];

    // 3. Conteúdo da aba ativa
    let tab_body: Element<'a, Message> = match props.active_tab {
        PropertiesTab::General => properties_general_tab(win_id, props, palette, strings, is_dir),
        PropertiesTab::Security => properties_security_tab(props, palette, strings, is_dir),
        PropertiesTab::History => properties_history_tab(props, palette, strings),
        PropertiesTab::Metadata => properties_metadata_tab(props, palette, strings, is_dir),
        PropertiesTab::Permissions => properties_permissions_tab(win_id, props, palette, strings),
    };

    // 4. Rodapé (Confirmar | Cancelar | Aplicar)
    let status_hint: Element<'a, Message> = if is_archive {
        text(strings.prop_archive_notice)
            .size(11)
            .font(FONT_UI)
            .color(palette.text_muted)
            .into()
    } else if dirty {
        text(strings.prop_pending_changes)
            .size(11)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.accent_primary)
            .into()
    } else {
        Space::new().width(0).height(0).into()
    };

    let confirm_btn = button(text(strings.prop_confirm).size(12).font(FONT_UI_SEMIBOLD))
        .padding([7, 16])
        .style(styles::pill_primary_button(palette))
        .on_press_maybe((!props.applying).then_some(Message::PropertiesApply {
            window_id: win_id,
            close_after: true,
        }));

    let cancel_btn = button(text(strings.cancel).size(12).font(FONT_UI))
        .padding([7, 16])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::ClosePropertiesWindow(win_id));

    let apply_active = dirty && !props.applying;
    let apply_btn = button(text(strings.prop_apply).size(12).font(if apply_active {
        FONT_UI_SEMIBOLD
    } else {
        FONT_UI
    }))
    .padding([7, 16])
    .style(move |theme, status| {
        if apply_active {
            styles::pill_primary_button(palette)(theme, status)
        } else {
            button::Style {
                background: Some(Background::Color(palette.surface_high)),
                text_color: palette.text_muted,
                border: Border {
                    color: palette.border,
                    width: 1.0,
                    radius: Radius::from(9999.0),
                },
                ..Default::default()
            }
        }
    })
    .on_press_maybe(apply_active.then_some(Message::PropertiesApply {
        window_id: win_id,
        close_after: false,
    }));

    let footer = row![
        status_hint,
        space::horizontal(),
        row![confirm_btn, cancel_btn, apply_btn].spacing(8),
    ]
    .align_y(Center);

    container(
        column![
            header,
            tab_bar,
            container(scrollable(tab_body).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            footer,
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(18)
    .style(styles::canvas_container(palette))
    .into()
}

fn properties_general_tab<'a>(
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
    is_dir: bool,
) -> Element<'a, Message> {
    let entry = &props.entry;
    let is_archive = props.is_archive();
    let (hero_icon, _) = entry_icon(entry, palette, 28.0);

    // Caixa Hero (ícone 52x52 + campo editável do nome)
    let icon_tile = container(hero_icon)
        .width(Length::Fixed(52.0))
        .height(Length::Fixed(52.0))
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::panel_surface(palette.accent_surface, palette.accent_border, 10.0));

    let name_control: Element<'a, Message> = if is_archive {
        text(sanitize_single_line(entry.name.as_str()))
            .size(16)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.text_primary)
            .wrapping(text::Wrapping::None)
            .into()
    } else {
        text_input("", &props.draft_name)
            .size(15)
            .padding([8, 12])
            .style(move |_, status| {
                let border_color = match status {
                    text_input::Status::Focused { .. } => palette.accent_primary,
                    text_input::Status::Hovered => palette.border_strong,
                    _ => Color::TRANSPARENT,
                };
                text_input::Style {
                    background: Background::Color(palette.surface_lowest),
                    border: Border {
                        color: border_color,
                        width: 1.0,
                        radius: Radius::from(8.0),
                    },
                    icon: palette.text_muted,
                    placeholder: palette.text_muted,
                    value: palette.text_primary,
                    selection: palette.accent_surface,
                }
            })
            .on_input(move |val| Message::PropertiesNameChanged(win_id, val))
            .on_submit(Message::PropertiesApply {
                window_id: win_id,
                close_after: true,
            })
            .into()
    };

    let hero_box = container(
        row![icon_tile, container(name_control).width(Length::Fill),]
            .spacing(12)
            .align_y(Center),
    )
    .width(Length::Fill)
    .padding(10)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 12.0));

    // Grade de metadados em 2 colunas
    let kind_desc = strings.describe_entry_kind(entry);
    let mime_desc = props
        .telemetry
        .as_ref()
        .and_then(|t| t.mime_type.as_ref())
        .map(|m| m.to_string())
        .unwrap_or_else(|| {
            if is_dir {
                "inode/directory".to_owned()
            } else if let Some(ext) = &entry.extension {
                format!(".{}", ext.to_ascii_lowercase())
            } else {
                "application/octet-stream".to_owned()
            }
        });

    let parent_str =
        sanitize_single_line(&entry.path.parent().unwrap_or(&entry.path).to_string_lossy());

    let (size_short, size_exact, size_ratio): (String, String, f32) = if is_dir {
        if props.size_loading {
            (strings.prop_calculating.to_owned(), String::new(), 0.35)
        } else if let Some(sz) = props.recursive_size {
            let prefix = if sz.truncated { ">" } else { "" };
            let short = format!("{prefix}{}", format_bytes(sz.total_bytes));
            let exact = strings.format_exact_bytes(sz.total_bytes);
            let ratio = props
                .volume_space
                .as_ref()
                .filter(|vs| vs.total_bytes > 0)
                .map(|vs| (sz.total_bytes as f64 / vs.total_bytes as f64).clamp(0.02, 1.0) as f32)
                .unwrap_or_else(|| size_bar_ratio(sz.total_bytes));
            (short, exact, ratio)
        } else {
            ("—".to_owned(), String::new(), 0.0)
        }
    } else {
        (
            format_bytes(entry.size_bytes).to_string(),
            strings.format_exact_bytes(entry.size_bytes),
            size_bar_ratio(entry.size_bytes),
        )
    };

    let col_kind = row![
        icon_file_text().size(16).color(palette.text_muted),
        column![
            text(strings.prop_kind_label)
                .size(11)
                .font(FONT_UI)
                .color(palette.text_muted),
            text(kind_desc)
                .size(13)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Center)
    .width(Length::FillPortion(1));

    let col_system = row![
        icon_sliders_horizontal().size(16).color(palette.text_muted),
        container(
            column![
                text(strings.prop_system_mime_label)
                    .size(11)
                    .font(FONT_UI)
                    .color(palette.text_muted),
                text(sanitize_single_line(&mime_desc))
                    .size(13)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2),
        )
        .width(Length::Fill)
        .clip(true),
        button(icon_external_link().size(13))
            .padding([6, 8])
            .style(styles::pill_secondary_button(palette))
            .on_press_maybe(
                (!is_archive).then(|| Message::PropertiesOpenDefault(entry.path.clone()))
            ),
    ]
    .spacing(10)
    .align_y(Center)
    .width(Length::FillPortion(1));

    let col_path = row![
        icon_folder().size(16).color(palette.text_muted),
        container(
            column![
                text(strings.location_label)
                    .size(11)
                    .font(FONT_UI)
                    .color(palette.text_muted),
                text(parent_str)
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.text_primary)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2),
        )
        .width(Length::Fill)
        .clip(true),
        button(icon_copy().size(13))
            .padding([6, 8])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::CopyPath(entry.path.clone())),
    ]
    .spacing(10)
    .align_y(Center)
    .width(Length::FillPortion(1));

    let filled_units = (size_ratio * 100.0).round() as u16;
    let empty_units = 100u16.saturating_sub(filled_units);
    let mut bar_children: Vec<Element<'a, Message>> = Vec::new();
    if filled_units > 0 {
        bar_children.push(
            container(Space::new().width(Length::Fill).height(6))
                .width(Length::FillPortion(filled_units))
                .style(move |_| container::Style {
                    background: Some(Background::Color(palette.accent_primary)),
                    border: Border {
                        radius: Radius::from(3.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
        );
    }
    if empty_units > 0 {
        bar_children.push(
            Space::new()
                .width(Length::FillPortion(empty_units))
                .height(6)
                .into(),
        );
    }
    let size_bar = container(row(bar_children).height(Length::Fixed(6.0)))
        .width(Length::Fill)
        .height(Length::Fixed(6.0))
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.surface_high)),
            border: Border {
                radius: Radius::from(3.0),
                ..Default::default()
            },
            ..Default::default()
        });

    let col_size = row![
        icon_hard_drive().size(16).color(palette.text_muted),
        column![
            row![
                text(strings.col_size)
                    .size(11)
                    .font(FONT_UI)
                    .color(palette.text_muted),
                size_bar,
            ]
            .spacing(10)
            .align_y(Center),
            row![
                text(size_short)
                    .size(13)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                text(size_exact)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted),
            ]
            .align_y(Center),
        ]
        .spacing(4)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Center)
    .width(Length::FillPortion(1));

    let mut meta_rows = vec![
        row![col_kind, col_system]
            .spacing(20)
            .align_y(Center)
            .into(),
        row![col_path, col_size].spacing(20).align_y(Center).into(),
    ];

    if is_dir {
        let contents_text = if props.size_loading {
            strings.prop_calculating.to_owned()
        } else if let Some(sz) = props.recursive_size {
            let prefix = if sz.truncated { ">" } else { "" };
            let total_txt = strings.format_items_count(sz.item_count as usize);
            if sz.hidden_count > 0 {
                format!(
                    "{prefix}{total_txt} ({})",
                    strings.format_hidden_items_count(sz.hidden_count)
                )
            } else {
                format!("{prefix}{total_txt}")
            }
        } else if let Some(cnt) = entry.child_count {
            strings.format_items_count(cnt as usize)
        } else {
            "—".to_owned()
        };

        let free_text = props
            .volume_space
            .as_ref()
            .map(|vs| {
                format!(
                    "{} {} {} ({})",
                    format_bytes(vs.free_bytes),
                    strings.free_of,
                    format_bytes(vs.total_bytes),
                    sanitize_single_line(&vs.label)
                )
            })
            .unwrap_or_else(|| "—".to_owned());

        let col_contents = row![
            icon_folder().size(16).color(palette.text_muted),
            column![
                text(strings.prop_contents_label)
                    .size(11)
                    .font(FONT_UI)
                    .color(palette.text_muted),
                text(contents_text)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Center)
        .width(Length::FillPortion(1));

        let col_free = row![
            icon_hard_drive().size(16).color(palette.text_muted),
            container(
                column![
                    text(strings.prop_free_space_label)
                        .size(11)
                        .font(FONT_UI)
                        .color(palette.text_muted),
                    text(free_text)
                        .size(12)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary)
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(2),
            )
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(10)
        .align_y(Center)
        .width(Length::FillPortion(1));

        meta_rows.push(
            row![col_contents, col_free]
                .spacing(20)
                .align_y(Center)
                .into(),
        );
    }

    let meta_grid = container(column(meta_rows).spacing(14))
        .width(Length::Fill)
        .padding([4, 6]);

    // Linha do tempo de timestamps (3 nós verticais)
    let created_val = entry
        .created_at
        .map(|t| strings.format_timestamp_short(t))
        .unwrap_or_else(|| "—".to_owned());
    let modified_val = entry
        .modified_at
        .map(|t| strings.format_timestamp_short(t))
        .unwrap_or_else(|| "—".to_owned());
    let accessed_val = props
        .extra
        .as_ref()
        .and_then(|e| e.accessed_at)
        .map(|t| strings.format_timestamp_short(t))
        .unwrap_or_else(|| "—".to_owned());

    let timeline_node =
        |dot_color: Color, label: &'a str, val: String, connect_top: bool, connect_bottom: bool| {
            let top_seg =
                container(Space::new().width(2).height(7)).style(move |_| container::Style {
                    background: if connect_top {
                        Some(Background::Color(palette.border_strong))
                    } else {
                        None
                    },
                    ..Default::default()
                });
            let bottom_seg =
                container(Space::new().width(2).height(7)).style(move |_| container::Style {
                    background: if connect_bottom {
                        Some(Background::Color(palette.border_strong))
                    } else {
                        None
                    },
                    ..Default::default()
                });
            let dot = container(Space::new().width(8).height(8)).style(move |_| container::Style {
                background: Some(Background::Color(dot_color)),
                border: Border {
                    radius: Radius::from(4.0),
                    ..Default::default()
                },
                ..Default::default()
            });
            row![
                column![top_seg, dot, bottom_seg]
                    .width(Length::Fixed(14.0))
                    .align_x(Center),
                text(format!("{label}: "))
                    .size(12)
                    .font(FONT_UI)
                    .color(palette.text_muted),
                text(val)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
            ]
            .spacing(8)
            .align_y(Center)
        };

    let timeline = container(
        column![
            timeline_node(
                palette.accent_primary,
                strings.created,
                created_val,
                false,
                true,
            ),
            timeline_node(
                palette.accent_tertiary,
                strings.modified,
                modified_val,
                true,
                true,
            ),
            timeline_node(
                palette.status_success,
                strings.prop_accessed_label,
                accessed_val,
                true,
                false,
            ),
        ]
        .spacing(0),
    )
    .width(Length::Fill)
    .padding([2, 8]);

    // Propriedades Rápidas + Configurações Avançadas
    let quick_bar = row![
        text(strings.prop_quick_props)
            .size(12)
            .font(FONT_UI)
            .color(palette.text_muted),
        quick_toggle_pill(
            strings.prop_readonly,
            props.readonly_draft,
            !is_archive && !props.applying,
            palette,
            Message::PropertiesToggleReadonly(win_id),
        ),
        quick_toggle_pill(
            strings.prop_hidden,
            props.hidden_draft,
            !is_archive && !props.applying,
            palette,
            Message::PropertiesToggleHidden(win_id),
        ),
        space::horizontal(),
        button(
            row![
                icon_settings().size(13),
                text(strings.prop_advanced_settings).size(12),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([6, 12])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::PropertiesSelectTab(
            win_id,
            PropertiesTab::Permissions
        )),
    ]
    .spacing(10)
    .align_y(Center);

    column![
        hero_box,
        meta_grid,
        container(Space::new().width(Length::Fill).height(1)).style(styles::divider_line(palette)),
        timeline,
        container(Space::new().width(Length::Fill).height(1)).style(styles::divider_line(palette)),
        quick_bar,
    ]
    .spacing(10)
    .into()
}

fn quick_toggle_pill<'a>(
    label: &'a str,
    checked: bool,
    enabled: bool,
    palette: ThemePalette,
    msg: Message,
) -> Element<'a, Message> {
    let track_bg = if checked {
        palette.accent_primary
    } else {
        palette.surface_high
    };
    let thumb_color = if checked {
        palette.on_accent
    } else {
        palette.text_muted
    };
    let left_pad = if checked { 12 } else { 2 };
    let switch_track = container(
        row![
            Space::new().width(Length::Fixed(left_pad as f32)),
            container(Space::new().width(10).height(10)).style(move |_| container::Style {
                background: Some(Background::Color(thumb_color)),
                border: Border {
                    radius: Radius::from(5.0),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ]
        .align_y(Center),
    )
    .width(Length::Fixed(26.0))
    .height(Length::Fixed(16.0))
    .padding(2)
    .style(move |_| container::Style {
        background: Some(Background::Color(track_bg)),
        border: Border {
            color: if checked {
                palette.accent_primary
            } else {
                palette.border_strong
            },
            width: 1.0,
            radius: Radius::from(8.0),
        },
        ..Default::default()
    });

    button(
        row![
            switch_track,
            text(label)
                .size(12)
                .font(if checked { FONT_UI_SEMIBOLD } else { FONT_UI })
                .color(if checked {
                    palette.text_primary
                } else {
                    palette.text_secondary
                }),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([5, 10])
    .style(move |_, status| {
        let bg = if checked {
            palette.accent_surface
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed if enabled => {
                    palette.surface_high
                }
                _ => palette.surface_low,
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: palette.text_primary,
            border: Border {
                color: if checked {
                    palette.accent_border
                } else {
                    palette.border
                },
                width: 1.0,
                radius: Radius::from(9999.0),
            },
            ..Default::default()
        }
    })
    .on_press_maybe(enabled.then_some(msg))
    .into()
}

fn properties_security_tab<'a>(
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
    is_dir: bool,
) -> Element<'a, Message> {
    let entry = &props.entry;
    let full_sha = props
        .telemetry
        .as_ref()
        .and_then(|t| t.sha256_full.as_ref())
        .map(|s| s.to_string());
    let sha_text = full_sha.clone().unwrap_or_else(|| {
        if is_dir {
            strings.prop_sha_na_folders.to_owned()
        } else {
            strings.prop_sha_computing.to_owned()
        }
    });

    let mut sha_card = column![
        row![
            icon_shield_check().size(15).color(palette.accent_primary),
            text("SHA-256")
                .size(13)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary),
        ]
        .spacing(8)
        .align_y(Center),
        container(
            text(sha_text)
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        )
        .width(Length::Fill)
        .padding(10)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 8.0)),
    ]
    .spacing(8);

    if let Some(sha) = full_sha {
        let notice = strings.hash_copied.to_owned();
        sha_card = sha_card.push(
            row![
                space::horizontal(),
                button(
                    row![icon_copy().size(13), text(strings.copy_sha256).size(12)]
                        .spacing(6)
                        .align_y(Center),
                )
                .padding([6, 12])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::CopyText(sha, notice)),
            ]
            .align_y(Center),
        );
    }

    let symlink_row: Option<Element<'a, Message>> = props
        .extra
        .as_ref()
        .and_then(|e| e.symlink_target.as_ref())
        .map(|target| {
            prop_kv_row(
                strings.prop_symlink_target,
                target.to_string_lossy().into_owned(),
                true,
                palette,
            )
        });

    let yes = strings.yes_label;
    let no = strings.no_label;
    let mut flags_col = column![
        prop_kv_row(
            strings.prop_readonly,
            if props.readonly_draft { yes } else { no }.to_owned(),
            false,
            palette,
        ),
        prop_kv_row(
            strings.prop_hidden,
            if props.hidden_draft { yes } else { no }.to_owned(),
            false,
            palette,
        ),
        prop_kv_row(
            strings.permissions,
            entry.permissions.format_compact(is_dir).to_string(),
            true,
            palette,
        ),
    ]
    .spacing(8);

    if let Some(sr) = symlink_row {
        flags_col = flags_col.push(sr);
    }

    column![
        container(sha_card)
            .width(Length::Fill)
            .padding(12)
            .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0)),
        container(flags_col)
            .width(Length::Fill)
            .padding(12)
            .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0)),
    ]
    .spacing(10)
    .into()
}

fn properties_history_tab<'a>(
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let entry = &props.entry;
    let format_ts = |opt: Option<std::time::SystemTime>| -> String {
        opt.map(|t| {
            format!(
                "{} ({})",
                strings.format_timestamp_short(t),
                strings.format_age(t)
            )
        })
        .unwrap_or_else(|| "—".to_owned())
    };
    let git_val = props
        .telemetry
        .as_ref()
        .and_then(|t| t.git_status.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "—".to_owned());

    container(
        column![
            prop_kv_row(strings.created, format_ts(entry.created_at), false, palette),
            prop_kv_row(
                strings.modified,
                format_ts(entry.modified_at),
                false,
                palette,
            ),
            prop_kv_row(
                strings.prop_accessed_label,
                format_ts(props.extra.as_ref().and_then(|e| e.accessed_at)),
                false,
                palette,
            ),
            prop_kv_row("Git", git_val, true, palette),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
    .into()
}

fn properties_metadata_tab<'a>(
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
    is_dir: bool,
) -> Element<'a, Message> {
    let entry = &props.entry;
    let ext = entry
        .extension
        .as_ref()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let mime = props
        .telemetry
        .as_ref()
        .and_then(|t| t.mime_type.as_ref())
        .map(|m| m.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let size_line = if is_dir {
        props
            .recursive_size
            .map(|sz| {
                format!(
                    "{}{} {}",
                    if sz.truncated { ">" } else { "" },
                    format_bytes(sz.total_bytes),
                    strings.format_exact_bytes(sz.total_bytes)
                )
            })
            .unwrap_or_else(|| "—".to_owned())
    } else {
        format!(
            "{} {}",
            format_bytes(entry.size_bytes),
            strings.format_exact_bytes(entry.size_bytes)
        )
    };

    container(
        column![
            prop_kv_row(strings.name_label, entry.name.to_string(), false, palette),
            row![
                prop_kv_row(
                    strings.location_label,
                    entry.path.to_string_lossy().into_owned(),
                    true,
                    palette,
                ),
                button(icon_copy().size(13))
                    .padding([5, 7])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::CopyPath(entry.path.clone())),
            ]
            .spacing(8)
            .align_y(Center),
            prop_kv_row(strings.ext_label, ext, true, palette),
            prop_kv_row("MIME", mime, true, palette),
            prop_kv_row(strings.col_size, size_line, true, palette),
            prop_kv_row("EntryId", format!("0x{:016X}", entry.id.0), true, palette),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
    .into()
}

fn properties_permissions_tab<'a>(
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let is_archive = props.is_archive();
    let is_dir = props.entry.kind.is_dir_like();
    let editable = !is_archive && !props.applying;

    if let Some(mode) = props.posix_mode_draft {
        let octal_label = format!("0o{mode:03o}");
        let symbolic = PermissionsMeta::PosixMode(mode)
            .format_compact(is_dir)
            .to_string();

        let bit_btn = |title: &'static str, bit: u32| {
            let active = (mode & bit) != 0;
            button(
                row![
                    container(if active {
                        Element::from(icon_check().size(11).color(palette.on_accent))
                    } else {
                        Space::new().width(11).height(11).into()
                    })
                    .width(Length::Fixed(18.0))
                    .height(Length::Fixed(18.0))
                    .align_x(Center)
                    .align_y(Center)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(if active {
                            palette.accent_primary
                        } else {
                            palette.surface_lowest
                        })),
                        border: Border {
                            color: if active {
                                palette.accent_primary
                            } else {
                                palette.border_strong
                            },
                            width: 1.0,
                            radius: Radius::from(4.0),
                        },
                        ..Default::default()
                    }),
                    text(title)
                        .size(12)
                        .font(FONT_UI)
                        .color(palette.text_primary),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([5, 10])
            .style(styles::ghost_button(palette, 6.0))
            .on_press_maybe(editable.then_some(Message::PropertiesTogglePosixBit(win_id, bit)))
        };

        let r_lbl = strings.perm_read;
        let w_lbl = strings.perm_write;
        let x_lbl = strings.perm_execute;

        let perm_group = |title: &'a str, r: u32, w: u32, x: u32| {
            row![
                text(title)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_secondary)
                    .width(Length::Fixed(110.0)),
                bit_btn(r_lbl, r),
                bit_btn(w_lbl, w),
                bit_btn(x_lbl, x),
            ]
            .spacing(10)
            .align_y(Center)
        };

        return column![
            container(
                row![
                    icon_lock().size(15).color(palette.accent_primary),
                    text(strings.prop_octal_mode)
                        .size(12)
                        .font(FONT_UI)
                        .color(palette.text_muted),
                    text(octal_label)
                        .size(13)
                        .font(FONT_MONO_BOLD)
                        .color(palette.text_primary),
                    space::horizontal(),
                    text(symbolic)
                        .size(13)
                        .font(FONT_MONO)
                        .color(palette.accent_primary),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .width(Length::Fill)
            .padding(12)
            .style(move |_| styles::panel_surface(
                palette.surface_low,
                palette.border,
                10.0
            )),
            container(
                column![
                    perm_group(strings.prop_owner, 0o400, 0o200, 0o100),
                    perm_group(strings.prop_group, 0o040, 0o020, 0o010),
                    perm_group(strings.prop_others, 0o004, 0o002, 0o001),
                ]
                .spacing(8),
            )
            .width(Length::Fill)
            .padding(12)
            .style(move |_| styles::panel_surface(
                palette.surface_low,
                palette.border,
                10.0
            )),
        ]
        .spacing(10)
        .into();
    }

    // Windows / fallback attributes
    let attrs_hex = match props.entry.permissions {
        PermissionsMeta::WinAttributes(a) => format!("0x{a:08X}"),
        PermissionsMeta::PosixMode(m) => format!("0o{m:04o}"),
    };
    container(
        column![
            prop_kv_row(
                strings.permissions,
                props.entry.permissions.format_compact(is_dir).to_string(),
                true,
                palette,
            ),
            prop_kv_row("Attributes", attrs_hex, true, palette),
            row![
                quick_toggle_pill(
                    strings.prop_readonly,
                    props.readonly_draft,
                    editable,
                    palette,
                    Message::PropertiesToggleReadonly(win_id),
                ),
                quick_toggle_pill(
                    strings.prop_hidden,
                    props.hidden_draft,
                    editable,
                    palette,
                    Message::PropertiesToggleHidden(win_id),
                ),
            ]
            .spacing(10),
        ]
        .spacing(12),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
    .into()
}

fn prop_kv_row<'a>(
    label: &'a str,
    val: String,
    mono: bool,
    palette: ThemePalette,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.text_muted)
            .width(Length::Fixed(130.0)),
        container(
            text(sanitize_single_line(&val))
                .size(12)
                .font(if mono { FONT_MONO } else { FONT_UI })
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None),
        )
        .width(Length::Fill)
        .clip(true),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

fn size_bar_ratio(bytes: u64) -> f32 {
    if bytes == 0 {
        return 0.04;
    }
    let log_val = (bytes as f64).log10();
    ((log_val / 11.0) as f32).clamp(0.10, 0.96)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LanguageOption {
    language: Option<Language>,
    label: String,
}

impl std::fmt::Display for LanguageOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub fn preferences_modal<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let theme_name = match app.config.theme {
        ThemePreset::AetherLight => "AetherLight",
        ThemePreset::ObsidianFlow => "ObsidianFlow",
    };
    let deco_name = match app.config.window_decorations {
        WindowDecorationsMode::NativeWm => strings.window_bar_native,
        WindowDecorationsMode::IntegratedCsd => strings.window_bar_csd,
    };

    let mut lang_options = Vec::with_capacity(Language::ALL.len() + 1);
    lang_options.push(LanguageOption {
        language: None,
        label: strings.follow_system.to_owned(),
    });
    for lang in Language::ALL {
        lang_options.push(LanguageOption {
            language: Some(lang),
            label: lang.native_name().to_owned(),
        });
    }
    let selected_lang = if app.config.language_customized {
        LanguageOption {
            language: Some(app.config.language),
            label: app.config.language.native_name().to_owned(),
        }
    } else {
        LanguageOption {
            language: None,
            label: strings.follow_system.to_owned(),
        }
    };
    let lang_controls: Element<'a, Message> = pick_list(lang_options, Some(selected_lang), |opt| {
        match opt.language {
            Some(lang) => Message::SetLanguage(lang),
            None => Message::UseSystemLanguage,
        }
    })
    .text_size(12)
    .font(FONT_UI)
    .padding([6, 10])
    .width(Length::Fixed(220.0))
    .style(styles::pick_list_style(palette))
    .menu_style(styles::dropdown_menu_style(palette))
    .into();

    let pref_row = |label: &'a str, control: Element<'a, Message>| {
        container(
            row![
                text(label)
                    .size(13)
                    .font(FONT_UI)
                    .color(palette.text_primary),
                space::horizontal(),
                control,
            ]
            .align_y(Center),
        )
        .width(Length::Fill)
        .padding([10, 12])
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0))
    };

    let view_pills = row![
        button(text(strings.view_grid).size(11).font(FONT_MONO))
            .padding([4, 10])
            .style(styles::tab_button(
                palette,
                app.config.default_view_mode == ViewMode::Grid
            ))
            .on_press(Message::SetDefaultViewMode(ViewMode::Grid)),
        button(text(strings.view_list).size(11).font(FONT_MONO))
            .padding([4, 10])
            .style(styles::tab_button(
                palette,
                app.config.default_view_mode == ViewMode::DetailTable
            ))
            .on_press(Message::SetDefaultViewMode(ViewMode::DetailTable)),
        button(text(strings.view_columns).size(11).font(FONT_MONO))
            .padding([4, 10])
            .style(styles::tab_button(
                palette,
                app.config.default_view_mode == ViewMode::Columns
            ))
            .on_press(Message::SetDefaultViewMode(ViewMode::Columns)),
    ]
    .spacing(4);

    let cfg_path_hint = fm_fs::config_file_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let prefs_tabs = column![
        row![
            modal_tab_button(
                strings.prefs_tab_general,
                app.prefs_tab == PreferencesTab::General,
                palette,
                Message::SelectPreferencesTab(PreferencesTab::General),
            ),
            modal_tab_button(
                strings.about_tab,
                app.prefs_tab == PreferencesTab::About,
                palette,
                Message::SelectPreferencesTab(PreferencesTab::About),
            ),
        ]
        .spacing(4),
        container(Space::new().width(Length::Fill).height(1)).style(move |_| container::Style {
            background: Some(Background::Color(palette.border)),
            ..Default::default()
        }),
    ]
    .spacing(0);

    let tab_body: Element<'a, Message> = match app.prefs_tab {
        PreferencesTab::General => column![
            pref_row(
                strings.theme_label,
                button(text(theme_name).size(12).font(FONT_MONO_BOLD))
                    .padding([5, 12])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::ToggleTheme)
                    .into(),
            ),
            pref_row(strings.lang_label, lang_controls),
            pref_row(
                strings.hero_card_label,
                button(
                    text(if app.config.show_hero_card {
                        "ON"
                    } else {
                        "OFF"
                    })
                    .size(12)
                    .font(FONT_MONO_BOLD),
                )
                .padding([5, 12])
                .style(styles::tab_button(palette, app.config.show_hero_card))
                .on_press(Message::ToggleHeroCard)
                .into(),
            ),
            pref_row(strings.default_view_label, view_pills.into()),
            pref_row(
                strings.grid_group_label,
                button(
                    text(if app.config.grid_group_folders {
                        "ON"
                    } else {
                        "OFF"
                    })
                    .size(12)
                    .font(FONT_MONO_BOLD),
                )
                .padding([5, 12])
                .style(styles::tab_button(palette, app.config.grid_group_folders))
                .on_press(Message::ToggleGridGroupFolders)
                .into(),
            ),
            pref_row(
                strings.dirs_first_label,
                button(
                    text(if app.config.dirs_first { "ON" } else { "OFF" })
                        .size(12)
                        .font(FONT_MONO_BOLD),
                )
                .padding([5, 12])
                .style(styles::tab_button(palette, app.config.dirs_first))
                .on_press(Message::ToggleDirsFirst)
                .into(),
            ),
            pref_row(
                strings.decorations_label,
                button(text(deco_name).size(12).font(FONT_MONO_BOLD))
                    .padding([5, 12])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::ToggleWindowDecorations)
                    .into(),
            ),
            Space::new().height(2),
            text(cfg_path_hint)
                .size(10)
                .font(FONT_MONO)
                .color(palette.text_muted),
        ]
        .spacing(10)
        .into(),
        PreferencesTab::About => about_tab_view(palette, strings),
    };

    let card = container(
        column![
            row![
                text(strings.prefs_title)
                    .size(15)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                button(
                    row![icon_x().size(14), text(strings.close).size(12)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([4, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::ClosePreferences),
            ]
            .align_y(Center),
            prefs_tabs,
            tab_body,
        ]
        .spacing(12)
        .width(Length::Fixed(520.0)),
    )
    .padding(20)
    .style(styles::modal_card(palette));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::modal_backdrop()(&iced::Theme::Light))
        .into()
}

/// Renders an underline-accented modal tab button shared between Preferences and Properties.
fn modal_tab_button<'a>(
    label: &'a str,
    active: bool,
    palette: ThemePalette,
    msg: Message,
) -> Element<'a, Message> {
    let underline_color = if active {
        palette.accent_primary
    } else {
        Color::TRANSPARENT
    };
    let label_color = if active {
        palette.text_primary
    } else {
        palette.text_muted
    };
    button(
        column![
            container(
                text(label)
                    .size(12)
                    .font(if active { FONT_UI_SEMIBOLD } else { FONT_UI })
                    .color(label_color)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .align_x(Center)
            .padding([6, 4]),
            container(Space::new().width(Length::Fill).height(2)).style(move |_| {
                container::Style {
                    background: Some(Background::Color(underline_color)),
                    ..Default::default()
                }
            }),
        ]
        .spacing(2),
    )
    .width(Length::Fill)
    .padding(0)
    .style(move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed if !active => {
                Some(Background::Color(palette.surface_low))
            }
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: label_color,
            border: Border::default(),
            ..Default::default()
        }
    })
    .on_press(msg)
    .into()
}

/// Renders the About tab inside the Preferences modal with version, developer status, and GPLv3 details.
fn about_tab_view<'a>(palette: ThemePalette, strings: Strings) -> Element<'a, Message> {
    let version_tag = concat!("v", env!("CARGO_PKG_VERSION"));
    let copy_payload = format!(
        "Vagnarok File Manager {version_tag} ({} / {}) · GNU GPL v3.0 (GPL-3.0-or-later)",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let copied_notice = strings.about_info_copied.to_owned();

    let hero = container(
        row![
            container(icon_hard_drive().size(24).color(palette.accent_primary))
                .width(48)
                .height(48)
                .align_x(Center)
                .align_y(Center)
                .style(move |_| styles::panel_surface(
                    palette.accent_surface,
                    palette.accent_border,
                    10.0
                )),
            column![
                row![
                    text("Vagnarok File Manager")
                        .size(16)
                        .font(FONT_UI_BOLD)
                        .color(palette.text_primary),
                    container(
                        text(version_tag)
                            .size(11)
                            .font(FONT_MONO_BOLD)
                            .color(palette.accent_primary),
                    )
                    .padding([2, 8])
                    .style(styles::badge_container(
                        palette.accent_surface,
                        palette.accent_border,
                        palette.accent_primary,
                        9999.0,
                    )),
                ]
                .spacing(8)
                .align_y(Center),
                text(strings.about_subtitle)
                    .size(12)
                    .font(FONT_UI)
                    .color(palette.text_secondary),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Center),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0));

    let meta_card = container(
        column![
            prop_kv_row(
                strings.about_version_label,
                version_tag.to_owned(),
                true,
                palette
            ),
            prop_kv_row(
                strings.about_stage_label,
                strings.about_stage_alpha.to_owned(),
                false,
                palette,
            ),
            prop_kv_row(
                strings.about_developer_label,
                strings.about_developer_value.to_owned(),
                false,
                palette,
            ),
            prop_kv_row(
                strings.about_license_label,
                "GNU GPL v3.0 (GPL-3.0-or-later)".to_owned(),
                true,
                palette,
            ),
            prop_kv_row(
                strings.about_stack_label,
                "Rust · Iced 0.14 + wgpu".to_owned(),
                true,
                palette,
            ),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0));

    let actions = row![
        space::horizontal(),
        button(
            row![icon_copy().size(13), text(strings.about_copy_info).size(12)]
                .spacing(6)
                .align_y(Center),
        )
        .padding([6, 14])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::CopyText(copy_payload, copied_notice)),
    ]
    .align_y(Center);

    column![hero, meta_card, actions]
        .spacing(12)
        .width(Length::Fill)
        .into()
}

pub fn format_transfer_summary(done: u64, total: u64) -> String {
    if total == 0 {
        format_bytes(done).to_string()
    } else {
        let pct = ((done as f64 / total as f64) * 100.0).min(100.0);
        format!(
            "{} / {} ({pct:.0}%)",
            format_bytes(done),
            format_bytes(total)
        )
    }
}

pub fn breadcrumb_menu_overlay<'a>(
    root: &'a Path,
    folders: &'a [FileEntry],
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mut items_col = column![].spacing(2).width(Length::Fill);

    if folders.is_empty() {
        items_col = items_col.push(
            container(
                text(strings.empty_subdir)
                    .size(12)
                    .font(FONT_UI)
                    .color(palette.text_muted),
            )
            .padding([6, 8]),
        );
    } else {
        for entry in folders.iter().take(40) {
            let p = entry.path.clone();
            items_col = items_col.push(
                button(
                    row![
                        icon_folder().size(14).color(palette.accent_primary),
                        text(sanitize_single_line(entry.name.as_str()))
                            .size(12)
                            .font(FONT_UI)
                            .color(palette.text_primary)
                            .wrapping(text::Wrapping::None),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
                .width(Length::Fill)
                .padding([5, 8])
                .style(styles::ghost_button(palette, 6.0))
                .on_press(Message::NavigateTo(p)),
            );
        }
    }

    let menu_card = container(
        column![
            row![
                text(sanitize_single_line(
                    &root
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| root.to_string_lossy().into_owned()),
                ))
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted),
                space::horizontal(),
                button(icon_x().size(12))
                    .padding([2, 4])
                    .style(styles::ghost_button(palette, 4.0))
                    .on_press(Message::CloseBreadcrumbMenu),
            ]
            .align_y(Center),
            scrollable(items_col).height(Length::Shrink),
        ]
        .spacing(6)
        .width(Length::Fixed(240.0)),
    )
    .max_height(320.0)
    .padding(10)
    .style(styles::modal_card(palette));

    mouse_area(
        container(menu_card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: 82.0,
                right: 16.0,
                bottom: 16.0,
                left: 160.0,
            }),
    )
    .on_press(Message::CloseBreadcrumbMenu)
    .into()
}

pub fn context_menu_overlay<'a>(
    app: &'a App,
    menu: &'a ContextMenuState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let menu_width = 230.0_f32;
    let has_clipboard = app.clipboard.is_some();

    let mut items = column![].spacing(2).width(Length::Fill);

    match &menu.kind {
        ContextMenuKind::NewMenu => {
            items = items
                .push(ctx_item(
                    icon_folder_plus()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.new_folder,
                    palette.text_primary,
                    Message::OpenCreateDialog(true),
                    palette,
                ))
                .push(ctx_item(
                    icon_file_plus()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.new_file,
                    palette.text_primary,
                    Message::OpenCreateDialog(false),
                    palette,
                ));
        }
        ContextMenuKind::Entry {
            index,
            path,
            is_dir,
        } => {
            items = items.push(ctx_item(
                icon_external_link()
                    .size(14)
                    .color(palette.accent_primary)
                    .into(),
                strings.open_item,
                palette.text_primary,
                Message::ActivateEntry(*index),
                palette,
            ));
            if *is_dir {
                items = items.push(ctx_item(
                    icon_folder().size(14).color(palette.accent_primary).into(),
                    strings.open_in_new_tab,
                    palette.text_primary,
                    Message::OpenInNewTab(path.clone()),
                    palette,
                ));
            } else {
                items = items.push(ctx_item(
                    icon_eye().size(14).color(palette.text_secondary).into(),
                    strings.quicklook_title,
                    palette.text_primary,
                    Message::ToggleQuickLook,
                    palette,
                ));
            }
            if app
                .places
                .trash
                .as_ref()
                .is_some_and(|tr| path.starts_with(tr))
            {
                items = items.push(ctx_item(
                    icon_rotate_ccw()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.restore_item,
                    palette.accent_primary,
                    Message::RestoreFromTrash,
                    palette,
                ));
            }
            items = items
                .push(ctx_divider(palette))
                .push(ctx_item(
                    icon_copy().size(14).color(palette.text_secondary).into(),
                    strings.copy,
                    palette.text_primary,
                    Message::CopySelected,
                    palette,
                ))
                .push(ctx_item(
                    icon_scissors()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.cut,
                    palette.text_primary,
                    Message::CutSelected,
                    palette,
                ));
            if has_clipboard {
                items = items.push(ctx_item(
                    icon_clipboard_paste()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.paste,
                    palette.text_primary,
                    Message::PasteClipboard,
                    palette,
                ));
            }
            items = items
                .push(ctx_item(
                    icon_clipboard_copy()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.copy_path,
                    palette.text_primary,
                    Message::CopyPath(path.clone()),
                    palette,
                ))
                .push(ctx_item(
                    icon_pencil().size(14).color(palette.text_secondary).into(),
                    strings.rename,
                    palette.text_primary,
                    Message::OpenRenameDialog,
                    palette,
                ))
                .push(ctx_item(
                    icon_info().size(14).color(palette.text_secondary).into(),
                    strings.properties,
                    palette.text_primary,
                    Message::OpenProperties(path.clone()),
                    palette,
                ))
                .push(ctx_divider(palette))
                .push(ctx_item(
                    icon_trash_2().size(14).color(palette.status_error).into(),
                    strings.move_to_trash,
                    palette.status_error,
                    Message::RequestDelete(false),
                    palette,
                ));
        }
        ContextMenuKind::SidebarPlace { path, is_favorite } => {
            items = items
                .push(ctx_item(
                    icon_external_link()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.open_item,
                    palette.text_primary,
                    Message::NavigateTo(path.clone()),
                    palette,
                ))
                .push(ctx_item(
                    icon_folder().size(14).color(palette.accent_primary).into(),
                    strings.open_in_new_tab,
                    palette.text_primary,
                    Message::OpenInNewTab(path.clone()),
                    palette,
                ))
                .push(ctx_item(
                    icon_clipboard_copy()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.copy_path,
                    palette.text_primary,
                    Message::CopyPath(path.clone()),
                    palette,
                ))
                .push(ctx_item(
                    icon_info().size(14).color(palette.text_secondary).into(),
                    strings.properties,
                    palette.text_primary,
                    Message::OpenProperties(path.clone()),
                    palette,
                ));
            if *is_favorite {
                items = items.push(ctx_divider(palette)).push(ctx_item(
                    icon_star_off().size(14).color(palette.status_error).into(),
                    strings.remove_favorite,
                    palette.status_error,
                    Message::RemoveFavorite(path.clone()),
                    palette,
                ));
            }
        }
        ContextMenuKind::FilterMenu => {
            if let Some(tab) = app.current_tab() {
                let active_ext = tab.extension_filter.as_deref();
                items = items.push(ctx_item_owned(
                    icon_filter().size(14).color(palette.accent_primary).into(),
                    strings.filter_all.to_owned(),
                    if active_ext.is_none() {
                        palette.accent_primary
                    } else {
                        palette.text_primary
                    },
                    Message::SetExtensionFilter(None),
                    palette,
                ));
                items = items.push(ctx_item_owned(
                    icon_folder().size(14).color(palette.accent_primary).into(),
                    strings.directories_only.to_owned(),
                    if active_ext == Some(DIR_FILTER_SENTINEL) {
                        palette.accent_primary
                    } else {
                        palette.text_primary
                    },
                    Message::SetExtensionFilter(Some(DIR_FILTER_SENTINEL.into())),
                    palette,
                ));
                let exts = tab.available_extensions();
                if !exts.is_empty() {
                    items = items.push(ctx_divider(palette));
                    for ext in exts.into_iter().take(16) {
                        let is_selected = active_ext == Some(ext.as_str());
                        let label = format!(".{ext}");
                        items = items.push(ctx_item_owned(
                            Space::new().width(14).height(14).into(),
                            label,
                            if is_selected {
                                palette.accent_primary
                            } else {
                                palette.text_primary
                            },
                            Message::SetExtensionFilter(Some(ext)),
                            palette,
                        ));
                    }
                }
            }
        }
        ContextMenuKind::LanguageMenu => {
            let auto_active = !app.config.language_customized;
            items = items
                .push(ctx_item(
                    if auto_active {
                        icon_check().size(14).color(palette.accent_primary).into()
                    } else {
                        Space::new().width(14).height(14).into()
                    },
                    strings.follow_system,
                    if auto_active {
                        palette.accent_primary
                    } else {
                        palette.text_primary
                    },
                    Message::UseSystemLanguage,
                    palette,
                ))
                .push(ctx_divider(palette));
            for lang in Language::ALL {
                let active = app.config.language_customized && app.config.language == lang;
                items = items.push(ctx_item(
                    if active {
                        icon_check().size(14).color(palette.accent_primary).into()
                    } else {
                        Space::new().width(14).height(14).into()
                    },
                    lang.native_name(),
                    if active {
                        palette.accent_primary
                    } else {
                        palette.text_primary
                    },
                    Message::SetLanguage(lang),
                    palette,
                ));
            }
        }
        ContextMenuKind::PreferencesMenu => {
            items = items
                .push(ctx_item(
                    icon_settings()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.preferences,
                    palette.text_primary,
                    Message::OpenPreferences,
                    palette,
                ))
                .push(ctx_item(
                    icon_info().size(14).color(palette.text_secondary).into(),
                    strings.about_tab,
                    palette.text_primary,
                    Message::OpenAbout,
                    palette,
                ));
        }
        ContextMenuKind::Background => {
            items = items
                .push(ctx_item(
                    icon_folder_plus()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.new_folder,
                    palette.text_primary,
                    Message::OpenCreateDialog(true),
                    palette,
                ))
                .push(ctx_item(
                    icon_file_plus()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.new_file,
                    palette.text_primary,
                    Message::OpenCreateDialog(false),
                    palette,
                ));
            if has_clipboard {
                items = items.push(ctx_item(
                    icon_clipboard_paste()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.paste,
                    palette.text_primary,
                    Message::PasteClipboard,
                    palette,
                ));
            }
            items = items.push(ctx_divider(palette)).push(ctx_item(
                icon_check_square()
                    .size(14)
                    .color(palette.text_secondary)
                    .into(),
                strings.select_all,
                palette.text_primary,
                Message::SelectAll,
                palette,
            ));
            if let Some(tab) = app.current_tab() {
                items = items.push(ctx_item(
                    icon_clipboard_copy()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.copy_path,
                    palette.text_primary,
                    Message::CopyPath(tab.current_path.clone()),
                    palette,
                ));
            }
            items = items.push(ctx_item(
                icon_terminal()
                    .size(14)
                    .color(palette.text_secondary)
                    .into(),
                strings.open_terminal,
                palette.text_primary,
                Message::OpenTerminal,
                palette,
            ));
            if let Some(tab) = app.current_tab() {
                items = items.push(ctx_divider(palette)).push(ctx_item(
                    icon_info().size(14).color(palette.accent_primary).into(),
                    strings.properties,
                    palette.text_primary,
                    Message::OpenProperties(tab.current_path.clone()),
                    palette,
                ));
            }
        }
    }

    let est_height = match &menu.kind {
        ContextMenuKind::NewMenu => 84.0,
        ContextMenuKind::Entry { .. } => 340.0,
        ContextMenuKind::SidebarPlace { is_favorite, .. } => {
            if *is_favorite {
                196.0
            } else {
                156.0
            }
        }
        ContextMenuKind::Background => 284.0,
        ContextMenuKind::FilterMenu => 320.0,
        ContextMenuKind::LanguageMenu => 256.0,
        ContextMenuKind::PreferencesMenu => 84.0,
    };
    let max_x = (app.window_size.width - menu_width - 12.0).max(8.0);
    let max_y = (app.window_size.height - est_height - 12.0).max(8.0);
    let x = menu.position.x.clamp(8.0, max_x);
    let y = menu.position.y.clamp(8.0, max_y);

    let card_body: Element<'a, Message> = if matches!(
        menu.kind,
        ContextMenuKind::LanguageMenu | ContextMenuKind::FilterMenu
    ) {
        scrollable(items.width(Length::Fixed(menu_width)))
            .height(Length::Shrink)
            .into()
    } else {
        items.width(Length::Fixed(menu_width)).into()
    };

    let card = container(card_body)
        .max_height(320.0)
        .padding(8)
        .style(styles::modal_card(palette));

    let backdrop = mouse_area(
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: y,
                right: 0.0,
                bottom: 0.0,
                left: x,
            }),
    )
    .on_press(Message::CloseContextMenu)
    .on_right_press(Message::CloseContextMenu);

    backdrop.into()
}

fn ctx_item<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    label_color: iced::Color,
    msg: Message,
    palette: ThemePalette,
) -> Element<'a, Message> {
    button(
        row![icon, text(label).size(12).font(FONT_UI).color(label_color)]
            .spacing(8)
            .align_y(Center),
    )
    .width(Length::Fill)
    .padding([6, 10])
    .style(styles::ghost_button(palette, 6.0))
    .on_press(msg)
    .into()
}

fn ctx_item_owned<'a>(
    icon: Element<'a, Message>,
    label: String,
    label_color: iced::Color,
    msg: Message,
    palette: ThemePalette,
) -> Element<'a, Message> {
    button(
        row![icon, text(label).size(12).font(FONT_UI).color(label_color)]
            .spacing(8)
            .align_y(Center),
    )
    .width(Length::Fill)
    .padding([6, 10])
    .style(styles::ghost_button(palette, 6.0))
    .on_press(msg)
    .into()
}

fn ctx_divider<'a>(palette: ThemePalette) -> Element<'a, Message> {
    container(
        container(Space::new().width(Length::Fill).height(1)).style(styles::divider_line(palette)),
    )
    .padding([3, 4])
    .into()
}
