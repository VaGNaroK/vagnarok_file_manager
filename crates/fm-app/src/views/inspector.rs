use fm_core::{Strings, TabState, ThemePreset, format_bytes};
use fm_index::{DateFilter, FileTypeFilter, SizeFilter};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, Spacing, ThemePalette,
    styles,
};
use iced::Alignment::Center;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, stroke};
use iced::widget::{
    Space, button, column, container, image, pick_list, row, scrollable, space, text, text_editor,
    text_input,
};
use iced::{ContentFit, Element, Length, Padding, Point, Radians, Rectangle, Renderer, Theme};
use lucide_icons::iced::{icon_file_text, icon_folder, icon_search, icon_x};

use crate::app::{
    App, Message, TextInputTarget, command_palette_input_id, deep_search_content_input_id,
    deep_search_input_id, quick_jump_input_id, thumbnail_key_for_entry,
};
use crate::views::{
    modal_overlay_backdrop, render_entry_icon, sanitize_single_line, wrap_text_input_context_menu,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SearchOption<T: Copy + PartialEq> {
    value: T,
    label: &'static str,
}

impl<T: Copy + PartialEq> std::fmt::Display for SearchOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label)
    }
}

fn toggle_chip<'a>(
    label: &'a str,
    active: bool,
    msg: Message,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let fg = if active {
        palette.accent_primary
    } else {
        palette.text_muted
    };
    let bg = if active {
        palette.accent_surface
    } else {
        palette.surface_lowest
    };
    let border = if active {
        palette.accent_border
    } else {
        palette.border_strong
    };

    button(text(label).size(11).font(FONT_MONO_BOLD).color(fg))
        .padding([4, 8])
        .style(move |_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                color: border,
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: fg,
            ..Default::default()
        })
        .on_press(msg)
        .into()
}

struct ProgressRingProgram {
    phase: f32,
    track_color: iced::Color,
    arc_color: iced::Color,
}

impl canvas::Program<Message> for ProgressRingProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) / 2.0 - 3.0).max(4.0);

        let track = Path::circle(center, radius);
        frame.stroke(
            &track,
            Stroke::default()
                .with_color(self.track_color)
                .with_width(3.0),
        );

        let arc = Path::new(|b| {
            b.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: Radians(self.phase),
                end_angle: Radians(self.phase + std::f32::consts::PI * 1.25),
            });
        });
        frame.stroke(
            &arc,
            Stroke::default()
                .with_color(self.arc_color)
                .with_width(3.0)
                .with_line_cap(stroke::LineCap::Round),
        );

        vec![frame.into_geometry()]
    }
}

pub fn deep_search_modal<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let query_row = container(
        row![
            icon_search().size(15).color(palette.accent_primary),
            text(strings.search_name_label)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_muted)
                .width(Length::Fixed(116.0)),
            wrap_text_input_context_menu(
                text_input(strings.search_placeholder, &app.deep_search_query)
                    .id(deep_search_input_id())
                    .on_input(Message::DeepSearchQueryChanged)
                    .size(13)
                    .padding([6, 8])
                    .style(styles::search_input_style(palette))
                    .into(),
                app.main_window_id,
                TextInputTarget::DeepSearchInput,
                app.deep_search_query.clone(),
            ),
            toggle_chip(
                ".*",
                app.deep_search_name_regex,
                Message::DeepSearchToggleNameRegex,
                palette,
            ),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([4, 12])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 10.0));

    let content_row = container(
        row![
            icon_file_text().size(15).color(palette.accent_primary),
            text(strings.search_containing_label)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.accent_primary)
                .width(Length::Fixed(116.0)),
            wrap_text_input_context_menu(
                text_input(
                    strings.search_containing_placeholder,
                    &app.deep_search_content_query,
                )
                .id(deep_search_content_input_id())
                .on_input(Message::DeepSearchContentQueryChanged)
                .size(13)
                .padding([6, 8])
                .style(styles::search_input_style(palette))
                .into(),
                app.main_window_id,
                TextInputTarget::DeepSearchContentInput,
                app.deep_search_content_query.clone(),
            ),
            toggle_chip(
                "Aa",
                app.deep_search_content_case_sensitive,
                Message::DeepSearchToggleContentCase,
                palette,
            ),
            toggle_chip(
                ".*",
                app.deep_search_content_regex,
                Message::DeepSearchToggleContentRegex,
                palette,
            ),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([4, 12])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border_strong, 10.0));

    let size_options = vec![
        SearchOption {
            value: SizeFilter::Any,
            label: strings.search_any,
        },
        SearchOption {
            value: SizeFilter::Gt(1024 * 1024),
            label: "> 1 MB",
        },
        SearchOption {
            value: SizeFilter::Gt(10 * 1024 * 1024),
            label: "> 10 MB",
        },
        SearchOption {
            value: SizeFilter::Lt(100 * 1024),
            label: "< 100 KB",
        },
    ];
    let selected_size = size_options
        .iter()
        .find(|o| o.value == app.deep_search_size_filter)
        .copied();

    let date_options = vec![
        SearchOption {
            value: DateFilter::Any,
            label: strings.search_any,
        },
        SearchOption {
            value: DateFilter::Today,
            label: strings.search_today,
        },
        SearchOption {
            value: DateFilter::Last7Days,
            label: strings.search_last_7d,
        },
        SearchOption {
            value: DateFilter::Last30Days,
            label: strings.search_last_30d,
        },
    ];
    let selected_date = date_options
        .iter()
        .find(|o| o.value == app.deep_search_date_filter)
        .copied();

    let type_options = vec![
        SearchOption {
            value: FileTypeFilter::Any,
            label: strings.search_any,
        },
        SearchOption {
            value: FileTypeFilter::Code,
            label: strings.search_type_code,
        },
        SearchOption {
            value: FileTypeFilter::Documents,
            label: strings.search_type_docs,
        },
        SearchOption {
            value: FileTypeFilter::Images,
            label: strings.search_type_images,
        },
        SearchOption {
            value: FileTypeFilter::Audio,
            label: strings.search_type_audio,
        },
        SearchOption {
            value: FileTypeFilter::Video,
            label: strings.search_type_video,
        },
        SearchOption {
            value: FileTypeFilter::Archives,
            label: strings.search_type_archives,
        },
    ];
    let selected_type = type_options
        .iter()
        .find(|o| o.value == app.deep_search_type_filter)
        .copied();

    let filters_row = row![
        text(strings.search_size_label)
            .size(11)
            .color(palette.text_muted),
        pick_list(size_options, selected_size, |opt| {
            Message::DeepSearchSetSizeFilter(opt.value)
        })
        .text_size(11)
        .padding([4, 8])
        .style(styles::pick_list_style(palette)),
        text(strings.search_date_label)
            .size(11)
            .color(palette.text_muted),
        pick_list(date_options, selected_date, |opt| {
            Message::DeepSearchSetDateFilter(opt.value)
        })
        .text_size(11)
        .padding([4, 8])
        .style(styles::pick_list_style(palette)),
        text(strings.search_type_label)
            .size(11)
            .color(palette.text_muted),
        pick_list(type_options, selected_type, |opt| {
            Message::DeepSearchSetTypeFilter(opt.value)
        })
        .text_size(11)
        .padding([4, 8])
        .style(styles::pick_list_style(palette)),
        space::horizontal(),
        toggle_chip(
            strings.search_hidden_label,
            app.deep_search_include_hidden,
            Message::DeepSearchToggleIncludeHidden,
            palette,
        ),
    ]
    .spacing(8)
    .align_y(Center);

    let results_body: Element<'a, Message> = if app.deep_search_loading {
        container(
            column![
                text(strings.searching)
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.text_secondary),
                canvas::Canvas::new(ProgressRingProgram {
                    phase: app.deep_search_spinner_phase,
                    track_color: palette.border_strong,
                    arc_color: palette.accent_primary,
                })
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0)),
            ]
            .spacing(12)
            .align_x(Center),
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
            let size_label = if hit.is_dir {
                String::new()
            } else {
                hit.size_bytes
                    .map(format_bytes)
                    .unwrap_or_default()
                    .to_string()
            };
            let hit_header = row![
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
                text(size_label)
                    .size(10)
                    .font(FONT_MONO)
                    .color(palette.text_secondary)
                    .wrapping(text::Wrapping::None),
                text(format!("#{}", hit.score))
                    .size(10)
                    .font(FONT_MONO)
                    .color(palette.accent_primary)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(10)
            .align_y(Center);

            let mut hit_col = column![hit_header].spacing(4);
            if !hit.content_matches.is_empty() {
                let mut snippets_col = column![].spacing(2).padding(Padding {
                    top: 2.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 25.0,
                });
                for snippet in &hit.content_matches {
                    snippets_col = snippets_col.push(
                        row![
                            text(format!("L{}", snippet.line_number))
                                .size(11)
                                .font(FONT_MONO_BOLD)
                                .color(palette.accent_primary)
                                .width(Length::Fixed(42.0)),
                            text(sanitize_single_line(snippet.line_text.as_str()))
                                .size(11)
                                .font(FONT_MONO)
                                .color(palette.text_secondary)
                                .wrapping(text::Wrapping::None),
                        ]
                        .spacing(6)
                        .align_y(Center),
                    );
                }
                hit_col = hit_col.push(snippets_col);
            }

            list = list.push(
                button(hit_col)
                    .width(Length::Fill)
                    .padding([8, 10])
                    .style(styles::ghost_button(palette, 8.0))
                    .on_press(Message::OpenSearchHit(hit.path.clone())),
            );
        }
        scrollable(list).height(Length::Fixed(320.0)).into()
    };

    let root_path_display = app
        .deep_search_root_override
        .as_ref()
        .map(|p| sanitize_single_line(&p.to_string_lossy()))
        .or_else(|| {
            app.current_tab()
                .map(|t| sanitize_single_line(&t.current_path.to_string_lossy()))
        })
        .unwrap_or_default();

    let status_bar = row![
        text(format!("{} resultados", app.deep_search_results.len()))
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_muted),
        space::horizontal(),
        text(strings.search_streaming_hint)
            .size(11)
            .color(palette.status_success),
    ]
    .align_y(Center);

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
                button(text(strings.arch_minimize).size(12))
                    .padding([4, 10])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::MinimizeDeepSearch),
                button(
                    row![icon_x().size(14), text(strings.close).size(12)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([4, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::CloseDeepSearch),
            ]
            .spacing(8)
            .align_y(Center),
            query_row,
            content_row,
            filters_row,
            status_bar,
            results_body,
        ]
        .spacing(10)
        .width(Length::Fixed(720.0)),
    )
    .padding(20)
    .style(styles::modal_card(palette));

    modal_overlay_backdrop(card)
}

pub fn command_palette_modal<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let filtered = crate::command_palette::filter_commands(
        crate::command_palette::build_catalog(app, strings),
        &app.command_palette_query,
        &app.command_palette_recent,
    );
    let selected_idx = if filtered.is_empty() {
        0
    } else {
        app.command_palette_selected.min(filtered.len() - 1)
    };

    let query_row = container(
        row![
            text(">_")
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(palette.accent_primary),
            wrap_text_input_context_menu(
                text_input(strings.cmd_palette_placeholder, &app.command_palette_query)
                    .id(command_palette_input_id())
                    .on_input(Message::CommandPaletteQueryChanged)
                    .on_submit(Message::CommandPaletteExecuteSelected)
                    .size(13)
                    .padding([6, 8])
                    .style(styles::search_input_style(palette))
                    .into(),
                app.main_window_id,
                TextInputTarget::CommandPaletteInput,
                app.command_palette_query.clone(),
            ),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([4, 12])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 10.0));

    let body: Element<'a, Message> = if filtered.is_empty() {
        container(
            text(strings.cmd_palette_empty)
                .size(12)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .width(Length::Fill)
        .height(Length::Fixed(260.0))
        .align_x(Center)
        .align_y(Center)
        .into()
    } else {
        let mut list = column![].spacing(4).width(Length::Fill).padding(Padding {
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 0.0,
        });

        for (idx, entry) in filtered.into_iter().take(50).enumerate() {
            let is_selected = idx == selected_idx;
            let title_color = if !entry.enabled {
                palette.text_muted
            } else if is_selected {
                palette.accent_primary
            } else {
                palette.text_primary
            };

            let cat_badge = container(text(entry.category).size(10).font(FONT_MONO_BOLD).color(
                if is_selected {
                    palette.accent_primary
                } else {
                    palette.text_secondary
                },
            ))
            .padding([2, 7])
            .style(move |_| {
                styles::panel_surface(
                    if is_selected {
                        palette.accent_surface
                    } else {
                        palette.surface_high
                    },
                    if is_selected {
                        palette.accent_border
                    } else {
                        palette.border
                    },
                    5.0,
                )
            });

            let mut row_content = row![
                cat_badge,
                text(entry.title)
                    .size(13)
                    .font(if is_selected {
                        FONT_UI_SEMIBOLD
                    } else {
                        FONT_UI
                    })
                    .color(title_color)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Center);

            if !entry.shortcut.is_empty() {
                row_content = row_content.push(
                    container(
                        text(entry.shortcut)
                            .size(10)
                            .font(FONT_MONO)
                            .color(palette.text_muted),
                    )
                    .padding([2, 6])
                    .style(move |_| {
                        styles::panel_surface(palette.surface_low, palette.border_strong, 5.0)
                    }),
                );
            }

            let btn = button(row_content)
                .width(Length::Fill)
                .padding([8, 10])
                .style(move |theme, status| {
                    if is_selected {
                        iced::widget::button::Style {
                            background: Some(iced::Background::Color(palette.accent_surface)),
                            border: iced::Border {
                                color: palette.accent_border,
                                width: 1.0,
                                radius: 8.0.into(),
                            },
                            text_color: title_color,
                            ..Default::default()
                        }
                    } else {
                        styles::ghost_button(palette, 8.0)(theme, status)
                    }
                });

            let btn = if entry.enabled {
                btn.on_press(Message::CommandPaletteExecute(entry.id))
            } else {
                btn
            };

            list = list.push(btn);
        }

        scrollable(list).height(Length::Fixed(320.0)).into()
    };

    let card = container(
        column![
            row![
                text(strings.cmd_palette_title)
                    .size(14)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                container(
                    text("Ctrl+Shift+P")
                        .size(10)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                )
                .padding([3, 7])
                .style(move |_| {
                    styles::panel_surface(palette.surface_low, palette.border_strong, 5.0)
                }),
                button(
                    row![icon_x().size(14), text(strings.close).size(12)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([4, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::CloseCommandPalette),
            ]
            .spacing(8)
            .align_y(Center),
            query_row,
            body,
            row![
                text(strings.cmd_palette_hint)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted),
            ]
            .align_y(Center),
        ]
        .spacing(10)
        .width(Length::Fixed(620.0)),
    )
    .padding(18)
    .style(styles::modal_card(palette));

    modal_overlay_backdrop(card)
}

pub fn quick_jump_modal<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let selected_idx = if app.quick_jump_items.is_empty() {
        0
    } else {
        app.quick_jump_selected.min(app.quick_jump_items.len() - 1)
    };

    let query_row = container(
        row![
            icon_folder().size(15).color(palette.accent_primary),
            wrap_text_input_context_menu(
                text_input(strings.quick_jump_placeholder, &app.quick_jump_query)
                    .id(quick_jump_input_id())
                    .on_input(Message::QuickJumpQueryChanged)
                    .on_submit(Message::QuickJumpOpenSelected { new_tab: false })
                    .size(13)
                    .padding([6, 8])
                    .style(styles::search_input_style(palette))
                    .into(),
                app.main_window_id,
                TextInputTarget::QuickJumpInput,
                app.quick_jump_query.clone(),
            ),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([4, 12])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 10.0));

    let body: Element<'a, Message> = if app.quick_jump_items.is_empty() {
        container(
            text(strings.quick_jump_empty)
                .size(12)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .width(Length::Fill)
        .height(Length::Fixed(260.0))
        .align_x(Center)
        .align_y(Center)
        .into()
    } else {
        let mut list = column![].spacing(4).width(Length::Fill).padding(Padding {
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 0.0,
        });

        for (idx, item) in app.quick_jump_items.iter().enumerate() {
            let is_selected = idx == selected_idx;
            let title_color = if is_selected {
                palette.accent_primary
            } else {
                palette.text_primary
            };

            let src_badge = container(
                text(item.source.label(strings))
                    .size(10)
                    .font(FONT_MONO_BOLD)
                    .color(if is_selected {
                        palette.accent_primary
                    } else {
                        palette.text_secondary
                    }),
            )
            .padding([2, 7])
            .style(move |_| {
                styles::panel_surface(
                    if is_selected {
                        palette.accent_surface
                    } else {
                        palette.surface_high
                    },
                    if is_selected {
                        palette.accent_border
                    } else {
                        palette.border
                    },
                    5.0,
                )
            });

            let main_btn = button(
                row![
                    icon_folder().size(15).color(if is_selected {
                        palette.accent_primary
                    } else {
                        palette.text_secondary
                    }),
                    container(
                        column![
                            text(sanitize_single_line(item.name.as_str()))
                                .size(13)
                                .font(if is_selected {
                                    FONT_UI_SEMIBOLD
                                } else {
                                    FONT_UI
                                })
                                .color(title_color)
                                .wrapping(text::Wrapping::None),
                            text(sanitize_single_line(item.display_path.as_str()))
                                .size(11)
                                .font(FONT_MONO)
                                .color(palette.text_muted)
                                .wrapping(text::Wrapping::None),
                        ]
                        .spacing(1),
                    )
                    .width(Length::Fill)
                    .clip(true),
                    src_badge,
                ]
                .spacing(10)
                .align_y(Center),
            )
            .width(Length::Fill)
            .padding([8, 10])
            .style(move |theme, status| {
                if is_selected {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(palette.accent_surface)),
                        border: iced::Border {
                            color: palette.accent_border,
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        text_color: title_color,
                        ..Default::default()
                    }
                } else {
                    styles::ghost_button(palette, 8.0)(theme, status)
                }
            })
            .on_press(Message::QuickJumpOpenPath {
                path: item.path.clone(),
                new_tab: false,
            });

            let new_tab_btn = button(
                text(strings.quick_jump_new_tab)
                    .size(10)
                    .font(FONT_MONO)
                    .color(palette.text_secondary),
            )
            .padding([6, 8])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::QuickJumpOpenPath {
                path: item.path.clone(),
                new_tab: true,
            });

            list = list.push(row![main_btn, new_tab_btn].spacing(6).align_y(Center));
        }

        scrollable(list).height(Length::Fixed(320.0)).into()
    };

    let card = container(
        column![
            row![
                text(strings.quick_jump_title)
                    .size(14)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                container(
                    text("Ctrl+P")
                        .size(10)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                )
                .padding([3, 7])
                .style(move |_| {
                    styles::panel_surface(palette.surface_low, palette.border_strong, 5.0)
                }),
                button(
                    row![icon_x().size(14), text(strings.close).size(12)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([4, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::CloseQuickJump),
            ]
            .spacing(8)
            .align_y(Center),
            query_row,
            body,
            row![
                text(strings.quick_jump_hint)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted),
            ]
            .align_y(Center),
        ]
        .spacing(10)
        .width(Length::Fixed(640.0)),
    )
    .padding(18)
    .style(styles::modal_card(palette));

    modal_overlay_backdrop(card)
}

pub fn deep_search_minimized_pill<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let label = if app.deep_search_loading {
        strings.searching.to_owned()
    } else {
        format!(
            "{} ({})",
            strings.deep_search_title,
            app.deep_search_results.len()
        )
    };

    button(
        row![
            icon_search().size(13).color(palette.accent_primary),
            text(label)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([4, 10])
    .style(styles::pill_secondary_button(palette))
    .on_press(Message::RestoreDeepSearch)
    .into()
}
