use std::path::Path;

use fm_core::{
    ConflictPolicy, DIR_FILTER_SENTINEL, FileEntry, Language, Strings, ThemePreset, ViewMode,
    WindowDecorationsMode, format_bytes,
};
use fm_theme::{FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_SEMIBOLD, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, space, text, text_input,
};
use iced::{Element, Length, Padding};
use lucide_icons::iced::{
    icon_check_square, icon_clipboard_copy, icon_clipboard_paste, icon_copy, icon_external_link,
    icon_eye, icon_file_plus, icon_filter, icon_folder, icon_folder_plus, icon_info, icon_pencil,
    icon_rotate_ccw, icon_scissors, icon_terminal, icon_trash_2, icon_x,
};

use crate::app::{ActiveDialog, App, ContextMenuKind, ContextMenuState, Message, dialog_input_id};

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

pub fn preferences_modal<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let theme_name = match app.config.theme {
        ThemePreset::AetherLight => "AetherLight",
        ThemePreset::ObsidianFlow => "ObsidianFlow",
    };
    let lang_name = match app.config.language {
        Language::PtBr => "Português (BR)",
        Language::EnUs => "English (US)",
    };
    let deco_name = match app.config.window_decorations {
        WindowDecorationsMode::NativeWm => "Native WM",
        WindowDecorationsMode::IntegratedCsd => "Integrated CSD",
    };

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
        button(text("Grid").size(11).font(FONT_MONO))
            .padding([4, 10])
            .style(styles::tab_button(
                palette,
                app.config.default_view_mode == ViewMode::Grid
            ))
            .on_press(Message::SetDefaultViewMode(ViewMode::Grid)),
        button(text("List").size(11).font(FONT_MONO))
            .padding([4, 10])
            .style(styles::tab_button(
                palette,
                app.config.default_view_mode == ViewMode::DetailTable
            ))
            .on_press(Message::SetDefaultViewMode(ViewMode::DetailTable)),
        button(text("Columns").size(11).font(FONT_MONO))
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
            pref_row(
                strings.theme_label,
                button(text(theme_name).size(12).font(FONT_MONO_BOLD))
                    .padding([5, 12])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::ToggleTheme)
                    .into(),
            ),
            pref_row(
                strings.lang_label,
                button(text(lang_name).size(12).font(FONT_MONO_BOLD))
                    .padding([5, 12])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::ToggleLanguage)
                    .into(),
            ),
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
                        text(entry.name.as_str())
                            .size(12)
                            .font(FONT_UI)
                            .color(palette.text_primary),
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
                text(
                    root.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| root.to_string_lossy().into_owned()),
                )
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
        }
    }

    let est_height = match &menu.kind {
        ContextMenuKind::NewMenu => 84.0,
        ContextMenuKind::Entry { .. } => 340.0,
        ContextMenuKind::Background => 240.0,
        ContextMenuKind::FilterMenu => 320.0,
    };
    let max_x = (app.window_size.width - menu_width - 12.0).max(8.0);
    let max_y = (app.window_size.height - est_height - 12.0).max(8.0);
    let x = menu.position.x.clamp(8.0, max_x);
    let y = menu.position.y.clamp(8.0, max_y);

    let card = container(items.width(Length::Fixed(menu_width)))
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
