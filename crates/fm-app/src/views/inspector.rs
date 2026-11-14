use fm_core::{Strings, TabState, format_bytes};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI_BOLD, FONT_UI_SEMIBOLD, Spacing, ThemePalette, styles,
};
use iced::Alignment::Center;
use iced::widget::{Space, button, column, container, row, scrollable, space, text};
use iced::{Element, Length, Padding};
use lucide_icons::iced::icon_x;

use crate::app::{App, Message};
use crate::views::entry_icon;

pub fn drawer<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    if !app.config.show_inspector {
        return Space::new().width(0).height(0).into();
    }

    let body: Element<'a, Message> = match tab {
        Some(tab) => {
            let count = tab.selection.count();
            if count == 0 {
                text(strings.inspector_empty)
                    .size(12)
                    .color(palette.text_secondary)
                    .into()
            } else if count > 1 {
                let total = tab.selection.selected_bytes(&tab.visible_entries);
                column![
                    text(format!("{count} {}", strings.items_unit))
                        .size(15)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    text(format_bytes(total).to_string())
                        .size(13)
                        .font(FONT_MONO_BOLD)
                        .color(palette.accent_primary),
                ]
                .spacing(6)
                .into()
            } else if let Some(entry) = tab.selection.single_selected(&tab.visible_entries) {
                let (icon_widget, _) = entry_icon(entry, palette, 46.0);
                let perms = entry.permissions.format_compact(entry.kind.is_dir_like());
                let size_str = if entry.kind.is_dir_like() {
                    format!("{} {}", entry.child_count.unwrap_or(0), strings.items_unit)
                } else {
                    format_bytes(entry.size_bytes).to_string()
                };

                let mut info_col = column![
                    container(icon_widget)
                        .width(Length::Fill)
                        .height(84)
                        .align_x(Center)
                        .align_y(Center)
                        .style(move |_| styles::panel_surface(
                            palette.surface_low,
                            palette.border,
                            12.0
                        )),
                    text(entry.name.as_str())
                        .size(15)
                        .font(FONT_UI_BOLD)
                        .color(palette.text_primary),
                    text(entry.path.to_string_lossy().into_owned())
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                    meta_row(strings.col_size, size_str, palette),
                    meta_row(strings.permissions, perms.to_string(), palette),
                ]
                .spacing(10);

                if let Some(tel) = &tab.selected_telemetry {
                    if let Some(mime) = &tel.mime_type {
                        info_col = info_col.push(meta_row(
                            strings.mime_type,
                            mime.to_string(),
                            palette,
                        ));
                    }
                    if let Some(hash) = &tel.sha256_prefix {
                        info_col = info_col.push(meta_row(
                            strings.checksum,
                            hash.to_string(),
                            palette,
                        ));
                    }
                }

                info_col.into()
            } else {
                Space::new().into()
            }
        }
        None => Space::new().into(),
    };

    let drawer_panel = container(
        column![
            row![
                text(strings.inspector_title)
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(palette.text_muted),
                space::horizontal(),
                button(icon_x().size(14))
                    .padding([3, 5])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::ToggleInspector),
            ]
            .align_y(Center),
            scrollable(body).height(Length::Fill),
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(Spacing::INSPECTOR_WIDTH))
    .height(Length::Fill)
    .padding(Padding::from([14, 14]))
    .style(styles::inspector_container(palette));

    row![
        container(Space::new().width(1).height(Length::Fill))
            .style(styles::divider_line(palette)),
        drawer_panel,
    ]
    .height(Length::Fill)
    .into()
}

fn meta_row<'a>(label: &'a str, value: String, palette: ThemePalette) -> Element<'a, Message> {
    container(
        column![
            text(label).size(10).font(FONT_MONO).color(palette.text_muted),
            text(value)
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_primary),
        ]
        .spacing(2),
    )
    .width(Length::Fill)
    .padding([8, 10])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0))
    .into()
}

pub fn quicklook_modal<'a>(
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let entry = tab.and_then(|t| t.selection.single_selected(&t.visible_entries));
    let snippet = tab
        .and_then(|t| t.selected_telemetry.as_ref())
        .and_then(|st| st.preview_snippet.as_deref());

    let title = entry
        .map(|e| e.name.as_str())
        .unwrap_or(strings.quicklook_title);

    let body: Element<'a, Message> = if let Some(code) = snippet {
        container(
            scrollable(
                text(code)
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.text_primary),
            )
            .height(Length::Fixed(300.0)),
        )
        .width(Length::Fill)
        .padding(14)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
        .into()
    } else if let Some(e) = entry {
        let (icon_widget, _) = entry_icon(e, palette, 56.0);
        column![
            icon_widget,
            text(e.name.as_str())
                .size(16)
                .font(FONT_UI_BOLD)
                .color(palette.text_primary),
            text(format_bytes(e.size_bytes).to_string())
                .size(13)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        ]
        .spacing(10)
        .align_x(Center)
        .into()
    } else {
        text(strings.inspector_empty).into()
    };

    let card = container(
        column![
            row![
                text(format!("{} — {title}", strings.quicklook_title))
                    .size(14)
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
                .on_press(Message::CloseQuickLook),
            ]
            .align_y(Center),
            body,
        ]
        .spacing(14)
        .width(Length::Fixed(620.0)),
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
