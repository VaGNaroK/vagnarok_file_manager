use fm_core::{Strings, TabState, ThemePreset, format_bytes};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, Spacing, ThemePalette,
    styles,
};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, image, row, scrollable, space, text, text_editor, text_input,
};
use iced::{ContentFit, Element, Length, Padding};
use lucide_icons::iced::{icon_file_text, icon_folder, icon_search, icon_x};

use crate::app::{App, Message, deep_search_input_id, thumbnail_key_for_entry};
use crate::views::{modal_overlay_backdrop, render_entry_icon, sanitize_single_line};

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
                    text(strings.format_items_count(count))
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
                let preview_box: Element<'a, Message> =
                    if let Some(handle) = app.gpu_thumbnails.get(&thumbnail_key_for_entry(entry)) {
                        image(handle.clone())
                            .width(74)
                            .height(74)
                            .content_fit(ContentFit::Contain)
                            .into()
                    } else {
                        render_entry_icon(app, entry, palette, 46.0)
                    };

                let perms = entry.permissions.format_compact(entry.kind.is_dir_like());
                let size_str = if entry.kind.is_dir_like() {
                    entry
                        .child_count
                        .map(|c| strings.format_items_count(c as usize))
                        .unwrap_or_else(|| strings.dir_badge.to_owned())
                } else {
                    format_bytes(entry.size_bytes).to_string()
                };

                let mut info_col = column![
                    container(preview_box)
                        .width(Length::Fill)
                        .height(92)
                        .align_x(Center)
                        .align_y(Center)
                        .style(move |_| styles::panel_surface(
                            palette.surface_low,
                            palette.border,
                            12.0
                        )),
                    text(sanitize_single_line(entry.name.as_str()))
                        .size(15)
                        .font(FONT_UI_BOLD)
                        .color(palette.text_primary),
                    text(sanitize_single_line(&entry.path.to_string_lossy()))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                    meta_row(strings.col_size, size_str, palette),
                    meta_row(strings.permissions, perms.to_string(), palette),
                ]
                .spacing(10);

                if let Some(modified) = entry.modified_at {
                    info_col = info_col.push(meta_row(
                        strings.modified,
                        strings.format_age(modified),
                        palette,
                    ));
                }
                if let Some(created) = entry.created_at {
                    info_col = info_col.push(meta_row(
                        strings.created,
                        strings.format_age(created),
                        palette,
                    ));
                }

                if let Some(tel) = &tab.selected_telemetry {
                    if let Some(mime) = &tel.mime_type {
                        info_col =
                            info_col.push(meta_row(strings.mime_type, mime.to_string(), palette));
                    }
                    if let Some(git_st) = &tel.git_status {
                        info_col = info_col.push(meta_row(
                            strings.git_status_label,
                            git_st.to_string(),
                            palette,
                        ));
                    }
                    if let Some(hash) = &tel.sha256_prefix {
                        info_col =
                            info_col.push(meta_row(strings.checksum, hash.to_string(), palette));
                    } else if !entry.kind.is_dir_like() {
                        info_col = info_col.push(meta_row(
                            strings.checksum,
                            strings.computing_hash.to_owned(),
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
        container(Space::new().width(1).height(Length::Fill)).style(styles::divider_line(palette)),
        drawer_panel,
    ]
    .height(Length::Fill)
    .into()
}

fn meta_row<'a>(label: &'a str, value: String, palette: ThemePalette) -> Element<'a, Message> {
    container(
        column![
            text(label)
                .size(10)
                .font(FONT_MONO)
                .color(palette.text_muted),
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
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let entry = tab.and_then(|t| t.selection.single_selected(&t.visible_entries));
    let snippet = tab
        .and_then(|t| t.selected_telemetry.as_ref())
        .and_then(|st| st.preview_snippet.as_deref());

    let title = entry
        .map(|e| sanitize_single_line(e.name.as_str()))
        .unwrap_or_else(|| strings.quicklook_title.to_owned());

    let img_handle = entry.and_then(|e| {
        app.quicklook_image
            .as_ref()
            .filter(|(p, _)| *p == e.path)
            .map(|(_, h)| h)
    });

    let body: Element<'a, Message> = if let Some(handle) = img_handle {
        container(
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fixed(360.0))
                .content_fit(ContentFit::Contain),
        )
        .width(Length::Fill)
        .padding(10)
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
        .into()
    } else if snippet.is_some() {
        let hl_theme = match app.config.theme {
            ThemePreset::ObsidianFlow => iced::highlighter::Theme::Base16Ocean,
            ThemePreset::AetherLight => iced::highlighter::Theme::InspiredGitHub,
        };
        let syntax_token = if app.quicklook_extension.is_empty() {
            "txt"
        } else {
            app.quicklook_extension.as_str()
        };
        container(
            text_editor(&app.quicklook_editor)
                .highlight(syntax_token, hl_theme)
                .font(FONT_MONO)
                .size(12)
                .height(Length::Fixed(360.0))
                .on_action(Message::QuickLookEditorAction),
        )
        .width(Length::Fill)
        .padding(8)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
        .into()
    } else if let Some(e) = entry {
        let icon_widget = render_entry_icon(app, e, palette, 56.0);
        column![
            icon_widget,
            text(sanitize_single_line(e.name.as_str()))
                .size(16)
                .font(FONT_UI_BOLD)
                .color(palette.text_primary),
            text(format_bytes(e.size_bytes).to_string())
                .size(13)
                .font(FONT_MONO)
                .color(palette.text_secondary),
            text(strings.binary_preview_hint)
                .size(12)
                .color(palette.text_muted),
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
        .width(Length::Fixed(680.0)),
    )
    .padding(20)
    .style(styles::modal_card(palette));

    modal_overlay_backdrop(card)
}

pub fn deep_search_modal<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let query_row = container(
        row![
            icon_search().size(15).color(palette.accent_primary),
            text_input(strings.search_placeholder, &app.deep_search_query)
                .id(deep_search_input_id())
                .on_input(Message::DeepSearchQueryChanged)
                .size(13)
                .padding([6, 8])
                .style(styles::search_input_style(palette)),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([4, 12])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 10.0));

    let results_body: Element<'a, Message> = if app.deep_search_loading {
        container(
            text(strings.searching)
                .size(12)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        )
        .width(Length::Fill)
        .height(Length::Fixed(300.0))
        .align_x(Center)
        .align_y(Center)
        .into()
    } else if app.deep_search_results.is_empty() {
        container(
            text(strings.deep_search_empty)
                .size(12)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .width(Length::Fill)
        .height(Length::Fixed(300.0))
        .align_x(Center)
        .align_y(Center)
        .into()
    } else {
        let mut list = column![]
            .spacing(4)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
                left: 0.0,
            });
        for hit in &app.deep_search_results {
            let icon = if hit.is_dir {
                icon_folder().size(15).color(palette.accent_primary)
            } else {
                icon_file_text().size(15).color(palette.text_secondary)
            };
            let hit_row = row![
                icon,
                container(
                    column![
                        text(sanitize_single_line(hit.name.as_str()))
                            .size(13)
                            .font(FONT_UI_SEMIBOLD)
                            .color(palette.text_primary)
                            .wrapping(text::Wrapping::None),
                        text(sanitize_single_line(hit.relative_path.as_str()))
                            .size(11)
                            .font(FONT_MONO)
                            .color(palette.text_muted)
                            .wrapping(text::Wrapping::None),
                    ]
                    .spacing(1),
                )
                .width(Length::Fill)
                .clip(true),
                text(format!("#{}", hit.score))
                    .size(10)
                    .font(FONT_MONO)
                    .color(palette.accent_primary)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(10)
            .align_y(Center);

            list = list.push(
                button(hit_row)
                    .width(Length::Fill)
                    .padding([8, 10])
                    .style(styles::ghost_button(palette, 8.0))
                    .on_press(Message::OpenSearchHit(hit.path.clone())),
            );
        }
        scrollable(list).height(Length::Fixed(320.0)).into()
    };

    let root_path_display = app
        .current_tab()
        .map(|t| sanitize_single_line(&t.current_path.to_string_lossy()))
        .unwrap_or_default();

    let card = container(
        column![
            row![
                column![
                    text(strings.deep_search_title)
                        .size(14)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.text_primary),
                    text(root_path_display)
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_muted)
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(2),
                space::horizontal(),
                button(
                    row![icon_x().size(14), text(strings.close).size(12)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([4, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::CloseDeepSearch),
            ]
            .align_y(Center),
            query_row,
            results_body,
        ]
        .spacing(12)
        .width(Length::Fixed(640.0)),
    )
    .padding(20)
    .style(styles::modal_card(palette));

    modal_overlay_backdrop(card)
}
