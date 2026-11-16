use fm_core::{
    CopyMode, DIR_FILTER_SENTINEL, FileEntry, JobProgress, SelectionModifiers, SortDirection,
    SortField, Strings, TabState, ViewMode, format_bytes,
};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles,
};
use iced::Alignment::{self, Center};
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, overlay};
use iced::keyboard;
use iced::mouse;
use iced::widget::{
    Space, button, column, container, image, mouse_area, responsive, row, scrollable, space, stack,
    text,
};
use iced::{Color, ContentFit, Element, Event, Length, Padding, Rectangle, Size, Vector};
use lucide_icons::iced::{
    icon_arrow_up_down, icon_chevron_right, icon_clipboard_paste, icon_copy, icon_eye,
    icon_eye_off, icon_filter, icon_folder_tree, icon_info, icon_minus, icon_pencil, icon_plus,
    icon_rotate_ccw, icon_scissors, icon_star, icon_terminal, icon_trash_2, icon_x,
};

use crate::app::{
    App, Message, canvas_scroll_id, selected_id_set_contains, thumbnail_key_for_entry,
};
use crate::views::modals::format_transfer_summary;
use crate::views::virtualized::{
    DETAIL_ROW_HEIGHT, DETAIL_ROW_STEP, GRID_BAND_SPACING, GRID_GAP, GridBand,
    build_tab_grid_bands, clamp_grid_card_width, compute_band_window_with_row_height,
    compute_fixed_window, grid_card_height, grid_columns_for_width,
};
use crate::views::{entry_icon, sanitize_single_line};

pub fn view<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let Some(tab) = tab else {
        return container(text(strings.no_active_tab))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let mut header_stack = column![action_bar(app, tab, palette, strings)].spacing(14);
    if app.config.show_hero_card {
        header_stack = header_stack.push(hero_card(tab, palette, strings));
    }
    if let Some(job) = &app.active_transfer {
        header_stack = header_stack.push(transfer_banner(job, palette, strings));
    }

    let body: Element<'a, Message> = if tab.is_loading {
        container(
            text(strings.loading)
                .size(13)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .into()
    } else if let Some(err) = &tab.last_error {
        error_banner(err, tab.can_go_back(), palette, strings)
    } else if tab.visible_entries.is_empty() {
        container(
            text(strings.empty_dir)
                .size(13)
                .font(FONT_UI)
                .color(palette.text_muted),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .into()
    } else {
        match tab.view_mode {
            ViewMode::Grid => grid_view(app, tab, palette, strings),
            ViewMode::DetailTable => detail_table_view(app, tab, palette, strings),
            ViewMode::Columns => columns_view(app, tab, palette, strings),
        }
    };

    let body = mouse_area(body).on_right_press(Message::OpenCanvasContextMenu);

    let panel: Element<'a, Message> = container(
        column![header_stack, body]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(Padding::from([14, 20]))
    .style(styles::canvas_container(palette))
    .into();

    let can_zoom = tab.view_mode == ViewMode::Grid
        && app.active_dialog.is_none()
        && app.context_menu.is_none()
        && app.breadcrumb_menu.is_none()
        && !app.quicklook_open
        && !app.prefs_open;

    Element::new(CtrlWheelZoom {
        content: panel,
        can_zoom,
        intercept: can_zoom && app.current_modifiers.control(),
        on_wheel: Box::new(Message::CtrlWheelZoom),
    })
}

fn error_banner<'a>(
    err: &'a str,
    can_go_back: bool,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mut actions = row![
        button(text(strings.retry).size(12).font(FONT_UI_SEMIBOLD))
            .padding([6, 14])
            .style(styles::pill_primary_button(palette))
            .on_press(Message::RefreshActiveTab),
    ]
    .spacing(8);

    if can_go_back {
        actions = actions.push(
            button(text(strings.go_back).size(12))
                .padding([6, 14])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::NavigateBack),
        );
    }

    container(
        container(
            column![
                text(err)
                    .size(13)
                    .font(FONT_MONO)
                    .color(palette.status_error),
                actions,
            ]
            .spacing(12)
            .align_x(Center),
        )
        .padding(20)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 12.0)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Center)
    .align_y(Center)
    .into()
}

fn action_bar<'a>(
    app: &'a App,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mut left_info = row![
        text(sanitize_single_line(tab.title.as_str()))
            .size(14)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.text_primary)
            .wrapping(text::Wrapping::None),
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
        row![
            icon_plus().size(14),
            text(strings.new_item).size(12).font(FONT_UI_SEMIBOLD)
        ]
        .spacing(4)
        .align_y(Center),
    )
    .padding([6, 14])
    .style(styles::pill_primary_button(palette))
    .on_press(Message::OpenNewMenu);

    let is_in_trash = app
        .places
        .trash
        .as_ref()
        .is_some_and(|tr| tab.current_path.starts_with(tr));
    let sel_count = tab.selection.count();
    let mut ops_row = row![].spacing(4).align_y(Center);
    if is_in_trash {
        if sel_count > 0 {
            ops_row = ops_row.push(
                button(
                    row![
                        icon_rotate_ccw().size(13).color(palette.accent_primary),
                        text(strings.restore_item)
                            .size(12)
                            .color(palette.accent_primary),
                    ]
                    .spacing(5)
                    .align_y(Center),
                )
                .padding([6, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::RestoreFromTrash),
            );
        }
        ops_row = ops_row.push(
            button(
                row![
                    icon_trash_2().size(13).color(palette.status_error),
                    text(strings.empty_trash)
                        .size(12)
                        .color(palette.status_error),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([6, 10])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::RequestEmptyTrash),
        );
    }
    if sel_count > 0 {
        ops_row = ops_row
            .push(
                button(icon_copy().size(14).color(palette.text_primary))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CopySelected),
            )
            .push(
                button(icon_scissors().size(14).color(palette.text_primary))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CutSelected),
            );
        if sel_count == 1 {
            ops_row = ops_row.push(
                button(icon_pencil().size(14).color(palette.text_primary))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::OpenRenameDialog),
            );
        }
        ops_row = ops_row.push(
            button(icon_trash_2().size(14).color(palette.status_error))
                .padding([6, 8])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::RequestDelete(false)),
        );
    }

    if let Some(clip) = &app.clipboard {
        let label = format!("{} ({})", strings.paste, clip.paths.len());
        let paste_color = match clip.mode {
            CopyMode::Copy => palette.accent_primary,
            CopyMode::Move => palette.status_warning,
        };
        ops_row = ops_row.push(
            button(
                row![
                    icon_clipboard_paste().size(13).color(paste_color),
                    text(label).size(12).color(paste_color),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([6, 10])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::PasteClipboard),
        );
    }

    let is_fav = app.config.favorites.contains(&tab.current_path);
    let fav_btn = button(icon_star().size(15).color(if is_fav {
        palette.accent_tertiary
    } else {
        palette.text_secondary
    }))
    .padding([6, 8])
    .style(styles::ghost_button(palette, 9999.0))
    .on_press(Message::ToggleFavoriteActive);

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

    let filter_label = match tab.extension_filter.as_deref() {
        None => strings.filter_ext.to_owned(),
        Some(DIR_FILTER_SENTINEL) => {
            format!("{}: {}", strings.filter_ext, strings.directories_only)
        }
        Some(ext) => format!("{}: .{ext}", strings.filter_ext),
    };
    let filter_active = tab.extension_filter.is_some();
    let filter_btn = button(
        row![
            icon_filter().size(13).color(if filter_active {
                palette.accent_primary
            } else {
                palette.text_secondary
            }),
            text(filter_label).size(12).color(if filter_active {
                palette.accent_primary
            } else {
                palette.text_primary
            }),
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([6, 12])
    .style(styles::pill_secondary_button(palette))
    .on_press(Message::OpenFilterMenu);

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

    let terminal_btn = button(
        row![
            icon_terminal().size(13),
            text(strings.open_terminal).size(12),
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([6, 10])
    .style(styles::pill_secondary_button(palette))
    .on_press(Message::OpenTerminal);

    let info_btn = button(icon_info().size(16).color(if app.config.show_inspector {
        palette.accent_primary
    } else {
        palette.text_secondary
    }))
    .padding([6, 8])
    .style(styles::ghost_button(palette, 9999.0))
    .on_press(Message::ToggleInspector);

    let mut toolbar_right = row![new_btn, ops_row].spacing(8).align_y(Center);

    if tab.view_mode == ViewMode::Grid {
        let card_px = clamp_grid_card_width(app.config.grid_card_width).round() as u32;
        let group_active = app.config.grid_group_folders;
        let group_btn = button(icon_folder_tree().size(13).color(if group_active {
            palette.accent_primary
        } else {
            palette.text_secondary
        }))
        .padding([6, 8])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::ToggleGridGroupFolders);

        let size_controls = container(
            row![
                button(icon_minus().size(12).color(palette.text_primary))
                    .padding([4, 6])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::StepGridCardWidth(-16.0)),
                button(
                    text(format!("{card_px}px"))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_secondary),
                )
                .padding([3, 6])
                .style(styles::ghost_button(palette, 6.0))
                .on_press(Message::ResetGridCardWidth),
                button(icon_plus().size(12).color(palette.text_primary))
                    .padding([4, 6])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::StepGridCardWidth(16.0)),
            ]
            .spacing(2)
            .align_y(Center),
        )
        .padding(2)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0));

        toolbar_right = toolbar_right.push(group_btn).push(size_controls);
    }

    toolbar_right = toolbar_right
        .push(sort_btn)
        .push(sort_dir_btn)
        .push(filter_btn)
        .push(hidden_btn)
        .push(terminal_btn)
        .push(fav_btn)
        .push(info_btn);

    row![left_info, space::horizontal(), toolbar_right]
        .align_y(Center)
        .into()
}

fn transfer_banner<'a>(
    job: &'a JobProgress,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let ratio = if job.bytes_total == 0 {
        0.0
    } else {
        (job.bytes_done as f32 / job.bytes_total as f32).clamp(0.0, 1.0)
    };
    let filled = (ratio * 1000.0).round() as u16;
    let remaining = 1000u16.saturating_sub(filled);

    let bar = container(
        row![
            container(Space::new().width(Length::Fill).height(4))
                .width(Length::FillPortion(filled.max(1)))
                .style(move |_| styles::panel_surface(
                    palette.accent_primary,
                    Color::TRANSPARENT,
                    9999.0
                )),
            Space::new()
                .width(Length::FillPortion(remaining.max(1)))
                .height(4),
        ]
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(1)
    .style(move |_| styles::panel_surface(palette.surface_high, palette.border, 9999.0));

    let mode_tag = match job.mode {
        CopyMode::Copy => strings.copy,
        CopyMode::Move => strings.cut,
    };
    let summary = format_transfer_summary(job.bytes_done, job.bytes_total);

    container(
        column![
            row![
                text(format!("{mode_tag}: {}", job.current_item))
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                text(format!(
                    "{summary} ({}/{})",
                    job.items_done, job.items_total
                ))
                .size(11)
                .font(FONT_MONO)
                .color(palette.accent_primary),
                button(icon_x().size(13))
                    .padding([2, 6])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::CancelTransfer),
            ]
            .spacing(8)
            .align_y(Center),
            bar,
        ]
        .spacing(6),
    )
    .width(Length::Fill)
    .padding([10, 14])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.accent_border, 10.0))
    .into()
}

fn hero_card<'a>(
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let stats = tab.telemetry.folder_stats;
    let folder_name_raw = tab
        .current_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| tab.current_path.to_string_lossy().into_owned());
    let folder_name = sanitize_single_line(&folder_name_raw);
    let path_display = sanitize_single_line(&tab.current_path.to_string_lossy());
    let fs_badge = tab.fs_type.as_deref().unwrap_or(strings.root_fs);

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
                container(
                    text(folder_name)
                        .size(18)
                        .font(FONT_UI_BOLD)
                        .color(palette.text_primary)
                        .wrapping(text::Wrapping::None),
                )
                .clip(true),
                container(
                    text(fs_badge)
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
            single_line_label(
                text(path_display)
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.text_secondary),
            ),
        ]
        .spacing(3)
        .width(Length::Fill),
    ]
    .spacing(14)
    .width(Length::Fill)
    .align_y(Center);

    let (size_label, size_value) = if let Some(rs) = tab.telemetry.recursive_size {
        let formatted = format_bytes(rs.total_bytes).to_string();
        let display = if rs.truncated {
            format!("≈ {formatted}")
        } else {
            formatted
        };
        (strings.recursive_size_label, display)
    } else {
        (
            strings.total_size,
            format_bytes(stats.total_direct_bytes).to_string(),
        )
    };

    let metrics = row![
        metric_block(
            strings.folders,
            strings.format_folders_count(stats.dir_count),
            palette.text_primary,
            palette,
        ),
        metric_block(
            strings.files,
            strings.format_files_count(stats.file_count),
            palette.text_primary,
            palette,
        ),
        metric_block(size_label, size_value, palette.accent_primary, palette),
    ]
    .spacing(28)
    .align_y(Center);

    container(row![left, metrics].spacing(18).align_y(Center))
        .width(Length::Fill)
        .padding([14, 20])
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
        text(value).size(13).font(FONT_MONO_BOLD).color(value_color),
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
    let group_folders = app.config.grid_group_folders;
    let modifiers = SelectionModifiers {
        ctrl: app.current_modifiers.control(),
        shift: app.current_modifiers.shift(),
    };
    let scroll_y = app.canvas_scroll_y;
    let viewport_h = app.canvas_viewport_height;
    let card_width = clamp_grid_card_width(app.config.grid_card_width);
    let card_height = grid_card_height(card_width);
    let preview_box = ((card_width - 32.0) * 0.58).round().clamp(44.0, 128.0);
    let label_chars = ((card_width - 16.0) / 7.2).round().clamp(12.0, 34.0) as usize;

    responsive(move |size| {
        let fit_cols = grid_columns_for_width(size.width, card_width);
        let bands = build_tab_grid_bands(&tab.visible_entries, fit_cols, group_folders, strings);
        let win =
            compute_band_window_with_row_height(&bands, card_height, scroll_y, viewport_h, 280.0);

        let max_row_items = bands
            .iter()
            .filter_map(|b| match b {
                GridBand::Items(chunk) => Some(chunk.len()),
                GridBand::SectionHeader { .. } => None,
            })
            .max()
            .unwrap_or(0);
        let active_cols = if max_row_items > 0 && max_row_items < fit_cols {
            max_row_items
        } else {
            fit_cols
        };
        let grid_block_width =
            active_cols as f32 * card_width + active_cols.saturating_sub(1) as f32 * GRID_GAP;
        let min_header_w = 300.0_f32.min(size.width.max(card_width));
        let block_width = grid_block_width
            .max(min_header_w)
            .min(size.width.max(card_width));
        let center_sparse_rows = max_row_items > 0 && max_row_items < fit_cols;

        let mut list = column![]
            .spacing(GRID_BAND_SPACING)
            .width(Length::Fixed(block_width));
        if win.top_spacer > 0.0 {
            list = list.push(Space::new().height(win.top_spacer));
        }

        for band in &bands[win.range] {
            match band {
                GridBand::SectionHeader {
                    title,
                    count,
                    perms,
                } => {
                    list = list.push(section_title_bar(title, *count, perms, palette));
                }
                GridBand::Items(chunk) => {
                    let mut r = row![].spacing(GRID_GAP);
                    for &(idx, entry) in chunk {
                        let thumb = app.gpu_thumbnails.get(&thumbnail_key_for_entry(entry));
                        r = r.push(grid_item_card(
                            idx,
                            entry,
                            selected_id_set_contains(tab, entry.id),
                            modifiers,
                            thumb,
                            card_width,
                            card_height,
                            preview_box,
                            label_chars,
                            strings,
                            palette,
                        ));
                    }
                    let row_wrap =
                        container(r)
                            .width(Length::Fill)
                            .align_x(if center_sparse_rows {
                                Center
                            } else {
                                Alignment::Start
                            });
                    list = list.push(row_wrap);
                }
            }
        }

        if win.bottom_spacer > 0.0 {
            list = list.push(Space::new().height(win.bottom_spacer));
        }

        let centered_canvas = container(list).width(Length::Fill).align_x(Center);

        scrollable(centered_canvas)
            .id(canvas_scroll_id())
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(|vp| Message::CanvasScrolled {
                offset_y: vp.absolute_offset().y,
                viewport_height: vp.bounds().height,
                viewport_width: vp.bounds().width,
            })
            .into()
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
    .height(24)
    .align_y(Center)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn grid_item_card<'a>(
    index: usize,
    entry: &'a FileEntry,
    selected: bool,
    modifiers: SelectionModifiers,
    thumb: Option<&'a image::Handle>,
    card_width: f32,
    card_height: f32,
    preview_box: f32,
    label_chars: usize,
    strings: Strings,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let preview_widget: Element<'a, Message> = if let Some(handle) = thumb {
        image(handle.clone())
            .width(preview_box - 4.0)
            .height(preview_box - 4.0)
            .content_fit(ContentFit::Contain)
            .into()
    } else {
        let icon_px = (preview_box * 0.56).clamp(26.0, 68.0);
        let (icon_widget, _color) = entry_icon(entry, palette, icon_px);
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
                container(
                    text(badge.as_str())
                        .size(9)
                        .font(FONT_MONO_BOLD)
                        .color(badge_fg),
                )
                .padding([1, 5])
                .style(styles::badge_container(
                    badge_bg,
                    badge_fg,
                    palette.border,
                    4.0,
                )),
            );
        }
        icon_box_col.into()
    };

    let sub_label = if entry.kind.is_dir_like() {
        entry
            .child_count
            .map(|c| strings.format_items_count(c as usize))
            .unwrap_or_else(|| strings.dir_badge.to_owned())
    } else {
        format_bytes(entry.size_bytes).to_string()
    };

    let display_name = truncate_label(&sanitize_single_line(&entry.name), label_chars);
    let name_widget: Element<'a, Message> = if selected {
        container(
            text(display_name)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.accent_primary)
                .wrapping(text::Wrapping::None),
        )
        .padding([2, 8])
        .height(22)
        .align_y(Center)
        .style(styles::badge_container(
            palette.accent_surface,
            palette.accent_primary,
            palette.accent_border,
            9999.0,
        ))
        .into()
    } else {
        container(
            text(display_name)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None),
        )
        .padding([2, 8])
        .height(22)
        .align_y(Center)
        .into()
    };

    let card_content = column![
        container(preview_widget)
            .width(preview_box)
            .height(preview_box)
            .align_x(Center)
            .align_y(Center),
        name_widget,
        text(sub_label).size(11).font(FONT_MONO).color(if selected {
            palette.accent_primary
        } else {
            palette.text_secondary
        }),
    ]
    .spacing(4)
    .align_x(Center)
    .width(Length::Fill);

    let card_btn = button(card_content)
        .width(Length::Fixed(card_width))
        .height(Length::Fixed(card_height))
        .padding([8, 6])
        .style(styles::entry_card_button(palette, selected, 14.0))
        .on_press(Message::SelectEntry { index, modifiers });

    let card_area = mouse_area(card_btn).on_right_press(Message::OpenEntryContextMenu(index));

    let grip_visual = container(Space::new().width(3).height(18)).style(move |_| {
        styles::panel_surface(
            if selected {
                palette.accent_border
            } else {
                palette.border
            },
            Color::TRANSPARENT,
            9999.0,
        )
    });
    let resize_grip = container(
        mouse_area(
            container(grip_visual)
                .width(10)
                .height(28)
                .align_x(Center)
                .align_y(Center),
        )
        .interaction(mouse::Interaction::ResizingHorizontally)
        .on_press(Message::StartGridResize),
    )
    .width(Length::Fixed(card_width))
    .height(Length::Fixed(card_height))
    .padding(Padding {
        top: 0.0,
        right: 2.0,
        bottom: 6.0,
        left: 0.0,
    })
    .align_x(Alignment::End)
    .align_y(Alignment::End);

    stack![card_area, resize_grip]
        .width(Length::Fixed(card_width))
        .height(Length::Fixed(card_height))
        .into()
}

fn sort_header_button<'a>(
    label: &'a str,
    field: SortField,
    active_sort: fm_core::SortSpec,
    portion: u16,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let is_active = active_sort.field == field;
    let display = if is_active {
        let arrow = match active_sort.direction {
            SortDirection::Ascending => "↑",
            SortDirection::Descending => "↓",
        };
        format!("{label} {arrow}")
    } else {
        label.to_owned()
    };

    button(
        text(display)
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(if is_active {
                palette.accent_primary
            } else {
                palette.text_muted
            }),
    )
    .width(Length::FillPortion(portion))
    .padding([2, 4])
    .style(styles::ghost_button(palette, 6.0))
    .on_press(Message::SortByField(field))
    .into()
}

fn col_resize_handle<'a>(divider_idx: usize, palette: ThemePalette) -> Element<'a, Message> {
    let grip = container(
        container(Space::new().width(1).height(14))
            .style(move |_| styles::panel_surface(palette.border, Color::TRANSPARENT, 0.0)),
    )
    .width(8)
    .height(22)
    .align_x(Center)
    .align_y(Center);

    mouse_area(grip)
        .interaction(mouse::Interaction::ResizingHorizontally)
        .on_press(Message::StartColumnResize(divider_idx))
        .into()
}

fn portion(weight: f32) -> u16 {
    (weight * 10.0).round().clamp(50.0, 9000.0) as u16
}

fn single_line_label<'a>(
    label: iced::widget::text::Text<'a, iced::Theme, iced::Renderer>,
) -> Element<'a, Message> {
    container(label.wrapping(text::Wrapping::None))
        .width(Length::Fill)
        .clip(true)
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
    let w = app.table_col_weights;
    let p = [
        portion(w[0]),
        portion(w[1]),
        portion(w[2]),
        portion(w[3]),
        portion(w[4]),
    ];

    let sticky_header = container(
        row![
            sort_header_button(strings.col_name, SortField::Name, tab.sort, p[0], palette),
            col_resize_handle(0, palette),
            sort_header_button(strings.col_size, SortField::Size, tab.sort, p[1], palette),
            col_resize_handle(1, palette),
            sort_header_button(strings.col_type, SortField::Kind, tab.sort, p[2], palette),
            col_resize_handle(2, palette),
            text(strings.col_perms)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted)
                .width(Length::FillPortion(p[3])),
            col_resize_handle(3, palette),
            sort_header_button(
                strings.col_modified,
                SortField::Modified,
                tab.sort,
                p[4],
                palette
            ),
        ]
        .align_y(Center)
        .width(Length::Fill)
        .padding([4, 8]),
    )
    .width(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0));

    let win = compute_fixed_window(
        tab.visible_entries.len(),
        DETAIL_ROW_STEP,
        app.canvas_scroll_y,
        app.canvas_viewport_height,
        6,
    );

    let mut rows = column![].spacing(4).width(Length::Fill);
    if win.top_spacer > 0.0 {
        rows = rows.push(Space::new().height(win.top_spacer));
    }

    for idx in win.range {
        let entry = &tab.visible_entries[idx];
        let selected = selected_id_set_contains(tab, entry.id);
        let (icon_widget, _) = entry_icon(entry, palette, 16.0);
        let size_txt = if entry.kind.is_dir_like() {
            entry
                .child_count
                .map(|c| strings.format_items_count(c as usize))
                .unwrap_or_else(|| "—".to_owned())
        } else {
            format_bytes(entry.size_bytes).to_string()
        };
        let kind_txt = if entry.kind.is_dir_like() {
            strings.dir_badge
        } else {
            entry.extension.as_deref().unwrap_or(strings.file_badge)
        };
        let perms = entry.permissions.format_compact(entry.kind.is_dir_like());
        let age_txt = entry
            .modified_at
            .map(|m| strings.format_age(m))
            .unwrap_or_else(|| "—".to_owned());

        let row_content = row![
            row![
                icon_widget,
                single_line_label(
                    text(sanitize_single_line(entry.name.as_str()))
                        .size(13)
                        .font(if selected { FONT_UI_SEMIBOLD } else { FONT_UI })
                        .color(palette.text_primary),
                ),
            ]
            .spacing(10)
            .align_y(Center)
            .width(Length::FillPortion(p[0])),
            Space::new().width(8),
            container(
                text(size_txt)
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.text_secondary)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::FillPortion(p[1]))
            .clip(true),
            Space::new().width(8),
            container(
                text(kind_txt)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::FillPortion(p[2]))
            .clip(true),
            Space::new().width(8),
            container(
                text(perms.as_str().to_owned())
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.text_secondary)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::FillPortion(p[3]))
            .clip(true),
            Space::new().width(8),
            container(
                text(age_txt)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::FillPortion(p[4]))
            .clip(true),
        ]
        .width(Length::Fill)
        .align_y(Center);

        let row_btn = button(row_content)
            .width(Length::Fill)
            .height(DETAIL_ROW_HEIGHT)
            .padding([6, 12])
            .style(styles::entry_card_button(palette, selected, 8.0))
            .on_press(Message::SelectEntry {
                index: idx,
                modifiers,
            });

        rows = rows.push(mouse_area(row_btn).on_right_press(Message::OpenEntryContextMenu(idx)));
    }

    if win.bottom_spacer > 0.0 {
        rows = rows.push(Space::new().height(win.bottom_spacer));
    }

    let scroll = scrollable(rows)
        .id(canvas_scroll_id())
        .height(Length::Fill)
        .on_scroll(|vp| Message::CanvasScrolled {
            offset_y: vp.absolute_offset().y,
            viewport_height: vp.bounds().height,
            viewport_width: vp.bounds().width,
        });

    column![sticky_header, scroll]
        .spacing(6)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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

    let mut cols_row = row![].spacing(12).height(Length::Fill);

    if app.column_ancestors.is_empty() {
        let parent_name = tab
            .current_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".to_owned());

        let col_fallback = container(
            column![
                button(
                    text(parent_name)
                        .size(11)
                        .font(FONT_MONO_BOLD)
                        .color(palette.text_muted),
                )
                .padding([3, 6])
                .style(styles::ghost_button(palette, 6.0))
                .on_press(Message::NavigateUp),
                container(
                    row![
                        single_line_label(
                            text(sanitize_single_line(tab.title.as_str()))
                                .size(13)
                                .font(FONT_UI_SEMIBOLD)
                                .color(palette.accent_primary),
                        ),
                        icon_chevron_right().size(14).color(palette.accent_primary),
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .padding([6, 8])
                .style(move |_| styles::panel_surface(
                    palette.accent_surface,
                    palette.accent_border,
                    8.0
                )),
            ]
            .spacing(8),
        )
        .width(Length::Fixed(200.0))
        .height(Length::Fill)
        .padding(12)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0));

        cols_row = cols_row.push(col_fallback);
    } else {
        for (anc_path, anc_entries) in &app.column_ancestors {
            let anc_title_raw = anc_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| anc_path.to_string_lossy().into_owned());
            let anc_title = sanitize_single_line(&anc_title_raw);
            let mut list = column![
                button(single_line_label(
                    text(anc_title)
                        .size(11)
                        .font(FONT_MONO_BOLD)
                        .color(palette.text_muted),
                ),)
                .padding([2, 4])
                .style(styles::ghost_button(palette, 4.0))
                .on_press(Message::NavigateTo(anc_path.clone())),
            ]
            .spacing(4);

            for child in anc_entries.iter().take(250) {
                let is_on_active_chain = tab.current_path.starts_with(&child.path);
                let (c_icon, _) = entry_icon(child, palette, 14.0);
                let mut r = row![
                    c_icon,
                    single_line_label(
                        text(sanitize_single_line(child.name.as_str()))
                            .size(12)
                            .font(if is_on_active_chain {
                                FONT_UI_SEMIBOLD
                            } else {
                                FONT_UI
                            }),
                    ),
                ]
                .spacing(6)
                .align_y(Center);
                if child.kind.is_dir_like() {
                    r = r.push(icon_chevron_right().size(12).color(palette.text_muted));
                }
                let target_nav = if child.kind.is_dir_like() {
                    child.path.clone()
                } else {
                    anc_path.clone()
                };
                list = list.push(
                    button(r)
                        .width(Length::Fill)
                        .height(26)
                        .padding([4, 8])
                        .style(styles::entry_card_button(palette, is_on_active_chain, 6.0))
                        .on_press(Message::NavigateTo(target_nav)),
                );
            }

            let anc_panel = container(scrollable(list).height(Length::Fill))
                .width(Length::Fixed(210.0))
                .height(Length::Fill)
                .padding(10)
                .style(move |_| {
                    styles::panel_surface(palette.surface_lowest, palette.border, 10.0)
                });

            cols_row = cols_row.push(anc_panel);
        }
    }

    let win = compute_fixed_window(
        tab.visible_entries.len(),
        32.0,
        app.canvas_scroll_y,
        app.canvas_viewport_height,
        6,
    );

    let mut active_col = column![single_line_label(
        text(sanitize_single_line(tab.title.as_str()))
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(palette.text_muted),
    ),]
    .spacing(4);

    if win.top_spacer > 0.0 {
        active_col = active_col.push(Space::new().height(win.top_spacer));
    }

    for idx in win.range {
        let entry = &tab.visible_entries[idx];
        let selected = selected_id_set_contains(tab, entry.id);
        let (icon_widget, _) = entry_icon(entry, palette, 15.0);
        let mut r = row![
            icon_widget,
            single_line_label(
                text(sanitize_single_line(entry.name.as_str()))
                    .size(13)
                    .font(if selected { FONT_UI_SEMIBOLD } else { FONT_UI })
                    .color(palette.text_primary),
            ),
        ]
        .spacing(8)
        .align_y(Center);
        if entry.kind.is_dir_like() {
            r = r.push(icon_chevron_right().size(13).color(palette.text_muted));
        }
        let item_btn = button(r)
            .width(Length::Fill)
            .height(28)
            .padding([4, 8])
            .style(styles::entry_card_button(palette, selected, 6.0))
            .on_press(Message::SelectEntry {
                index: idx,
                modifiers,
            });
        active_col = active_col
            .push(mouse_area(item_btn).on_right_press(Message::OpenEntryContextMenu(idx)));
    }

    if win.bottom_spacer > 0.0 {
        active_col = active_col.push(Space::new().height(win.bottom_spacer));
    }

    let col2 = container(
        scrollable(active_col)
            .id(canvas_scroll_id())
            .height(Length::Fill)
            .on_scroll(|vp| Message::CanvasScrolled {
                offset_y: vp.absolute_offset().y,
                viewport_height: vp.bounds().height,
                viewport_width: vp.bounds().width,
            }),
    )
    .width(Length::Fixed(app.columns_panel_width))
    .height(Length::Fill)
    .padding(12)
    .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0));

    let col_divider = mouse_area(
        container(
            container(Space::new().width(2).height(36))
                .style(move |_| styles::panel_surface(palette.border, Color::TRANSPARENT, 9999.0)),
        )
        .width(8)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center),
    )
    .interaction(mouse::Interaction::ResizingHorizontally)
    .on_press(Message::StartColumnsPanelResize);

    let col3: Element<'a, Message> = if let Some(selected) =
        tab.selection.single_selected(&tab.visible_entries)
    {
        if selected.kind.is_dir_like() {
            let sub_content: Element<'a, Message> = match &app.column_preview {
                Some((dir_path, children)) if dir_path == &selected.path => {
                    if children.is_empty() {
                        container(
                            text(strings.empty_subdir)
                                .size(12)
                                .color(palette.text_muted),
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Center)
                        .align_y(Center)
                        .into()
                    } else {
                        let mut sub_list = column![single_line_label(
                            text(sanitize_single_line(selected.name.as_str()))
                                .size(11)
                                .font(FONT_MONO_BOLD)
                                .color(palette.text_muted),
                        ),]
                        .spacing(4);
                        for child in children.iter().take(200) {
                            let (c_icon, _) = entry_icon(child, palette, 14.0);
                            let mut r = row![
                                c_icon,
                                single_line_label(
                                    text(sanitize_single_line(child.name.as_str()))
                                        .size(12)
                                        .color(palette.text_primary),
                                ),
                            ]
                            .spacing(8)
                            .align_y(Center);
                            if child.kind.is_dir_like() {
                                r = r.push(icon_chevron_right().size(12).color(palette.text_muted));
                            } else {
                                r = r.push(
                                    text(format_bytes(child.size_bytes).to_string())
                                        .size(11)
                                        .font(FONT_MONO)
                                        .color(palette.text_muted)
                                        .wrapping(text::Wrapping::None),
                                );
                            }
                            let target_path = if child.kind.is_dir_like() {
                                child.path.clone()
                            } else {
                                selected.path.clone()
                            };
                            sub_list = sub_list.push(
                                button(r)
                                    .width(Length::Fill)
                                    .height(26)
                                    .padding([4, 8])
                                    .style(styles::ghost_button(palette, 6.0))
                                    .on_press(Message::NavigateTo(target_path)),
                            );
                        }
                        scrollable(sub_list).height(Length::Fill).into()
                    }
                }
                _ => container(
                    text(strings.loading)
                        .size(12)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Center)
                .align_y(Center)
                .into(),
            };

            container(sub_content)
                .width(Length::Fixed(app.columns_panel_width.max(270.0)))
                .height(Length::Fill)
                .padding(12)
                .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0))
                .into()
        } else {
            let preview_visual: Element<'a, Message> =
                if let Some(handle) = app.gpu_thumbnails.get(&thumbnail_key_for_entry(selected)) {
                    image(handle.clone())
                        .width(96)
                        .height(96)
                        .content_fit(ContentFit::Contain)
                        .into()
                } else {
                    let (icon_widget, _) = entry_icon(selected, palette, 42.0);
                    icon_widget.into()
                };

            let mut preview_col = column![
                preview_visual,
                text(sanitize_single_line(selected.name.as_str()))
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
            .align_x(Center);

            if let Some(tel) = &tab.selected_telemetry
                && let Some(mime) = &tel.mime_type
            {
                preview_col = preview_col.push(
                    text(mime.as_str())
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.accent_primary),
                );
            }

            container(preview_col)
                .width(Length::Fixed(270.0))
                .height(Length::Fill)
                .padding(16)
                .align_x(Center)
                .align_y(Center)
                .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0))
                .into()
        }
    } else {
        container(
            text(strings.inspector_empty)
                .size(12)
                .color(palette.text_muted),
        )
        .width(Length::Fixed(270.0))
        .height(Length::Fill)
        .padding(16)
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0))
        .into()
    };

    cols_row = cols_row.push(col2).push(col_divider).push(col3);

    scrollable(cols_row)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new(),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Intercepts mouse wheel events before child `scrollable` viewports when
#[derive(Default)]
struct CtrlWheelState {
    ctrl_held: bool,
}

/// Wrapper that intercepts mouse wheel events over the central canvas while
/// `Ctrl` is held so the grid zooms smoothly in place without scrolling.
struct CtrlWheelZoom<'a, Message> {
    content: Element<'a, Message>,
    can_zoom: bool,
    intercept: bool,
    on_wheel: Box<dyn Fn(mouse::ScrollDelta) -> Message + 'a>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for CtrlWheelZoom<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<CtrlWheelState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(CtrlWheelState::default())
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<CtrlWheelState>();
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
                state.ctrl_held = mods.control();
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Control))
                    || modifiers.control()
                {
                    state.ctrl_held = true;
                }
            }
            Event::Keyboard(keyboard::Event::KeyReleased { key, modifiers, .. }) => {
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Control)) {
                    state.ctrl_held = false;
                } else {
                    state.ctrl_held = modifiers.control();
                }
            }
            Event::Window(iced::window::Event::Unfocused) => {
                state.ctrl_held = false;
            }
            _ => {}
        }

        if self.can_zoom
            && (self.intercept || state.ctrl_held)
            && matches!(event, Event::Mouse(mouse::Event::WheelScrolled { .. }))
            && cursor.is_over(layout.bounds())
            && let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
        {
            shell.publish((self.on_wheel)(*delta));
            shell.capture_event();
            return;
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn truncate_label(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_owned();
    }
    let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{prefix}…")
}
