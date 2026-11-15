use fm_core::{Language, Strings, ThemePreset, ViewMode, WindowDecorationsMode, format_bytes};
use fm_theme::{FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_SEMIBOLD, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{Space, button, column, container, row, space, text, text_input};
use iced::{Element, Length};
use lucide_icons::iced::{icon_file_plus, icon_folder_plus, icon_pencil, icon_trash_2, icon_x};

use crate::app::{ActiveDialog, App, Message, dialog_input_id};

pub fn dialog_modal<'a>(
    dialog: &'a ActiveDialog,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let body: Element<'a, Message> = match dialog {
        ActiveDialog::CreateItem { is_dir, name } => {
            let folder_pill = button(
                row![icon_folder_plus().size(13), text(strings.new_folder).size(12)]
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
                text(warn)
                    .size(13)
                    .font(FONT_UI)
                    .color(if *permanent {
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
        container(row![
            text(label)
                .size(13)
                .font(FONT_UI)
                .color(palette.text_primary),
            space::horizontal(),
            control,
        ]
        .align_y(Center))
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
                    text(if app.config.show_hero_card { "ON" } else { "OFF" })
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
