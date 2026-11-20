use std::path::Path;

use fm_core::{
    ConflictPolicy, DIR_FILTER_SENTINEL, FileEntry, Language, OpenWithApp, PermissionsMeta,
    Strings, ThemePreset, ViewMode, WindowDecorationsMode, format_bytes, is_posix_executable,
    toggle_posix_executable_mode,
};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles,
};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, image, mouse_area, opaque, pick_list, row, scrollable, space,
    stack, text, text_input, tooltip,
};
use iced::{Background, Border, Color, Element, Length, Padding, border::Radius, mouse, window};
use lucide_icons::iced::{
    icon_arrow_left, icon_arrow_up_from_line, icon_check, icon_check_square, icon_chevron_right,
    icon_clipboard_copy, icon_clipboard_paste, icon_copy, icon_disc_3, icon_external_link,
    icon_eye, icon_file_archive, icon_file_plus, icon_file_text, icon_filter, icon_folder,
    icon_folder_plus, icon_hard_drive, icon_info, icon_link_2, icon_lock, icon_monitor,
    icon_pencil, icon_plug, icon_power, icon_rotate_ccw, icon_scissors, icon_search, icon_send,
    icon_settings, icon_shield_alert, icon_shield_check, icon_sliders_horizontal, icon_star_off,
    icon_terminal, icon_trash_2, icon_x,
};

use crate::app::{
    ActiveDialog, App, ContextMenuKind, ContextMenuState, Message, PreferencesTab, PropertiesTab,
    PropertiesWindowState, UndoAction, dialog_input_id, effective_properties_modal_rect,
    filter_ext_input_id, open_with_props_type_key, resolve_open_in_new_tab_targets,
};
use crate::archive_jobs::ArchiveUiMsg;
use crate::duplicate_finder::DuplicateFinderMsg;
use crate::views::{
    entry_icon, modal_overlay_backdrop, modal_overlay_backdrop_at, sanitize_single_line,
    with_tooltip,
};

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

    modal_overlay_backdrop(card)
}

pub fn properties_modal_overlay<'a>(
    app: &'a App,
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (x, y, modal_w, modal_h) = effective_properties_modal_rect(app);
    let card = container(properties_window_view(app, win_id, props, palette, strings))
        .width(Length::Fixed(modal_w))
        .height(Length::Fixed(modal_h))
        .clip(true)
        .style(styles::modal_card(palette));

    modal_overlay_backdrop_at(card, x, y)
}

pub fn properties_window_view<'a>(
    app: &'a App,
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let entry = &props.entry;
    let is_dir = entry.kind.is_dir_like();
    let is_multi = props.is_multi();
    let is_archive = props.is_archive();
    let dirty = props.is_dirty();

    // 1. Cabeçalho arrastável da janela flutuante
    let header_title = if let Some(multi) = &props.multi {
        format!(
            "{}{}",
            strings.properties_header_prefix,
            strings.format_items_count(multi.paths.len())
        )
    } else {
        format!(
            "{}{}",
            strings.properties_header_prefix,
            sanitize_single_line(entry.name.as_str())
        )
    };
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
    .interaction(if app.dragging_properties_modal.is_some() {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    })
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

    let mut tabs_row = row![tab_item(PropertiesTab::General, strings.prop_tab_general)]
        .spacing(4)
        .width(Length::Fill);

    if !is_multi {
        tabs_row = tabs_row
            .push(tab_item(PropertiesTab::Security, strings.prop_tab_security))
            .push(tab_item(PropertiesTab::History, strings.prop_tab_history))
            .push(tab_item(PropertiesTab::Metadata, strings.prop_tab_metadata))
            .push(tab_item(
                PropertiesTab::Permissions,
                strings.prop_tab_permissions,
            ));
    }

    if props.supports_open_with() {
        tabs_row = tabs_row.push(tab_item(
            PropertiesTab::OpenWith,
            strings.prop_tab_open_with,
        ));
    }

    let tab_bar = column![
        tabs_row,
        container(Space::new().width(Length::Fill).height(1)).style(styles::divider_line(palette)),
    ];

    // 3. Conteúdo da aba ativa
    let tab_body: Element<'a, Message> = match props.active_tab {
        PropertiesTab::General => properties_general_tab(win_id, props, palette, strings, is_dir),
        PropertiesTab::Security => properties_security_tab(props, palette, strings, is_dir),
        PropertiesTab::History => properties_history_tab(props, palette, strings),
        PropertiesTab::Metadata => properties_metadata_tab(props, palette, strings, is_dir),
        PropertiesTab::Permissions => properties_permissions_tab(win_id, props, palette, strings),
        PropertiesTab::OpenWith => properties_open_with_tab(app, win_id, props, palette, strings),
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

    let resize_grip = mouse_area(
        container(
            column![
                row![Space::new().width(6).height(2), grip_dot(palette),].spacing(2),
                row![
                    Space::new().width(2).height(2),
                    grip_dot(palette),
                    grip_dot(palette),
                ]
                .spacing(2),
                row![grip_dot(palette), grip_dot(palette), grip_dot(palette),].spacing(2),
            ]
            .spacing(2),
        )
        .padding([4, 4]),
    )
    .interaction(mouse::Interaction::ResizingDiagonallyDown)
    .on_press(Message::StartPropertiesResize);

    let footer_buttons = if is_multi {
        row![confirm_btn].spacing(8)
    } else {
        row![confirm_btn, cancel_btn, apply_btn].spacing(8)
    };

    let footer = row![
        status_hint,
        space::horizontal(),
        footer_buttons,
        resize_grip,
    ]
    .spacing(6)
    .align_y(Center);

    let body_slot: Element<'a, Message> = if props.active_tab == PropertiesTab::OpenWith {
        container(tab_body)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(scrollable(tab_body).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    container(
        column![
            header,
            tab_bar,
            body_slot,
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
    .into()
}

fn grip_dot<'a>(palette: ThemePalette) -> Element<'a, Message> {
    container(Space::new().width(2).height(2))
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.text_muted)),
            border: Border {
                radius: Radius::from(1.0),
                ..Default::default()
            },
            ..Default::default()
        })
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
    let multi = props.multi.as_ref();
    let is_multi = multi.is_some();
    let is_archive = props.is_archive();
    let hero_icon: Element<'a, Message> = if let Some(m) = multi {
        if m.file_count == 0 {
            icon_folder().size(28).color(palette.accent_primary).into()
        } else if m.dir_count == 0 {
            icon_file_text()
                .size(28)
                .color(palette.accent_primary)
                .into()
        } else {
            icon_check_square()
                .size(26)
                .color(palette.accent_primary)
                .into()
        }
    } else {
        entry_icon(entry, palette, 28.0).0.into()
    };

    // Caixa Hero (ícone 52x52 + nome / resumo de seleção)
    let icon_tile = container(hero_icon)
        .width(Length::Fixed(52.0))
        .height(Length::Fixed(52.0))
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::panel_surface(palette.accent_surface, palette.accent_border, 10.0));

    let name_control: Element<'a, Message> = if let Some(m) = multi {
        text(strings.format_items_count(m.paths.len()))
            .size(16)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.text_primary)
            .wrapping(text::Wrapping::None)
            .into()
    } else if is_archive {
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
                    selection: styles::text_selection_color(palette),
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
    let kind_desc = if let Some(m) = multi {
        if m.dir_count > 0 && m.file_count > 0 {
            format!(
                "{}, {}",
                strings.format_folders_count(m.dir_count),
                strings.format_files_count(m.file_count)
            )
        } else if m.dir_count > 0 {
            strings.format_folders_count(m.dir_count)
        } else {
            strings.format_files_count(m.file_count)
        }
    } else {
        strings.describe_entry_kind(entry)
    };
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

    let parent_path = multi
        .map(|m| m.common_parent.as_path())
        .unwrap_or_else(|| entry.path.parent().unwrap_or(&entry.path));
    let parent_str = sanitize_single_line(&parent_path.to_string_lossy());
    let copy_path_target = multi
        .map(|m| m.common_parent.clone())
        .unwrap_or_else(|| entry.path.clone());

    let uses_recursive_size = is_dir || is_multi;
    let (size_short, size_exact, size_ratio): (String, String, f32) = if uses_recursive_size {
        if props.size_loading {
            (strings.prop_calculating.to_owned(), String::new(), 0.35)
        } else if let Some(sz) = props.recursive_size {
            let prefix = if sz.truncated { ">" } else { "" };
            let short = format!("{prefix}{}", format_bytes(sz.total_bytes));
            let exact = strings.format_exact_bytes(sz.total_bytes);
            let ratio = if sz.total_bytes == 0 {
                0.0
            } else {
                props
                    .volume_space
                    .as_ref()
                    .filter(|vs| vs.total_bytes > 0)
                    .map(|vs| {
                        (sz.total_bytes as f64 / vs.total_bytes as f64).clamp(0.02, 1.0) as f32
                    })
                    .unwrap_or_else(|| size_bar_ratio(sz.total_bytes))
            };
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
            .on_press(Message::CopyPath(copy_path_target)),
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

    let make_contents_and_free = || {
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
        } else if !is_multi && let Some(cnt) = entry.child_count {
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

        (col_contents, col_free)
    };

    let meta_rows: Vec<Element<'a, Message>> = if is_multi {
        let (col_contents, col_free) = make_contents_and_free();
        vec![
            row![col_kind, col_size].spacing(20).align_y(Center).into(),
            row![col_path, col_free].spacing(20).align_y(Center).into(),
            row![col_contents, Space::new().width(Length::FillPortion(1))]
                .spacing(20)
                .align_y(Center)
                .into(),
        ]
    } else {
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

        let mut rows = vec![
            row![col_kind, col_system]
                .spacing(20)
                .align_y(Center)
                .into(),
            row![col_path, col_size].spacing(20).align_y(Center).into(),
        ];
        if is_dir {
            let (col_contents, col_free) = make_contents_and_free();
            rows.push(
                row![col_contents, col_free]
                    .spacing(20)
                    .align_y(Center)
                    .into(),
            );
        }
        rows
    };

    let meta_grid = container(column(meta_rows).spacing(14))
        .width(Length::Fill)
        .padding([4, 6]);

    if is_multi {
        return column![hero_box, meta_grid].spacing(12).into();
    }

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
                    perm_checkbox_box(active, palette),
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

        let mut perms_col = column![
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
        .spacing(10);

        if !is_dir {
            let exec_active = is_posix_executable(mode);
            perms_col = perms_col.push(
                container(
                    button(
                        row![
                            perm_checkbox_box(exec_active, palette),
                            text(strings.perm_allow_exec)
                                .size(12)
                                .font(FONT_UI_SEMIBOLD)
                                .color(palette.text_primary),
                        ]
                        .spacing(8)
                        .align_y(Center),
                    )
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press_maybe(
                        editable.then_some(Message::PropertiesToggleExecutable(win_id)),
                    ),
                )
                .width(Length::Fill)
                .padding(8)
                .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0)),
            );
        }

        return perms_col.into();
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

fn properties_open_with_tab<'a>(
    app: &'a App,
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    if let Some(picker) = &props.exec_picker {
        return open_with_exec_picker_view(win_id, picker, palette, strings);
    }

    let type_key = open_with_props_type_key(props);
    let type_label = props
        .open_with_catalog
        .as_ref()
        .map(|c| c.type_label.as_str())
        .unwrap_or(type_key.as_str());

    let intro_card = container(
        row![
            container(
                icon_sliders_horizontal()
                    .size(16)
                    .color(palette.accent_primary)
            )
            .padding(8)
            .style(move |_| styles::panel_surface(
                palette.accent_surface,
                palette.accent_border,
                8.0
            )),
            column![
                text(strings.open_with_prompt)
                    .size(12)
                    .font(FONT_UI)
                    .color(palette.text_secondary),
                row![
                    text(sanitize_single_line(props.entry.name.as_str()))
                        .size(12)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    container(
                        text(format!("{type_label} ({type_key})"))
                            .size(11)
                            .font(FONT_MONO)
                            .color(palette.accent_primary),
                    )
                    .padding([2, 8])
                    .style(move |_| styles::panel_surface(
                        palette.accent_surface,
                        palette.accent_border,
                        9999.0
                    )),
                ]
                .spacing(8)
                .align_y(Center),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .width(Length::Fill)
    .padding(10)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0));

    let list_body: Element<'a, Message> = if props.open_with_loading {
        container(
            text(strings.open_with_loading)
                .size(12)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .width(Length::Fill)
        .padding(20)
        .into()
    } else if let Some(catalog) = &props.open_with_catalog {
        let user_override = app.config.open_with_defaults.get(&type_key);
        let active_default: Option<OpenWithApp> = user_override
            .map(|a| a.to_app(true, true, a.app_id.starts_with("custom:")))
            .or_else(|| catalog.system_default.clone());
        let active_default_id = active_default.as_ref().map(|a| a.id.as_str().to_owned());

        let mut list_col = column![].spacing(4).width(Length::Fill);

        if let Some(def_app) = active_default {
            list_col = list_col.push(open_with_section_label(
                strings.open_with_default_app,
                palette,
            ));
            list_col = list_col.push(render_open_with_app_row(
                app, win_id, props, &def_app, palette,
            ));
        }

        let mut recommended: Vec<OpenWithApp> = catalog
            .recommended
            .iter()
            .filter(|a| active_default_id.as_deref() != Some(a.id.as_str()))
            .cloned()
            .collect();

        if let Some(customs) = app.config.open_with_custom_apps.get(&type_key) {
            for custom in customs {
                if active_default_id.as_deref() != Some(custom.app_id.as_str())
                    && !recommended
                        .iter()
                        .any(|a| a.id.as_str() == custom.app_id.as_str())
                {
                    recommended.push(custom.to_app(true, false, true));
                }
            }
        }

        if let Some(sys_def) = &catalog.system_default
            && active_default_id.as_deref() != Some(sys_def.id.as_str())
            && !recommended
                .iter()
                .any(|a| a.id.as_str() == sys_def.id.as_str())
        {
            recommended.insert(0, sys_def.clone());
        }

        if !recommended.is_empty() {
            list_col = list_col.push(open_with_section_label(
                strings.open_with_recommended_apps,
                palette,
            ));
            for item in &recommended {
                list_col =
                    list_col.push(render_open_with_app_row(app, win_id, props, item, palette));
            }
        }

        let others: Vec<&OpenWithApp> = catalog
            .others
            .iter()
            .filter(|a| active_default_id.as_deref() != Some(a.id.as_str()))
            .collect();
        if !others.is_empty() {
            list_col = list_col.push(open_with_section_label(
                strings.open_with_other_apps,
                palette,
            ));
            for item in others {
                list_col =
                    list_col.push(render_open_with_app_row(app, win_id, props, item, palette));
            }
        }

        if active_default_id.is_none() && recommended.is_empty() && catalog.others.is_empty() {
            list_col = list_col.push(
                container(
                    text(strings.open_with_empty)
                        .size(12)
                        .font(FONT_UI)
                        .color(palette.text_muted),
                )
                .padding(14),
            );
        }

        scrollable(list_col.padding([4, 6]))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    } else {
        container(
            text(strings.open_with_empty)
                .size(12)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .padding(16)
        .into()
    };

    let apps_box = container(list_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(4)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0));

    let has_custom = !props.open_with_custom_command.trim().is_empty();
    let has_selection = props.open_with_selected_id.is_some() || has_custom;
    let has_saved_override = app.config.open_with_defaults.contains_key(&type_key)
        || props.entry.extension.as_deref().is_some_and(|e| {
            app.config
                .open_with_defaults
                .contains_key(&format!(".{}", e.to_ascii_lowercase()))
        })
        || props
            .open_with_catalog
            .as_ref()
            .is_some_and(|c| c.system_default.is_some());

    let custom_row = column![
        text(strings.open_with_custom_hint)
            .size(11)
            .font(FONT_UI)
            .color(palette.text_secondary),
        row![
            container(
                text_input(
                    strings.open_with_custom_placeholder,
                    &props.open_with_custom_command,
                )
                .on_input(move |val| Message::OpenWithCustomCommandChanged(win_id, val))
                .size(12)
                .padding([6, 10])
                .style(styles::search_input_style(palette)),
            )
            .padding([1, 4])
            .width(Length::Fill)
            .style(move |_| styles::panel_surface(
                palette.surface_low,
                palette.border,
                8.0
            )),
            with_tooltip(
                button(icon_folder().size(15).color(palette.text_primary))
                    .padding([7, 10])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::OpenWithToggleExecPicker(win_id)),
                strings.open_with_pick_exec,
                tooltip::Position::Top,
                palette,
            ),
        ]
        .spacing(8)
        .align_y(Center),
    ]
    .spacing(6);

    let add_btn = open_with_action_btn(
        strings.open_with_add_to_list,
        false,
        has_custom,
        palette,
        Message::OpenWithAddCustom(win_id),
    );
    let set_def_btn = open_with_action_btn(
        strings.open_with_set_default,
        true,
        has_selection,
        palette,
        Message::OpenWithSetDefault(win_id),
    );
    let reset_btn = open_with_action_btn(
        strings.open_with_reset_default,
        false,
        has_saved_override,
        palette,
        Message::OpenWithResetDefault(win_id),
    );
    let open_now_btn = open_with_action_btn(
        strings.open_with_open_once,
        false,
        has_selection,
        palette,
        Message::OpenWithOpenNow(win_id),
    );

    let actions_row = row![
        add_btn,
        set_def_btn,
        reset_btn,
        space::horizontal(),
        open_now_btn
    ]
    .spacing(8)
    .align_y(Center);

    column![intro_card, apps_box, custom_row, actions_row]
        .spacing(10)
        .width(Length::Fill)
        .into()
}

fn open_with_section_label<'a>(label: &'a str, palette: ThemePalette) -> Element<'a, Message> {
    container(
        text(label.to_uppercase())
            .size(10)
            .font(FONT_MONO_BOLD)
            .color(palette.text_muted),
    )
    .padding([6, 8])
    .into()
}

fn render_open_with_app_row<'a>(
    app: &'a App,
    win_id: window::Id,
    props: &'a PropertiesWindowState,
    item: &OpenWithApp,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let selected = props.open_with_selected_id.as_deref() == Some(item.id.as_str());
    let icon_widget: Element<'a, Message> = if let Some(hint) = item.icon_hint.as_deref()
        && let Some(handle) = app.open_with_icons.get(hint)
    {
        image(handle.clone())
            .width(Length::Fixed(20.0))
            .height(Length::Fixed(20.0))
            .into()
    } else if item.runs_in_terminal || item.is_custom {
        icon_terminal()
            .size(16)
            .color(palette.accent_primary)
            .into()
    } else {
        icon_external_link()
            .size(16)
            .color(palette.accent_primary)
            .into()
    };

    let mut row_content = row![
        icon_widget,
        text(sanitize_single_line(item.display_name.as_str()))
            .size(12)
            .font(if selected { FONT_UI_SEMIBOLD } else { FONT_UI })
            .color(palette.text_primary)
            .wrapping(text::Wrapping::None),
        space::horizontal(),
    ]
    .spacing(10)
    .align_y(Center);

    if selected {
        row_content = row_content.push(icon_check().size(13).color(palette.accent_primary));
    }

    let app_id = item.id.to_string();
    button(row_content)
        .width(Length::Fill)
        .padding([7, 10])
        .style(move |_, status| {
            let bg = if selected {
                palette.accent_surface
            } else {
                match status {
                    button::Status::Hovered | button::Status::Pressed => palette.surface_high,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: palette.text_primary,
                border: Border {
                    color: if selected {
                        palette.accent_border
                    } else {
                        Color::TRANSPARENT
                    },
                    width: if selected { 1.0 } else { 0.0 },
                    radius: Radius::from(6.0),
                },
                ..Default::default()
            }
        })
        .on_press(Message::OpenWithSelectApp(win_id, app_id))
        .into()
}

fn open_with_action_btn<'a>(
    label: &'a str,
    primary: bool,
    enabled: bool,
    palette: ThemePalette,
    msg: Message,
) -> Element<'a, Message> {
    button(text(label).size(12).font(if primary && enabled {
        FONT_UI_SEMIBOLD
    } else {
        FONT_UI
    }))
    .padding([6, 12])
    .style(move |theme, status| {
        if !enabled {
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
        } else if primary {
            styles::pill_primary_button(palette)(theme, status)
        } else {
            styles::pill_secondary_button(palette)(theme, status)
        }
    })
    .on_press_maybe(enabled.then_some(msg))
    .into()
}

fn open_with_exec_picker_view<'a>(
    win_id: window::Id,
    picker: &'a crate::app::ExecPickerState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let parent_btn = picker.current_dir.parent().map(|parent| {
        let parent_owned = parent.to_path_buf();
        button(
            row![
                icon_arrow_left().size(13),
                text("..").size(12).font(FONT_MONO)
            ]
            .spacing(4)
            .align_y(Center),
        )
        .padding([5, 10])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::OpenWithExecPickerNavigate(win_id, parent_owned))
    });

    let mut top_bar = row![].spacing(8).align_y(Center);
    if let Some(p_btn) = parent_btn {
        top_bar = top_bar.push(p_btn);
    }
    top_bar = top_bar
        .push(
            container(
                text(sanitize_single_line(&picker.current_dir.to_string_lossy()))
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_secondary)
                    .wrapping(text::Wrapping::None),
            )
            .padding([5, 8])
            .width(Length::Fill)
            .clip(true)
            .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 6.0)),
        )
        .push(
            button(text(strings.close).size(12))
                .padding([5, 12])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::OpenWithExecPickerClose(win_id)),
        );

    let list_el: Element<'a, Message> = if picker.loading {
        container(
            text(strings.loading)
                .size(12)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .padding(18)
        .into()
    } else if let Some(err) = &picker.error {
        container(
            text(err.as_str())
                .size(12)
                .font(FONT_UI)
                .color(palette.status_error),
        )
        .padding(14)
        .into()
    } else {
        let mut col = column![].spacing(2).width(Length::Fill);
        for entry in &picker.entries {
            let is_dir = entry.kind.is_dir_like();
            let path = entry.path.clone();
            let msg = if is_dir {
                Message::OpenWithExecPickerNavigate(win_id, path)
            } else {
                Message::OpenWithExecPickerPick(win_id, path)
            };
            let ico: Element<'a, Message> = if is_dir {
                icon_folder().size(14).color(palette.accent_primary).into()
            } else {
                icon_terminal()
                    .size(14)
                    .color(palette.text_secondary)
                    .into()
            };
            col = col.push(
                button(
                    row![
                        ico,
                        text(sanitize_single_line(entry.name.as_str()))
                            .size(12)
                            .font(if is_dir { FONT_UI_SEMIBOLD } else { FONT_MONO })
                            .color(palette.text_primary)
                            .wrapping(text::Wrapping::None),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
                .width(Length::Fill)
                .padding([5, 8])
                .style(styles::ghost_button(palette, 6.0))
                .on_press(msg),
            );
        }
        scrollable(col.padding(4))
            .height(Length::Fixed(275.0))
            .width(Length::Fill)
            .into()
    };

    column![
        top_bar,
        container(list_el)
            .width(Length::Fill)
            .padding(4)
            .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0)),
    ]
    .spacing(10)
    .width(Length::Fill)
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
                strings.grid_center_label,
                row![
                    button(
                        text(strings.grid_align_left)
                            .size(12)
                            .font(FONT_UI_SEMIBOLD)
                    )
                    .padding([5, 10])
                    .style(styles::tab_button(palette, !app.config.grid_center_items))
                    .on_press(Message::SetGridCenterItems(false)),
                    button(
                        text(strings.grid_align_center)
                            .size(12)
                            .font(FONT_UI_SEMIBOLD)
                    )
                    .padding([5, 10])
                    .style(styles::tab_button(palette, app.config.grid_center_items))
                    .on_press(Message::SetGridCenterItems(true)),
                ]
                .spacing(6)
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

    modal_overlay_backdrop(card)
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
    let menu_width = match &menu.kind {
        ContextMenuKind::Entry {
            posix_mode: Some(_),
            ..
        } => 278.0_f32,
        ContextMenuKind::Entry { path, .. }
            if fm_fs::archive::detect_archive_kind(path).is_some() =>
        {
            284.0_f32
        }
        ContextMenuKind::Entry { is_dir: true, .. } | ContextMenuKind::Background => 256.0_f32,
        _ => 236.0_f32,
    };
    let has_clipboard = app.clipboard.is_some();
    let show_open_as_root = fm_fs::supports_open_as_root();

    let mut items = column![].spacing(2).width(Length::Fill);
    let mut send_to_submenu: Option<(f32, std::path::PathBuf, std::path::PathBuf)> = None;

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
            posix_mode,
        } => {
            let close_sub_on_hover = menu.send_to_open;
            let mut offset_y = 8.0_f32;
            const ROW_STEP: f32 = 30.0;
            const DIVIDER_STEP: f32 = 9.0;

            items = items.push(ctx_entry_item(
                icon_external_link()
                    .size(14)
                    .color(palette.accent_primary)
                    .into(),
                strings.open_item,
                palette.text_primary,
                Message::ActivateEntry(*index),
                palette,
                close_sub_on_hover,
            ));
            offset_y += ROW_STEP;

            let open_tab_targets = app
                .current_tab()
                .map(|tab| {
                    resolve_open_in_new_tab_targets(
                        path,
                        *is_dir,
                        &tab.visible_entries,
                        &tab.selection.selected_ids,
                    )
                })
                .unwrap_or_else(|| {
                    if *is_dir {
                        vec![path.clone()]
                    } else {
                        Vec::new()
                    }
                });

            if !open_tab_targets.is_empty() {
                let target_count = open_tab_targets.len();
                let open_tab_item = if target_count > 1 {
                    ctx_entry_item_owned(
                        icon_folder().size(14).color(palette.accent_primary).into(),
                        format!("{} ({target_count})", strings.open_in_new_tab),
                        palette.text_primary,
                        Message::OpenManyInNewTabs(open_tab_targets),
                        palette,
                        close_sub_on_hover,
                    )
                } else {
                    ctx_entry_item(
                        icon_folder().size(14).color(palette.accent_primary).into(),
                        strings.open_in_new_tab,
                        palette.text_primary,
                        Message::OpenManyInNewTabs(open_tab_targets),
                        palette,
                        close_sub_on_hover,
                    )
                };
                items = items.push(open_tab_item);
                offset_y += ROW_STEP;
            }

            if *is_dir {
                if show_open_as_root
                    && !fm_core::is_virtual_trash_path(path)
                    && !fm_fs::archive::is_inside_archive_path(path)
                {
                    items = items.push(ctx_entry_item(
                        icon_shield_alert()
                            .size(14)
                            .color(palette.status_warning)
                            .into(),
                        strings.open_as_root,
                        palette.text_primary,
                        Message::OpenAsRoot(path.clone()),
                        palette,
                        close_sub_on_hover,
                    ));
                    offset_y += ROW_STEP;
                }
            } else {
                let sel_count_preview = app.current_tab().map(|t| t.selection.count()).unwrap_or(0);
                if sel_count_preview <= 1
                    && !fm_core::is_virtual_trash_path(path)
                    && !fm_fs::archive::is_inside_archive_path(path)
                    && fm_core::is_mountable_disk_image(path)
                {
                    items = items.push(ctx_entry_item(
                        icon_disc_3().size(14).color(palette.accent_primary).into(),
                        strings.mount_disk_image,
                        palette.text_primary,
                        Message::MountDiskImage(path.clone()),
                        palette,
                        close_sub_on_hover,
                    ));
                    offset_y += ROW_STEP;
                }
                if !fm_core::is_virtual_trash_path(path)
                    && !fm_fs::archive::is_inside_archive_path(path)
                {
                    items = items.push(ctx_entry_item(
                        icon_sliders_horizontal()
                            .size(14)
                            .color(palette.accent_primary)
                            .into(),
                        strings.open_with,
                        palette.text_primary,
                        Message::OpenWithDialog(path.clone()),
                        palette,
                        close_sub_on_hover,
                    ));
                    offset_y += ROW_STEP;
                }
                items = items.push(ctx_entry_item(
                    icon_eye().size(14).color(palette.text_secondary).into(),
                    strings.quicklook_title,
                    palette.text_primary,
                    Message::ToggleQuickLook,
                    palette,
                    close_sub_on_hover,
                ));
                offset_y += ROW_STEP;
            }
            let is_in_trash = app.is_current_tab_trash()
                || app
                    .places
                    .trash
                    .as_ref()
                    .is_some_and(|tr| path.starts_with(tr));
            if is_in_trash {
                items = items.push(ctx_entry_item(
                    icon_rotate_ccw()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.restore_item,
                    palette.accent_primary,
                    Message::RestoreFromTrash,
                    palette,
                    close_sub_on_hover,
                ));
                offset_y += ROW_STEP;
            }
            items = items.push(ctx_divider(palette)).push(ctx_entry_item(
                icon_copy().size(14).color(palette.text_secondary).into(),
                strings.copy,
                palette.text_primary,
                Message::CopySelected,
                palette,
                close_sub_on_hover,
            ));
            offset_y += DIVIDER_STEP + ROW_STEP;
            if !is_in_trash {
                items = items.push(ctx_entry_item(
                    icon_scissors()
                        .size(14)
                        .color(palette.text_secondary)
                        .into(),
                    strings.cut,
                    palette.text_primary,
                    Message::CutSelected,
                    palette,
                    close_sub_on_hover,
                ));
                offset_y += ROW_STEP;
                if has_clipboard {
                    items = items.push(ctx_entry_item(
                        icon_clipboard_paste()
                            .size(14)
                            .color(palette.accent_primary)
                            .into(),
                        strings.paste,
                        palette.text_primary,
                        Message::PasteClipboard,
                        palette,
                        close_sub_on_hover,
                    ));
                    offset_y += ROW_STEP;
                }
            }
            items = items.push(ctx_entry_item(
                icon_clipboard_copy()
                    .size(14)
                    .color(palette.text_secondary)
                    .into(),
                strings.copy_path,
                palette.text_primary,
                Message::CopyPath(path.clone()),
                palette,
                close_sub_on_hover,
            ));
            offset_y += ROW_STEP;
            let sel_count = app.current_tab().map(|t| t.selection.count()).unwrap_or(0);
            if !is_in_trash && !fm_fs::archive::is_inside_archive_path(path) {
                match rename_menu_action(sel_count) {
                    RenameMenuAction::Bulk => {
                        items = items.push(ctx_entry_item(
                            icon_pencil().size(14).color(palette.accent_primary).into(),
                            strings.br_menu_item,
                            palette.text_primary,
                            Message::OpenBulkRenameDialog,
                            palette,
                            close_sub_on_hover,
                        ));
                    }
                    RenameMenuAction::Single => {
                        items = items.push(ctx_entry_item(
                            icon_pencil().size(14).color(palette.text_secondary).into(),
                            strings.rename,
                            palette.text_primary,
                            Message::OpenRenameDialog,
                            palette,
                            close_sub_on_hover,
                        ));
                    }
                }
                offset_y += ROW_STEP;
            }
            let can_send_to_shortcut = !is_in_trash
                && !fm_core::is_virtual_trash_path(path)
                && !fm_fs::archive::is_inside_archive_path(path);
            if can_send_to_shortcut {
                let trigger_y = offset_y;
                items = items.push(ctx_submenu_trigger(
                    icon_send().size(14).color(palette.accent_primary).into(),
                    strings.ctx_send_to,
                    menu.send_to_open,
                    palette,
                ));
                if menu.send_to_open {
                    let here_dir = path
                        .parent()
                        .map(Path::to_path_buf)
                        .or_else(|| app.current_tab().map(|t| t.current_path.clone()))
                        .unwrap_or_default();
                    send_to_submenu = Some((trigger_y, path.clone(), here_dir));
                }
            }
            let tab_is_writable_local = !is_in_trash
                && app.current_tab().is_some_and(|t| {
                    !fm_core::is_virtual_trash_path(&t.current_path)
                        && fm_fs::archive::split_archive_path(&t.current_path).is_none()
                });
            if tab_is_writable_local {
                items = items.push(ctx_divider(palette));
                if sel_count <= 1 && !*is_dir && fm_fs::archive::detect_archive_kind(path).is_some()
                {
                    let folder_hint = path
                        .file_name()
                        .map(|n| fm_core::strip_known_archive_suffix(&n.to_string_lossy()))
                        .unwrap_or_default();
                    let subdir_label = if folder_hint.is_empty() {
                        strings.arch_extract_subdir.to_owned()
                    } else {
                        format!("{} ({folder_hint}/)", strings.arch_extract_subdir)
                    };
                    items = items
                        .push(ctx_entry_item(
                            icon_file_archive()
                                .size(14)
                                .color(palette.accent_primary)
                                .into(),
                            strings.arch_extract_here,
                            palette.text_primary,
                            Message::Archive(ArchiveUiMsg::Extract {
                                archive_path: path.clone(),
                                mode: fm_core::ExtractTargetMode::Here,
                            }),
                            palette,
                            close_sub_on_hover,
                        ))
                        .push(ctx_entry_item_owned(
                            icon_folder_plus()
                                .size(14)
                                .color(palette.accent_primary)
                                .into(),
                            subdir_label,
                            palette.text_primary,
                            Message::Archive(ArchiveUiMsg::Extract {
                                archive_path: path.clone(),
                                mode: fm_core::ExtractTargetMode::Subfolder,
                            }),
                            palette,
                            close_sub_on_hover,
                        ));
                }
                items = items
                    .push(ctx_entry_item(
                        icon_file_archive()
                            .size(14)
                            .color(palette.text_secondary)
                            .into(),
                        strings.arch_add_7z,
                        palette.text_primary,
                        Message::Archive(ArchiveUiMsg::QuickAddSevenZ),
                        palette,
                        close_sub_on_hover,
                    ))
                    .push(ctx_entry_item(
                        icon_file_archive()
                            .size(14)
                            .color(palette.accent_primary)
                            .into(),
                        strings.arch_compress_as,
                        palette.text_primary,
                        Message::Archive(ArchiveUiMsg::OpenCompressDialog),
                        palette,
                        close_sub_on_hover,
                    ));
                if *is_dir {
                    items = items.push(ctx_entry_item(
                        icon_search().size(14).color(palette.accent_primary).into(),
                        strings.dup_menu_action,
                        palette.text_primary,
                        Message::DuplicateFinder(DuplicateFinderMsg::OpenForPath(path.clone())),
                        palette,
                        close_sub_on_hover,
                    ));
                }
            }
            if !is_in_trash && let Some(mode) = *posix_mode {
                let exec_enabled = is_posix_executable(mode);
                let next_mode = toggle_posix_executable_mode(mode);
                items = items
                    .push(ctx_divider(palette))
                    .push(
                        container(
                            text(strings.permissions.to_uppercase())
                                .size(10)
                                .font(FONT_MONO_BOLD)
                                .color(palette.text_muted),
                        )
                        .padding([2, 10]),
                    )
                    .push(ctx_entry_item(
                        perm_checkbox_box(exec_enabled, palette),
                        strings.perm_allow_exec,
                        palette.text_primary,
                        Message::ToggleEntryExecutable {
                            path: path.clone(),
                            next_mode,
                        },
                        palette,
                        close_sub_on_hover,
                    ));
            }
            items = items
                .push(ctx_entry_item(
                    icon_info().size(14).color(palette.text_secondary).into(),
                    strings.properties,
                    palette.text_primary,
                    Message::OpenProperties(path.clone()),
                    palette,
                    close_sub_on_hover,
                ))
                .push(ctx_divider(palette))
                .push(ctx_entry_item(
                    icon_trash_2().size(14).color(palette.status_error).into(),
                    if is_in_trash {
                        strings.delete_permanent
                    } else {
                        strings.move_to_trash
                    },
                    palette.status_error,
                    Message::RequestDelete(is_in_trash),
                    palette,
                    close_sub_on_hover,
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
                ));
            if show_open_as_root && !fm_core::is_virtual_trash_path(path) {
                items = items.push(ctx_item(
                    icon_shield_alert()
                        .size(14)
                        .color(palette.status_warning)
                        .into(),
                    strings.open_as_root,
                    palette.text_primary,
                    Message::OpenAsRoot(path.clone()),
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
                    icon_info().size(14).color(palette.text_secondary).into(),
                    strings.properties,
                    palette.text_primary,
                    Message::OpenProperties(path.clone()),
                    palette,
                ));
            if fm_core::is_virtual_trash_path(path) {
                items = items.push(ctx_divider(palette)).push(ctx_item(
                    icon_trash_2().size(14).color(palette.status_error).into(),
                    strings.empty_trash,
                    palette.status_error,
                    Message::RequestEmptyTrash,
                    palette,
                ));
            } else if *is_favorite {
                items = items.push(ctx_divider(palette)).push(ctx_item(
                    icon_star_off().size(14).color(palette.status_error).into(),
                    strings.remove_favorite,
                    palette.status_error,
                    Message::RemoveFavorite(path.clone()),
                    palette,
                ));
            }
        }
        ContextMenuKind::Drive {
            label,
            mount_path,
            device_path,
            is_mounted,
            can_unmount,
            is_ejectable,
            is_disk_image,
        } => {
            if *is_mounted {
                items = items
                    .push(ctx_item(
                        icon_external_link()
                            .size(14)
                            .color(palette.accent_primary)
                            .into(),
                        strings.open_item,
                        palette.text_primary,
                        Message::NavigateTo(mount_path.clone()),
                        palette,
                    ))
                    .push(ctx_item(
                        icon_folder().size(14).color(palette.accent_primary).into(),
                        strings.open_in_new_tab,
                        palette.text_primary,
                        Message::OpenInNewTab(mount_path.clone()),
                        palette,
                    ));
                if show_open_as_root {
                    items = items.push(ctx_item(
                        icon_shield_alert()
                            .size(14)
                            .color(palette.status_warning)
                            .into(),
                        strings.open_as_root,
                        palette.text_primary,
                        Message::OpenAsRoot(mount_path.clone()),
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
                        Message::CopyPath(mount_path.clone()),
                        palette,
                    ))
                    .push(ctx_item(
                        icon_info().size(14).color(palette.text_secondary).into(),
                        strings.properties,
                        palette.text_primary,
                        Message::OpenProperties(mount_path.clone()),
                        palette,
                    ));
                if *can_unmount || *is_ejectable {
                    items = items.push(ctx_divider(palette));
                }
                if *can_unmount {
                    items = items.push(ctx_item(
                        icon_arrow_up_from_line()
                            .size(14)
                            .color(palette.status_warning)
                            .into(),
                        if *is_disk_image {
                            strings.unmount_disk_image
                        } else {
                            strings.unmount_drive
                        },
                        palette.text_primary,
                        Message::UnmountDrive {
                            device_path: device_path.clone(),
                            mount_path: Some(mount_path.clone()),
                            label: label.clone(),
                        },
                        palette,
                    ));
                }
                if *is_ejectable && !*is_disk_image {
                    items = items.push(ctx_item(
                        icon_power().size(14).color(palette.status_error).into(),
                        strings.eject_safely,
                        palette.text_primary,
                        Message::EjectDrive {
                            device_path: device_path.clone(),
                            mount_path: Some(mount_path.clone()),
                            label: label.clone(),
                        },
                        palette,
                    ));
                }
            } else if let Some(dev) = device_path {
                items = items.push(ctx_item(
                    icon_plug().size(14).color(palette.accent_primary).into(),
                    strings.mount_drive,
                    palette.text_primary,
                    Message::MountAndOpenDrive(dev.clone()),
                    palette,
                ));
                if *is_ejectable {
                    items = items.push(ctx_divider(palette)).push(ctx_item(
                        icon_power().size(14).color(palette.status_error).into(),
                        strings.eject_safely,
                        palette.text_primary,
                        Message::EjectDrive {
                            device_path: Some(dev.clone()),
                            mount_path: None,
                            label: label.clone(),
                        },
                        palette,
                    ));
                }
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
                items = items.push(ctx_divider(palette)).push(
                    container(
                        container(
                            text_input(strings.filter_ext_placeholder, &app.filter_ext_draft)
                                .id(filter_ext_input_id())
                                .size(12)
                                .font(FONT_UI)
                                .padding([5, 8])
                                .width(Length::Fill)
                                .style(styles::search_input_style(palette))
                                .on_input(Message::FilterExtDraftChanged)
                                .on_submit(Message::SubmitFilterExtDraft),
                        )
                        .padding([2, 4])
                        .style(move |_| {
                            styles::panel_surface(palette.surface_low, palette.accent_border, 8.0)
                        }),
                    )
                    .padding([4, 4]),
                );
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
            if let Some(top_undo) = app.undo_stack.last() {
                let undo_label = match top_undo {
                    UndoAction::Trash { .. } => strings.undo_trash_action,
                    UndoAction::BulkRename { .. } => strings.br_undo_last,
                    UndoAction::Rename { .. } => strings.undo_action,
                };
                items = items.push(ctx_item(
                    icon_rotate_ccw()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    undo_label,
                    palette.text_primary,
                    Message::UndoLastAction,
                    palette,
                ));
            } else if app.last_bulk_rename_undo.is_some() {
                items = items.push(ctx_item(
                    icon_rotate_ccw()
                        .size(14)
                        .color(palette.accent_primary)
                        .into(),
                    strings.br_undo_last,
                    palette.text_primary,
                    Message::UndoLastBulkRename,
                    palette,
                ));
            }
            if let Some(tab) = app.current_tab() {
                let can_scan_dups = !fm_core::is_virtual_trash_path(&tab.current_path)
                    && fm_fs::archive::split_archive_path(&tab.current_path).is_none();
                if can_scan_dups {
                    items = items.push(ctx_item(
                        icon_search().size(14).color(palette.accent_primary).into(),
                        strings.dup_menu_action,
                        palette.text_primary,
                        Message::DuplicateFinder(DuplicateFinderMsg::OpenForCurrentDir),
                        palette,
                    ));
                }
                if show_open_as_root && can_scan_dups {
                    items = items.push(ctx_item(
                        icon_shield_alert()
                            .size(14)
                            .color(palette.status_warning)
                            .into(),
                        strings.open_as_root,
                        palette.text_primary,
                        Message::OpenAsRoot(tab.current_path.clone()),
                        palette,
                    ));
                }
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

    let root_extra = if show_open_as_root { 30.0 } else { 0.0 };
    let undo_extra = if !app.undo_stack.is_empty() || app.last_bulk_rename_undo.is_some() {
        30.0
    } else {
        0.0
    };
    let clipboard_extra = if has_clipboard { 30.0 } else { 0.0 };
    let est_height: f32 = match &menu.kind {
        ContextMenuKind::NewMenu => 84.0,
        ContextMenuKind::Entry {
            path,
            is_dir,
            posix_mode,
            ..
        } => {
            if app.is_current_tab_trash() {
                192.0
            } else if *is_dir {
                416.0 + root_extra
            } else {
                let archive_extra = if fm_fs::archive::detect_archive_kind(path).is_some() {
                    60.0
                } else {
                    0.0
                };
                let disk_image_extra = if fm_core::is_mountable_disk_image(path) {
                    30.0
                } else {
                    0.0
                };
                let perm_extra = if posix_mode.is_some() { 59.0 } else { 0.0 };
                let open_tab_extra = app
                    .current_tab()
                    .map(|tab| {
                        if resolve_open_in_new_tab_targets(
                            path,
                            *is_dir,
                            &tab.visible_entries,
                            &tab.selection.selected_ids,
                        )
                        .is_empty()
                        {
                            0.0
                        } else {
                            30.0
                        }
                    })
                    .unwrap_or(0.0);
                416.0 + archive_extra + disk_image_extra + perm_extra + open_tab_extra
            }
        }
        ContextMenuKind::SidebarPlace {
            path, is_favorite, ..
        } => {
            if fm_core::is_virtual_trash_path(path) {
                186.0
            } else if *is_favorite {
                186.0 + root_extra
            } else {
                150.0 + root_extra
            }
        }
        ContextMenuKind::Drive {
            is_mounted,
            can_unmount,
            is_ejectable,
            is_disk_image,
            ..
        } => {
            if !*is_mounted {
                if *is_ejectable { 92.0 } else { 52.0 }
            } else {
                let mut h = 150.0 + root_extra;
                if *can_unmount || *is_ejectable {
                    h += 10.0;
                }
                if *can_unmount {
                    h += 30.0;
                }
                if *is_ejectable && !*is_disk_image {
                    h += 30.0;
                }
                h
            }
        }
        ContextMenuKind::Background => 296.0 + clipboard_extra + root_extra + undo_extra,
        ContextMenuKind::FilterMenu => 128.0,
        ContextMenuKind::LanguageMenu => 246.0,
        ContextMenuKind::PreferencesMenu => 84.0,
    };

    let main_card_w = menu_width + 16.0;
    let max_card_h = (app.window_size.height - 16.0).max(200.0);
    let rendered_h = est_height.min(max_card_h);
    let (x, y) = compute_context_menu_origin(
        menu.position,
        main_card_w + 2.0,
        rendered_h,
        app.window_size,
    );

    // Só ativa barra de rolagem se o menu realmente exceder a altura útil da janela.
    let needs_scroll = est_height > max_card_h;
    let card_body: Element<'a, Message> = if needs_scroll {
        scrollable(items.width(Length::Fixed(menu_width)))
            .height(Length::Shrink)
            .into()
    } else {
        items.width(Length::Fixed(menu_width)).into()
    };

    let card = opaque(
        container(card_body)
            .max_height(max_card_h)
            .padding(8)
            .style(styles::modal_card(palette)),
    );

    let backdrop_catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
        .on_press(Message::CloseContextMenu)
        .on_right_press(Message::CloseContextMenu);

    let main_layer = container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: y,
            right: 0.0,
            bottom: 0.0,
            left: x,
        });

    if let Some((trigger_offset_y, fallback_target, here_dir)) = send_to_submenu {
        let sub_inner_w = 276.0_f32;
        let sub_card_w = sub_inner_w + 16.0;
        let sub_card_h = 84.0_f32;
        let (sub_x, sub_y) = compute_submenu_origin(
            x,
            y,
            main_card_w,
            trigger_offset_y,
            sub_card_w,
            sub_card_h,
            app.window_size,
        );

        let here_label = if cfg!(target_os = "windows") {
            strings.ctx_shortcut_here_win
        } else {
            strings.ctx_shortcut_here_unix
        };

        let sub_items = column![
            ctx_item(
                icon_monitor().size(14).color(palette.accent_primary).into(),
                strings.ctx_shortcut_desktop,
                palette.text_primary,
                Message::CreateShortcutInDir {
                    dest_dir: None,
                    fallback_target: fallback_target.clone(),
                },
                palette,
            ),
            ctx_item(
                icon_link_2().size(14).color(palette.text_secondary).into(),
                here_label,
                palette.text_primary,
                Message::CreateShortcutInDir {
                    dest_dir: Some(here_dir),
                    fallback_target,
                },
                palette,
            ),
        ]
        .spacing(2)
        .width(Length::Fixed(sub_inner_w));

        let sub_card = opaque(
            mouse_area(
                container(sub_items)
                    .padding(8)
                    .style(styles::modal_card(palette)),
            )
            .on_enter(Message::ContextMenuSetSendToOpen(true)),
        );

        let sub_layer = container(sub_card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: sub_y,
                right: 0.0,
                bottom: 0.0,
                left: sub_x,
            });

        stack![backdrop_catcher, main_layer, sub_layer].into()
    } else {
        stack![backdrop_catcher, main_layer].into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameMenuAction {
    Single,
    Bulk,
}

fn rename_menu_action(selected_count: usize) -> RenameMenuAction {
    if selected_count > 1 {
        RenameMenuAction::Bulk
    } else {
        RenameMenuAction::Single
    }
}

/// Posiciona o submenu lateral em cascata ("Enviar para... ▸") alinhado à linha de gatilho:
/// - Abre à direita do menu principal (com sobreposição de 2px para evitar vão morto ao mover o mouse);
/// - Inverte automaticamente para a esquerda se não houver espaço até a borda direita da janela;
/// - Limita a coordenada vertical dentro da área útil da janela.
fn compute_submenu_origin(
    main_x: f32,
    main_y: f32,
    main_w: f32,
    trigger_offset_y: f32,
    sub_w: f32,
    sub_h: f32,
    window_size: iced::Size,
) -> (f32, f32) {
    const MARGIN: f32 = 8.0;
    const OVERLAP: f32 = 2.0;

    let right_x = main_x + main_w - OVERLAP;
    let left_x = main_x - sub_w + OVERLAP;
    let sub_x = if right_x + sub_w + MARGIN <= window_size.width {
        right_x
    } else if left_x >= MARGIN {
        left_x
    } else {
        (window_size.width - sub_w - MARGIN).max(MARGIN)
    };

    let desired_y = (main_y + trigger_offset_y - 8.0).max(MARGIN);
    let max_y = (window_size.height - sub_h - MARGIN).max(MARGIN);
    let sub_y = desired_y.min(max_y);

    (sub_x, sub_y)
}

/// Posiciona o menu de contexto no estilo Windows/nativo:
/// - Abre para baixo do cursor quando couber;
/// - Quando acionado próximo à borda inferior (ex.: pasta no rodapé), inverte e abre **acima** do cursor;
/// - Mesmo comportamento horizontal (inverte para a esquerda próximo à borda direita);
/// - Mantém margem mínima segura dentro da janela.
fn compute_context_menu_origin(
    cursor: iced::Point,
    menu_w: f32,
    menu_h: f32,
    window_size: iced::Size,
) -> (f32, f32) {
    const MARGIN: f32 = 8.0;
    let clamped_w = menu_w.min((window_size.width - MARGIN * 2.0).max(160.0));
    let clamped_h = menu_h.min((window_size.height - MARGIN * 2.0).max(120.0));

    let x = if cursor.x + clamped_w + MARGIN <= window_size.width {
        cursor.x.max(MARGIN)
    } else if cursor.x - clamped_w >= MARGIN {
        cursor.x - clamped_w
    } else {
        (window_size.width - clamped_w - MARGIN).max(MARGIN)
    };

    let y = if cursor.y + clamped_h + MARGIN <= window_size.height {
        cursor.y.max(MARGIN)
    } else if cursor.y - clamped_h >= MARGIN {
        cursor.y - clamped_h
    } else {
        (window_size.height - clamped_h - MARGIN).max(MARGIN)
    };

    (x, y)
}

fn ctx_entry_item<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    label_color: iced::Color,
    msg: Message,
    palette: ThemePalette,
    close_sub_on_hover: bool,
) -> Element<'a, Message> {
    let item = ctx_item(icon, label, label_color, msg, palette);
    if close_sub_on_hover {
        mouse_area(item)
            .on_enter(Message::ContextMenuSetSendToOpen(false))
            .into()
    } else {
        item
    }
}

fn ctx_entry_item_owned<'a>(
    icon: Element<'a, Message>,
    label: String,
    label_color: iced::Color,
    msg: Message,
    palette: ThemePalette,
    close_sub_on_hover: bool,
) -> Element<'a, Message> {
    let item = ctx_item_owned(icon, label, label_color, msg, palette);
    if close_sub_on_hover {
        mouse_area(item)
            .on_enter(Message::ContextMenuSetSendToOpen(false))
            .into()
    } else {
        item
    }
}

fn ctx_submenu_trigger<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    is_open: bool,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let btn = button(
        row![
            icon,
            text(label)
                .size(12)
                .font(FONT_UI)
                .color(if is_open {
                    palette.accent_primary
                } else {
                    palette.text_primary
                })
                .wrapping(text::Wrapping::None),
            Space::new().width(Length::Fill),
            icon_chevron_right().size(13).color(if is_open {
                palette.accent_primary
            } else {
                palette.text_muted
            }),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .width(Length::Fill)
    .padding([5, 10])
    .style(move |theme, status| {
        if is_open {
            styles::tab_button(palette, true)(theme, status)
        } else {
            styles::ghost_button(palette, 6.0)(theme, status)
        }
    })
    .on_press(Message::ContextMenuSetSendToOpen(!is_open));

    mouse_area(btn)
        .on_enter(Message::ContextMenuSetSendToOpen(true))
        .into()
}

fn ctx_item<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    label_color: iced::Color,
    msg: Message,
    palette: ThemePalette,
) -> Element<'a, Message> {
    button(
        row![
            icon,
            text(label)
                .size(12)
                .font(FONT_UI)
                .color(label_color)
                .wrapping(text::Wrapping::None)
        ]
        .spacing(8)
        .align_y(Center),
    )
    .width(Length::Fill)
    .padding([5, 10])
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
        row![
            icon,
            text(label)
                .size(12)
                .font(FONT_UI)
                .color(label_color)
                .wrapping(text::Wrapping::None)
        ]
        .spacing(8)
        .align_y(Center),
    )
    .width(Length::Fill)
    .padding([5, 10])
    .style(styles::ghost_button(palette, 6.0))
    .on_press(msg)
    .into()
}

fn perm_checkbox_box<'a>(active: bool, palette: ThemePalette) -> Element<'a, Message> {
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
    })
    .into()
}

fn ctx_divider<'a>(palette: ThemePalette) -> Element<'a, Message> {
    container(
        container(Space::new().width(Length::Fill).height(1)).style(styles::divider_line(palette)),
    )
    .padding([3, 4])
    .into()
}

#[cfg(test)]
mod tests {
    use super::{
        RenameMenuAction, compute_context_menu_origin, compute_submenu_origin, rename_menu_action,
    };
    use iced::{Point, Size};

    #[test]
    fn rename_menu_action_switches_between_single_and_bulk() {
        assert_eq!(rename_menu_action(0), RenameMenuAction::Single);
        assert_eq!(rename_menu_action(1), RenameMenuAction::Single);
        assert_eq!(rename_menu_action(2), RenameMenuAction::Bulk);
        assert_eq!(rename_menu_action(15), RenameMenuAction::Bulk);
    }

    #[test]
    fn context_menu_flips_above_and_left_near_viewport_edges() {
        let win = Size::new(1280.0, 800.0);
        // Área superior esquerda: abre para baixo e para a direita do cursor.
        let (x, y) = compute_context_menu_origin(Point::new(120.0, 100.0), 280.0, 480.0, win);
        assert_eq!((x, y), (120.0, 100.0));

        // Item na parte inferior da tela: inverte e abre acima do cursor.
        let (x_bot, y_bot) =
            compute_context_menu_origin(Point::new(300.0, 720.0), 280.0, 480.0, win);
        assert_eq!(x_bot, 300.0);
        assert_eq!(y_bot, 720.0 - 480.0);

        // Item próximo à borda direita: inverte para a esquerda do cursor.
        let (x_right, y_right) =
            compute_context_menu_origin(Point::new(1200.0, 140.0), 280.0, 480.0, win);
        assert_eq!(x_right, 1200.0 - 280.0);
        assert_eq!(y_right, 140.0);

        // Janela curta onde não cabe nem acima nem abaixo do cursor: encaixa na margem superior/inferior.
        let short_win = Size::new(1000.0, 560.0);
        let (_, y_mid) =
            compute_context_menu_origin(Point::new(200.0, 300.0), 280.0, 480.0, short_win);
        assert_eq!(y_mid, 560.0 - 480.0 - 8.0);
    }

    #[test]
    fn submenu_flips_left_near_right_edge_and_clamps_vertically() {
        let win = Size::new(1280.0, 800.0);
        // Espaço livre à direita: abre à direita do menu principal com overlap de 2px.
        let (sx, sy) = compute_submenu_origin(200.0, 100.0, 260.0, 180.0, 292.0, 84.0, win);
        assert_eq!(sx, 200.0 + 260.0 - 2.0);
        assert_eq!(sy, 100.0 + 180.0 - 8.0);

        // Menu principal encostado à borda direita: submenu inverte para a esquerda.
        let (sx_left, _) = compute_submenu_origin(990.0, 100.0, 260.0, 180.0, 292.0, 84.0, win);
        assert_eq!(sx_left, 990.0 - 292.0 + 2.0);

        // Gatilho muito próximo ao rodapé: submenu permanece dentro da margem inferior.
        let (_, sy_bot) = compute_submenu_origin(200.0, 620.0, 260.0, 160.0, 292.0, 84.0, win);
        assert_eq!(sy_bot, 800.0 - 84.0 - 8.0);
    }
}
