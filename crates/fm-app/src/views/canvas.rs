use fm_core::{
    CopyMode, DIR_FILTER_SENTINEL, FileEntry, IconThemeMode, JobProgress, SelectionModifiers,
    SortDirection, SortField, Strings, TabState, ViewMode, format_bytes,
};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles,
};
use iced::Alignment::{self, Center};
use iced::advanced::layout;
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, overlay};
use iced::keyboard;
use iced::mouse;
use iced::widget::{
    Space, button, column, container, image, mouse_area, responsive, row, scrollable, space, stack,
    text, tooltip,
};
use iced::{
    Background, Border, Color, ContentFit, Element, Event, Length, Padding, Point, Rectangle, Size,
    Vector,
};
use lucide_icons::iced::{
    icon_align_center, icon_align_left, icon_arrow_left, icon_arrow_right, icon_arrow_up,
    icon_arrow_up_down, icon_clipboard_paste, icon_copy, icon_eye, icon_eye_off, icon_filter,
    icon_folder_tree, icon_info, icon_minus, icon_pencil, icon_plus, icon_rotate_ccw,
    icon_scissors, icon_shapes, icon_shield_alert, icon_star, icon_terminal, icon_trash_2, icon_x,
};

use crate::app::{
    App, InternalEntryDrag, Message, SplitPane, SplitViewState, pane_scroll_id,
    selected_id_set_contains, thumbnail_key_for_entry,
};
use crate::views::modals::format_transfer_summary;
use crate::views::virtualized::{
    ContentRect, DETAIL_LIST_SIDE_PADDING, DETAIL_ROW_HEIGHT, DETAIL_ROW_STEP, GRID_BAND_SPACING,
    GRID_GAP, GridBand, build_tab_grid_bands, clamp_grid_card_width, clamp_scroll_y,
    compute_band_window_with_row_height, compute_fixed_window, compute_grid_layout_metrics,
    grid_bands_total_height, grid_card_height, grid_columns_for_width, grid_preview_box_size,
};
use crate::views::{render_entry_icon, sanitize_single_line, with_tooltip};

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

    let is_split = app.split_view.is_some();
    let mut header_stack = column![action_bar(app, tab, palette, strings)].spacing(14);
    if !app.update_banner_dismissed_session
        && let Some(update) = app.visible_update()
    {
        header_stack = header_stack.push(update_banner(update, palette, strings));
    }
    if !is_split && app.config.show_hero_card {
        header_stack = header_stack.push(hero_card(tab, palette, strings));
    }
    if let Some(job) = &app.active_transfer {
        header_stack = header_stack.push(transfer_banner(job, palette, strings));
    }

    let workspace: Element<'a, Message> = if let Some(split) = &app.split_view {
        let left_p = (split.ratio * 1000.0).round().clamp(250.0, 750.0) as u16;
        let right_p = 1000_u16.saturating_sub(left_p).max(250);
        let left_pane = container(split_pane_panel(
            app,
            SplitPane::Left,
            app.left_tab(),
            split.active_pane == SplitPane::Left,
            palette,
            strings,
        ))
        .width(Length::FillPortion(left_p))
        .height(Length::Fill);

        let right_pane = container(split_pane_panel(
            app,
            SplitPane::Right,
            Some(&split.right_tab),
            split.active_pane == SplitPane::Right,
            palette,
            strings,
        ))
        .width(Length::FillPortion(right_p))
        .height(Length::Fill);

        row![left_pane, split_divider(split, palette), right_pane]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        let content = pane_body(app, SplitPane::Left, tab, palette, strings);
        mouse_area(content)
            .on_enter(Message::PanePointerEnter(SplitPane::Left))
            .on_exit(Message::PanePointerExit(SplitPane::Left))
            .on_right_press(Message::OpenCanvasContextMenu(SplitPane::Left))
            .into()
    };

    let outer_padding = if is_split {
        Padding::from([10, 12])
    } else {
        Padding::from([14, 20])
    };
    let panel: Element<'a, Message> = container(
        column![header_stack, workspace]
            .spacing(if is_split { 10 } else { 16 })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(outer_padding)
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

fn pane_body<'a>(
    app: &'a App,
    pane: SplitPane,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    if tab.is_loading {
        mouse_area(
            container(
                text(strings.loading)
                    .size(13)
                    .font(FONT_MONO)
                    .color(palette.text_secondary),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Center)
            .align_y(Center),
        )
        .on_press(Message::FocusSplitPane(pane))
        .into()
    } else if let Some(err) = &tab.last_error {
        error_banner(err, tab.can_go_back(), palette, strings)
    } else if tab.visible_entries.is_empty() {
        mouse_area(
            container(
                text(strings.empty_dir)
                    .size(13)
                    .font(FONT_UI)
                    .color(palette.text_muted),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Center)
            .align_y(Center),
        )
        .on_press(Message::FocusSplitPane(pane))
        .into()
    } else {
        match tab.view_mode {
            ViewMode::Grid => grid_view(app, pane, tab, palette, strings),
            ViewMode::DetailTable => detail_table_view(app, pane, tab, palette, strings),
        }
    }
}

fn split_pane_panel<'a>(
    app: &'a App,
    pane: SplitPane,
    tab: Option<&'a TabState>,
    is_active: bool,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let Some(tab) = tab else {
        return container(text(strings.no_active_tab))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let is_pane_drop_target = app.entry_drag.is_some()
        && app.drag_hover_pane == Some(pane)
        && app.drag_hover_folder.is_none()
        && app
            .current_drag_destination()
            .is_some_and(|dest| dest == tab.current_path);

    let pane_bg = if is_active {
        palette.surface_lowest
    } else {
        match palette.preset {
            fm_core::ThemePreset::ObsidianFlow => palette.canvas,
            fm_core::ThemePreset::AetherLight => palette.surface_low,
        }
    };
    let pane_border_color = if is_pane_drop_target {
        palette.accent_primary
    } else if is_active {
        palette.border_strong
    } else {
        palette.border
    };
    let pane_border_width = if is_pane_drop_target { 2.0 } else { 1.0 };

    let top_accent = container(Space::new().width(Length::Fill).height(2.0)).style(move |_| {
        styles::panel_surface(
            if is_pane_drop_target || is_active {
                palette.accent_primary
            } else {
                Color::TRANSPARENT
            },
            Color::TRANSPARENT,
            0.0,
        )
    });

    let header = split_pane_header(pane, tab, is_active, palette, strings);
    let body = container(pane_body(app, pane, tab, palette, strings))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::from([6, 8]));

    let inner = container(
        column![
            top_accent,
            header,
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            body,
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true)
    .style(move |_| container::Style {
        background: Some(Background::Color(pane_bg)),
        border: Border {
            color: pane_border_color,
            width: pane_border_width,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    });

    mouse_area(inner)
        .on_enter(Message::PanePointerEnter(pane))
        .on_exit(Message::PanePointerExit(pane))
        .on_right_press(Message::OpenCanvasContextMenu(pane))
        .into()
}

fn split_pane_header<'a>(
    pane: SplitPane,
    tab: &'a TabState,
    is_active: bool,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let dot_color = if is_active {
        palette.accent_primary
    } else {
        palette.text_muted
    };
    let dot = container(Space::new().width(7).height(7)).style(styles::badge_container(
        dot_color,
        dot_color,
        Color::TRANSPARENT,
        9999.0,
    ));

    let path_str = sanitize_single_line(&tab.current_path.to_string_lossy());
    let title_area = mouse_area(
        row![
            dot,
            text(sanitize_single_line(tab.title.as_str()))
                .size(12)
                .font(if is_active { FONT_UI_SEMIBOLD } else { FONT_UI })
                .color(if is_active {
                    palette.text_primary
                } else {
                    palette.text_secondary
                })
                .wrapping(text::Wrapping::None),
            container(
                text(path_str)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(8)
        .align_y(Center)
        .width(Length::Fill),
    )
    .on_press(Message::FocusSplitPane(pane));

    let mut back_btn = button(icon_arrow_left().size(13))
        .padding([3, 5])
        .style(styles::ghost_button(palette, 6.0));
    if tab.can_go_back() {
        back_btn = back_btn.on_press(Message::SplitNavigateBack(pane));
    }

    let mut fwd_btn = button(icon_arrow_right().size(13))
        .padding([3, 5])
        .style(styles::ghost_button(palette, 6.0));
    if tab.can_go_forward() {
        fwd_btn = fwd_btn.on_press(Message::SplitNavigateForward(pane));
    }

    let mut up_btn = button(icon_arrow_up().size(13))
        .padding([3, 5])
        .style(styles::ghost_button(palette, 6.0));
    if !fm_core::is_virtual_readonly_dir(&tab.current_path) && tab.current_path.parent().is_some() {
        up_btn = up_btn.on_press(Message::SplitNavigateUp(pane));
    }

    let controls = row![
        back_btn,
        fwd_btn,
        up_btn,
        with_tooltip(
            button(icon_x().size(13).color(palette.text_muted))
                .padding([3, 5])
                .style(styles::ghost_button(palette, 6.0))
                .on_press(Message::CloseSplitPane(pane)),
            strings.close_split_pane,
            tooltip::Position::Bottom,
            palette,
        ),
    ]
    .spacing(2)
    .align_y(Center);

    let header_bg = if is_active {
        palette.surface_low
    } else {
        palette.surface_lowest
    };

    container(
        row![title_area, controls]
            .spacing(6)
            .align_y(Center)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding([4, 10])
    .style(move |_| styles::panel_surface(header_bg, Color::TRANSPARENT, 0.0))
    .into()
}

fn split_divider<'a>(split: &'a SplitViewState, palette: ThemePalette) -> Element<'a, Message> {
    let highlighted = split.resizing.is_some() || split.divider_hovered;
    let line_color = if highlighted {
        palette.accent_primary
    } else {
        palette.border
    };
    let line_width = if highlighted { 2.0 } else { 1.0 };

    let full_line = container(
        container(Space::new().width(line_width).height(Length::Fill))
            .style(move |_| styles::panel_surface(line_color, Color::TRANSPARENT, 0.0)),
    )
    .width(Length::Fixed(10.0))
    .height(Length::Fill)
    .align_x(Center);

    let content: Element<'a, Message> =
        if highlighted {
            let grip = container(container(Space::new().width(4.0).height(40.0)).style(
                move |_| styles::panel_surface(palette.accent_primary, Color::TRANSPARENT, 9999.0),
            ))
            .width(Length::Fixed(10.0))
            .height(Length::Fill)
            .align_x(Center)
            .align_y(Center);
            stack![full_line, grip].into()
        } else {
            full_line.into()
        };

    mouse_area(content)
        .interaction(mouse::Interaction::ResizingHorizontally)
        .on_enter(Message::SplitDividerHovered(true))
        .on_exit(Message::SplitDividerHovered(false))
        .on_press(Message::StartSplitResize)
        .on_right_press(Message::ResetSplitRatio)
        .into()
}

/// Pill flutuante seguindo o cursor durante drag & drop interno entre painéis/pastas.
pub fn drag_preview_overlay<'a>(
    app: &'a App,
    drag: &'a InternalEntryDrag,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mode = app.current_drag_mode();
    let dest = app.current_drag_destination();
    let count = drag.paths.len();

    let item_summary = if count == 1 {
        truncate_label(&sanitize_single_line(&drag.primary_label), 28)
    } else {
        strings.format_items_count(count)
    };
    let prefix = match mode {
        CopyMode::Copy => strings.drag_copy_prefix,
        CopyMode::Move => strings.drag_move_prefix,
    };

    let mut content = row![
        match mode {
            CopyMode::Copy => icon_copy().size(13).color(palette.accent_primary),
            CopyMode::Move => icon_arrow_right().size(13).color(palette.accent_primary),
        },
        text(format!("{prefix}: {item_summary}"))
            .size(12)
            .font(FONT_UI_SEMIBOLD)
            .color(palette.text_primary)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(6)
    .align_y(Center);

    if let Some(dest_dir) = dest {
        let dest_name = dest_dir
            .file_name()
            .map(|n| sanitize_single_line(&n.to_string_lossy()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| sanitize_single_line(&dest_dir.to_string_lossy()));
        content = content.push(
            text(format!("→ {}", truncate_label(&dest_name, 22)))
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(palette.accent_primary)
                .wrapping(text::Wrapping::None),
        );
    }

    if mode == CopyMode::Move {
        content = content.push(
            container(
                text(strings.drag_ctrl_copy_hint)
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
        );
    }

    let pill = container(content)
        .padding([6, 12])
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.surface_low)),
            border: Border {
                color: palette.accent_primary,
                width: 1.5,
                radius: 10.0.into(),
            },
            shadow: iced::Shadow {
                color: Color {
                    a: 0.35,
                    ..Color::BLACK
                },
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..container::Style::default()
        });

    let max_x = (app.window_size.width - 280.0).max(8.0);
    let max_y = (app.window_size.height - 44.0).max(8.0);
    let x = (app.cursor_position.x + 16.0).clamp(8.0, max_x);
    let y = (app.cursor_position.y + 18.0).clamp(8.0, max_y);

    container(pill)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: y,
            right: 0.0,
            bottom: 0.0,
            left: x,
        })
        .into()
}

fn update_banner<'a>(
    update: &'a fm_core::UpdateInfo,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let content = row![
        icon_arrow_up().size(14).color(palette.accent_primary),
        text(format!(
            "{} {}",
            strings.update_available_banner, update.tag_name
        ))
        .size(12)
        .font(FONT_UI_SEMIBOLD)
        .color(palette.text_primary),
        space::horizontal(),
        button(
            text(strings.update_ignore_btn)
                .size(11)
                .font(FONT_UI)
                .color(palette.text_secondary),
        )
        .padding([4, 10])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::DismissUpdateVersion(update.tag_name.clone())),
        button(
            text(strings.update_download_btn)
                .size(12)
                .font(FONT_UI_SEMIBOLD),
        )
        .padding([5, 12])
        .style(styles::pill_primary_button(palette))
        .on_press(Message::OpenUpdateReleasePage(update.release_url.clone())),
        button(icon_x().size(13).color(palette.text_muted))
            .padding([4, 6])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::DismissUpdateSession),
    ]
    .spacing(8)
    .align_y(Center);

    container(content)
        .width(Length::Fill)
        .padding([8, 14])
        .style(move |_| styles::panel_surface(palette.accent_surface, palette.accent_border, 10.0))
        .into()
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

    if app.is_elevated {
        left_info = left_info.push(
            container(
                row![
                    icon_shield_alert().size(12).color(palette.status_error),
                    text(strings.root_mode_badge)
                        .size(11)
                        .font(FONT_UI_SEMIBOLD)
                        .color(palette.status_error),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([3, 10])
            .style(styles::badge_container(
                Color {
                    a: 0.12,
                    ..palette.status_error
                },
                palette.status_error,
                palette.status_error,
                9999.0,
            )),
        );
    }

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

    let new_btn = with_tooltip(
        button(
            row![
                icon_plus().size(14),
                text(strings.new_item).size(12).font(FONT_UI_SEMIBOLD)
            ]
            .spacing(4)
            .align_y(Center),
        )
        .padding([6, 14])
        .style(styles::pill_primary_button(palette))
        .on_press(Message::OpenNewMenu),
        strings.tip_new_item,
        tooltip::Position::Bottom,
        palette,
    );

    let is_in_trash = app.is_current_tab_trash();
    let is_in_recent = app.is_current_tab_recent();
    let is_inside_archive = fm_fs::archive::split_archive_path(&tab.current_path).is_some();
    let sel_count = tab.selection.count();
    let mut ops_row = row![].spacing(4).align_y(Center);
    if is_in_recent {
        ops_row = ops_row.push(
            button(
                row![
                    icon_trash_2().size(13).color(palette.status_error),
                    text(strings.clear_recent_files)
                        .size(12)
                        .color(palette.status_error),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([6, 10])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::ClearRecentFiles),
        );
    }
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
            .push(with_tooltip(
                button(icon_copy().size(14).color(palette.text_primary))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CopySelected),
                strings.copy,
                tooltip::Position::Bottom,
                palette,
            ))
            .push(with_tooltip(
                button(icon_scissors().size(14).color(palette.text_primary))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::CutSelected),
                strings.cut,
                tooltip::Position::Bottom,
                palette,
            ));
    }

    if !is_in_trash && !is_in_recent && !is_inside_archive {
        let paste_btn: Element<'a, Message> = if let Some(clip) = &app.clipboard {
            let tip = format!("{} ({})", strings.paste, clip.paths.len());
            let paste_color = match clip.mode {
                CopyMode::Copy => palette.accent_primary,
                CopyMode::Move => palette.status_warning,
            };
            with_tooltip(
                button(
                    row![
                        icon_clipboard_paste().size(14).color(paste_color),
                        text(clip.paths.len().to_string())
                            .size(11)
                            .font(FONT_MONO_BOLD)
                            .color(paste_color),
                    ]
                    .spacing(4)
                    .align_y(Center),
                )
                .padding([6, 8])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::PasteClipboard),
                tip,
                tooltip::Position::Bottom,
                palette,
            )
        } else {
            with_tooltip(
                button(
                    icon_clipboard_paste()
                        .size(14)
                        .color(palette.text_secondary),
                )
                .padding([6, 8])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::PasteClipboard),
                strings.paste,
                tooltip::Position::Bottom,
                palette,
            )
        };
        ops_row = ops_row.push(paste_btn);
    }

    if sel_count > 0 {
        let rename_tip = if sel_count > 1 {
            strings.br_title
        } else {
            strings.rename
        };
        ops_row = ops_row
            .push(with_tooltip(
                button(icon_pencil().size(14).color(palette.text_primary))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::OpenRenameDialog),
                rename_tip,
                tooltip::Position::Bottom,
                palette,
            ))
            .push(with_tooltip(
                button(icon_trash_2().size(14).color(palette.status_error))
                    .padding([6, 8])
                    .style(styles::pill_secondary_button(palette))
                    .on_press(Message::RequestDelete(is_in_trash)),
                if is_in_trash {
                    strings.delete_permanent
                } else {
                    strings.move_to_trash
                },
                tooltip::Position::Bottom,
                palette,
            ));
    }

    let is_fav = app.config.favorites.contains(&tab.current_path);
    let fav_btn = with_tooltip(
        button(icon_star().size(15).color(if is_fav {
            palette.accent_tertiary
        } else {
            palette.text_secondary
        }))
        .padding([6, 8])
        .style(styles::ghost_button(palette, 9999.0))
        .on_press(Message::ToggleFavoriteActive),
        strings.tip_favorite,
        tooltip::Position::Bottom,
        palette,
    );

    let sort_btn = with_tooltip(
        button(
            row![
                icon_arrow_up_down().size(13),
                text(format!("{}: {sort_name}", strings.sort)).size(12),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([6, 12])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::CycleSortField),
        strings.tip_sort,
        tooltip::Position::Bottom,
        palette,
    );

    let sort_dir_btn = with_tooltip(
        button(text(dir_arrow).size(12).font(FONT_MONO_BOLD))
            .padding([6, 10])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::ToggleSortDirection),
        strings.tip_sort_dir,
        tooltip::Position::Bottom,
        palette,
    );

    let filter_label = match tab.extension_filter.as_deref() {
        None => strings.filter_ext.to_owned(),
        Some(DIR_FILTER_SENTINEL) => {
            format!("{}: {}", strings.filter_ext, strings.directories_only)
        }
        Some(ext) => format!(
            "{}: {}",
            strings.filter_ext,
            fm_core::format_extension_filter_badge(ext)
        ),
    };
    let filter_active = tab.extension_filter.is_some();
    let filter_btn = with_tooltip(
        button(
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
        .on_press(Message::OpenFilterMenu),
        strings.tip_filter,
        tooltip::Position::Bottom,
        palette,
    );

    let hidden_icon = if tab.show_hidden {
        icon_eye().size(13)
    } else {
        icon_eye_off().size(13)
    };
    let hidden_btn = with_tooltip(
        button(
            row![hidden_icon, text(strings.hidden).size(12)]
                .spacing(6)
                .align_y(Center),
        )
        .padding([6, 12])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::ToggleHidden),
        strings.tip_hidden,
        tooltip::Position::Bottom,
        palette,
    );

    let sys_icons_active = app.config.icon_theme_mode == IconThemeMode::HostSystem;
    let icons_btn = with_tooltip(
        button(icon_shapes().size(13).color(if sys_icons_active {
            palette.accent_primary
        } else {
            palette.text_secondary
        }))
        .padding([6, 8])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::ToggleSystemIcons),
        strings.tip_system_icons,
        tooltip::Position::Bottom,
        palette,
    );

    let terminal_btn = with_tooltip(
        button(
            row![
                icon_terminal().size(13),
                text(strings.open_terminal).size(12),
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([6, 10])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::OpenTerminal),
        strings.tip_terminal,
        tooltip::Position::Bottom,
        palette,
    );

    let info_btn = with_tooltip(
        button(icon_info().size(16).color(if app.config.show_inspector {
            palette.accent_primary
        } else {
            palette.text_secondary
        }))
        .padding([6, 8])
        .style(styles::ghost_button(palette, 9999.0))
        .on_press(Message::ToggleInspector),
        strings.tip_inspector,
        tooltip::Position::Bottom,
        palette,
    );

    let mut toolbar_right = row![].spacing(8).align_y(Center);
    if !is_in_trash && !is_in_recent && !is_inside_archive {
        toolbar_right = toolbar_right.push(new_btn);
    }
    toolbar_right = toolbar_right.push(ops_row);

    if tab.view_mode == ViewMode::Grid {
        let card_px = clamp_grid_card_width(app.config.grid_card_width).round() as u32;
        let group_active = app.config.grid_group_folders;
        let group_btn = with_tooltip(
            button(icon_folder_tree().size(13).color(if group_active {
                palette.accent_primary
            } else {
                palette.text_secondary
            }))
            .padding([6, 8])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::ToggleGridGroupFolders),
            strings.tip_group_folders,
            tooltip::Position::Bottom,
            palette,
        );

        let center_active = app.config.grid_center_items;
        let align_icon = if center_active {
            icon_align_center().size(13).color(palette.accent_primary)
        } else {
            icon_align_left().size(13).color(palette.text_secondary)
        };
        let align_btn = with_tooltip(
            button(align_icon)
                .padding([6, 8])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::ToggleGridCenterItems),
            strings.tip_grid_alignment,
            tooltip::Position::Bottom,
            palette,
        );

        let size_controls = container(
            row![
                with_tooltip(
                    button(icon_minus().size(12).color(palette.text_primary))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::StepGridCardWidth(-16.0)),
                    strings.tip_zoom_out,
                    tooltip::Position::Bottom,
                    palette,
                ),
                with_tooltip(
                    button(
                        text(format!("{card_px}px"))
                            .size(11)
                            .font(FONT_MONO)
                            .color(palette.text_secondary),
                    )
                    .padding([3, 6])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::ResetGridCardWidth),
                    strings.tip_zoom_reset,
                    tooltip::Position::Bottom,
                    palette,
                ),
                with_tooltip(
                    button(icon_plus().size(12).color(palette.text_primary))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::StepGridCardWidth(16.0)),
                    strings.tip_zoom_in,
                    tooltip::Position::Bottom,
                    palette,
                ),
            ]
            .spacing(2)
            .align_y(Center),
        )
        .padding(2)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0));

        toolbar_right = toolbar_right
            .push(group_btn)
            .push(align_btn)
            .push(size_controls);
    }

    toolbar_right = toolbar_right
        .push(sort_btn)
        .push(sort_dir_btn)
        .push(filter_btn)
        .push(hidden_btn)
        .push(icons_btn)
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
    pane: SplitPane,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let group_folders = app.config.grid_group_folders;
    let center_items = app.config.grid_center_items;
    let modifiers = SelectionModifiers {
        ctrl: app.current_modifiers.control(),
        shift: app.current_modifiers.shift(),
    };
    let (scroll_y, viewport_h, _) = app.pane_scroll_metrics(pane);
    let card_width = clamp_grid_card_width(app.config.grid_card_width);
    let card_height = grid_card_height(card_width);
    let box_width = (card_width - 14.0).max(88.0);
    let preview_box = grid_preview_box_size(card_width);
    let line_chars = ((box_width - 14.0) / 7.2).round().clamp(11.0, 32.0) as usize;
    let label_chars = line_chars.saturating_mul(3).max(36);
    let drag_dest = app.current_drag_destination();

    responsive(move |size| {
        let fit_cols = grid_columns_for_width(size.width, card_width);
        let bands = build_tab_grid_bands(&tab.visible_entries, fit_cols, group_folders, strings);
        let total_content_h = if bands.is_empty() {
            48.0
        } else {
            grid_bands_total_height(&bands, card_height) + GRID_BAND_SPACING + 48.0
        };
        let effective_viewport_h = if size.height > 1.0 {
            size.height
        } else {
            viewport_h
        };
        let effective_scroll_y = clamp_scroll_y(scroll_y, total_content_h, effective_viewport_h);
        let win = compute_band_window_with_row_height(
            &bands,
            card_height,
            effective_scroll_y,
            effective_viewport_h,
            280.0,
        );
        let metrics = compute_grid_layout_metrics(&bands, size.width, card_width, center_items);

        let mut list = column![]
            .spacing(GRID_BAND_SPACING)
            .width(Length::Fixed(metrics.block_width));
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
                        let is_drop_target = entry.kind.is_dir_like()
                            && app
                                .drag_hover_folder
                                .as_ref()
                                .is_some_and(|(p, folder)| *p == pane && folder == &entry.path)
                            && drag_dest.as_ref().is_some_and(|d| d == &entry.path);
                        r = r.push(grid_item_card(
                            app,
                            pane,
                            idx,
                            entry,
                            selected_id_set_contains(tab, entry.id),
                            is_drop_target,
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
                            .align_x(if metrics.center_sparse_rows {
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
        // Garante área vazia clicável ao final do grid mesmo quando a pasta preenche toda a largura.
        list = list.push(Space::new().height(48.0));

        let aligned_canvas = if center_items {
            container(list).width(Length::Fill).align_x(Center)
        } else {
            container(list)
                .width(Length::Fill)
                .padding(Padding::from([0.0, metrics.list_x]))
                .align_x(Alignment::Start)
        };

        let scroll = scrollable(aligned_canvas)
            .id(pane_scroll_id(pane))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(move |vp| Message::CanvasScrolled {
                pane,
                offset_y: vp.absolute_offset().y,
                viewport_height: vp.bounds().height,
                viewport_width: vp.bounds().width,
            });
        wrap_marquee(app, pane, scroll.into(), effective_scroll_y, palette)
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
    app: &'a App,
    pane: SplitPane,
    index: usize,
    entry: &'a FileEntry,
    selected: bool,
    is_drop_target: bool,
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
        let icon_px = (preview_box * 0.68).clamp(28.0, 72.0);
        let icon_widget = render_entry_icon(app, entry, palette, icon_px);
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
    let name_text = text(display_name)
        .size(12)
        .line_height(1.25)
        .font(FONT_UI_SEMIBOLD)
        .color(if selected {
            palette.on_accent
        } else {
            palette.text_primary
        })
        .align_x(Center)
        .wrapping(text::Wrapping::WordOrGlyph);

    let sub_text = text(sub_label)
        .size(11)
        .font(FONT_MONO)
        .color(if selected {
            Color {
                a: 0.90,
                ..palette.on_accent
            }
        } else {
            palette.text_secondary
        })
        .wrapping(text::Wrapping::None);

    let box_width = (card_width - 14.0).max(88.0);

    let label_content = column![
        container(name_text)
            .max_width(box_width - 12.0)
            .max_height(48.0)
            .clip(true)
            .align_x(Center),
        sub_text,
    ]
    .spacing(2)
    .align_x(Center);

    let label_block: Element<'a, Message> = if selected {
        container(label_content)
            .padding([4, 8])
            .align_x(Center)
            .style(styles::badge_container(
                palette.accent_primary,
                palette.on_accent,
                Color::TRANSPARENT,
                6.0,
            ))
            .into()
    } else {
        container(label_content)
            .padding([4, 6])
            .align_x(Center)
            .into()
    };

    let card_content = column![
        container(preview_widget)
            .height(preview_box)
            .align_x(Center)
            .align_y(Alignment::End),
        label_block,
    ]
    .spacing(5)
    .align_x(Center);

    let card_btn = button(card_content)
        .width(Length::Shrink)
        .height(Length::Shrink)
        .padding([2, 2])
        .style(styles::nemo_grid_item_button(palette))
        .on_press(Message::SelectEntry {
            pane,
            index,
            modifiers,
        });

    let card_inner: Element<'a, Message> = if is_drop_target {
        container(card_btn)
            .padding(2)
            .style(move |_| container::Style {
                background: Some(Background::Color(Color {
                    a: 0.14,
                    ..palette.accent_primary
                })),
                border: Border {
                    color: palette.accent_primary,
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    } else {
        card_btn.into()
    };

    let mut card_area =
        mouse_area(card_inner).on_right_press(Message::OpenEntryContextMenu { pane, index });
    if entry.kind.is_dir_like() && app.entry_drag.is_some() {
        card_area = card_area
            .on_enter(Message::DragHoverFolderEnter {
                pane,
                path: entry.path.clone(),
            })
            .on_exit(Message::DragHoverFolderExit(entry.path.clone()));
    }

    let draggable = wrap_draggable_entry(app, pane, index, card_area.into());

    // Mantém a célula virtualizada de tamanho fixo enquanto o botão envolve apenas ícone + rótulo.
    container(draggable)
        .width(Length::Fixed(card_width))
        .height(Length::Fixed(card_height))
        .align_x(Center)
        .align_y(Alignment::Start)
        .into()
}

fn sort_header_button<'a>(
    pane: SplitPane,
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
    .on_press(Message::SortByField { pane, field })
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
    pane: SplitPane,
    tab: &'a TabState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let modifiers = SelectionModifiers {
        ctrl: app.current_modifiers.control(),
        shift: app.current_modifiers.shift(),
    };
    let (scroll_y, viewport_h, _) = app.pane_scroll_metrics(pane);
    let drag_dest = app.current_drag_destination();
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
            sort_header_button(
                pane,
                strings.col_name,
                SortField::Name,
                tab.sort,
                p[0],
                palette
            ),
            col_resize_handle(0, palette),
            sort_header_button(
                pane,
                strings.col_size,
                SortField::Size,
                tab.sort,
                p[1],
                palette
            ),
            col_resize_handle(1, palette),
            sort_header_button(
                pane,
                strings.col_type,
                SortField::Kind,
                tab.sort,
                p[2],
                palette
            ),
            col_resize_handle(2, palette),
            text(strings.col_perms)
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(palette.text_muted)
                .width(Length::FillPortion(p[3])),
            col_resize_handle(3, palette),
            sort_header_button(
                pane,
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

    let total_content_h = tab.visible_entries.len() as f32 * DETAIL_ROW_STEP + 48.0;
    let effective_scroll_y = clamp_scroll_y(scroll_y, total_content_h, viewport_h);
    let win = compute_fixed_window(
        tab.visible_entries.len(),
        DETAIL_ROW_STEP,
        effective_scroll_y,
        viewport_h,
        6,
    );

    let mut rows = column![].spacing(4).width(Length::Fill);
    if win.top_spacer > 0.0 {
        rows = rows.push(Space::new().height(win.top_spacer));
    }

    for idx in win.range {
        let entry = &tab.visible_entries[idx];
        let selected = selected_id_set_contains(tab, entry.id);
        let icon_widget = render_entry_icon(app, entry, palette, 16.0);
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

        let is_drop_target = entry.kind.is_dir_like()
            && app
                .drag_hover_folder
                .as_ref()
                .is_some_and(|(p, folder)| *p == pane && folder == &entry.path)
            && drag_dest.as_ref().is_some_and(|d| d == &entry.path);

        let row_btn = button(row_content)
            .width(Length::Fill)
            .height(DETAIL_ROW_HEIGHT)
            .padding([6, 12])
            .style(styles::entry_card_button(
                palette,
                selected || is_drop_target,
                8.0,
            ))
            .on_press(Message::SelectEntry {
                pane,
                index: idx,
                modifiers,
            });

        let mut row_area =
            mouse_area(row_btn).on_right_press(Message::OpenEntryContextMenu { pane, index: idx });
        if entry.kind.is_dir_like() && app.entry_drag.is_some() {
            row_area = row_area
                .on_enter(Message::DragHoverFolderEnter {
                    pane,
                    path: entry.path.clone(),
                })
                .on_exit(Message::DragHoverFolderExit(entry.path.clone()));
        }

        rows = rows.push(wrap_draggable_entry(app, pane, idx, row_area.into()));
    }

    if win.bottom_spacer > 0.0 {
        rows = rows.push(Space::new().height(win.bottom_spacer));
    }
    rows = rows.push(Space::new().height(48.0));

    let padded_rows = container(rows)
        .width(Length::Fill)
        .padding(Padding::from([0.0, DETAIL_LIST_SIDE_PADDING]));

    let scroll = scrollable(padded_rows)
        .id(pane_scroll_id(pane))
        .height(Length::Fill)
        .on_scroll(move |vp| Message::CanvasScrolled {
            pane,
            offset_y: vp.absolute_offset().y,
            viewport_height: vp.bounds().height,
            viewport_width: vp.bounds().width,
        });

    column![
        sticky_header,
        wrap_marquee(app, pane, scroll.into(), effective_scroll_y, palette)
    ]
    .spacing(6)
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

fn wrap_draggable_entry<'a>(
    app: &'a App,
    pane: SplitPane,
    index: usize,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let enabled = app.active_dialog.is_none()
        && app.context_menu.is_none()
        && app.breadcrumb_menu.is_none()
        && !app.quicklook_open
        && !app.prefs_open
        && !app.deep_search_open;
    Element::new(DraggableEntry {
        content,
        pane,
        index,
        enabled,
    })
}

#[derive(Default)]
struct DraggableEntryState {
    press_origin: Option<Point>,
    dragging: bool,
}

/// Wrapper leve sobre itens do grid/lista que detecta arraste (>= 8px) sem bloquear cliques ou hover de destinos.
struct DraggableEntry<'a> {
    content: Element<'a, Message>,
    pane: SplitPane,
    index: usize,
    enabled: bool,
}

impl Widget<Message, iced::Theme, iced::Renderer> for DraggableEntry<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<DraggableEntryState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DraggableEntryState::default())
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
        let state = tree.state.downcast_mut::<DraggableEntryState>();
        let bounds = layout.bounds();

        if self.enabled {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if !shell.is_event_captured()
                        && let Some(pos) = cursor.position_over(bounds)
                    {
                        state.press_origin = Some(pos);
                        state.dragging = false;
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    if let Some(origin) = state.press_origin
                        && !state.dragging
                        && (position.x - origin.x).hypot(position.y - origin.y) >= 8.0
                    {
                        state.dragging = true;
                        shell.publish(Message::StartEntryDrag {
                            pane: self.pane,
                            index: self.index,
                        });
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Window(iced::window::Event::Unfocused) => {
                    state.press_origin = None;
                    state.dragging = false;
                }
                _ => {}
            }
        } else {
            state.press_origin = None;
            state.dragging = false;
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
        let state = tree.state.downcast_ref::<DraggableEntryState>();
        if state.dragging {
            return mouse::Interaction::Grabbing;
        }
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

fn wrap_marquee<'a>(
    app: &'a App,
    pane: SplitPane,
    content: Element<'a, Message>,
    scroll_y: f32,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let enabled = app.active_dialog.is_none()
        && app.context_menu.is_none()
        && app.breadcrumb_menu.is_none()
        && app.entry_drag.is_none()
        && !app.quicklook_open
        && !app.prefs_open
        && !app.deep_search_open;
    Element::new(MarqueeSelect {
        content,
        pane,
        enabled,
        scroll_y,
        accent: palette.accent_primary,
    })
}

#[derive(Debug, Clone, Copy)]
struct MarqueeDrag {
    anchor_screen: Point,
    anchor_content_x: f32,
    anchor_content_y: f32,
    current_content_x: f32,
    current_content_y: f32,
    additive: bool,
    activated: bool,
}

#[derive(Default)]
struct MarqueeState {
    drag: Option<MarqueeDrag>,
    ctrl_held: bool,
    shift_held: bool,
}

/// Wrapper que captura arraste com botão esquerdo em área vazia do canvas para seleção em caixa (marquee).
struct MarqueeSelect<'a> {
    content: Element<'a, Message>,
    pane: SplitPane,
    enabled: bool,
    scroll_y: f32,
    accent: Color,
}

impl Widget<Message, iced::Theme, iced::Renderer> for MarqueeSelect<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<MarqueeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(MarqueeState::default())
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

        let state = tree.state.downcast_ref::<MarqueeState>();
        if let Some(drag) = state.drag
            && drag.activated
        {
            let bounds = layout.bounds();
            let x0 = bounds.x + drag.anchor_content_x;
            let x1 = bounds.x + drag.current_content_x;
            let y0 = bounds.y + drag.anchor_content_y - self.scroll_y;
            let y1 = bounds.y + drag.current_content_y - self.scroll_y;
            let raw_rect = Rectangle {
                x: x0.min(x1),
                y: y0.min(y1),
                width: (x1 - x0).abs().max(1.0),
                height: (y1 - y0).abs().max(1.0),
            };
            if let Some(visible_rect) = raw_rect.intersection(&bounds) {
                renderer.with_layer(bounds, |renderer| {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: visible_rect,
                            border: Border {
                                color: self.accent,
                                width: 1.5,
                                radius: 2.0.into(),
                            },
                            ..renderer::Quad::default()
                        },
                        Background::Color(Color {
                            a: 0.14,
                            ..self.accent
                        }),
                    );
                });
            }
        }
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
        let state = tree.state.downcast_mut::<MarqueeState>();
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
                state.ctrl_held = mods.control();
                state.shift_held = mods.shift();
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Control))
                    || modifiers.control()
                {
                    state.ctrl_held = true;
                }
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Shift))
                    || modifiers.shift()
                {
                    state.shift_held = true;
                }
            }
            Event::Keyboard(keyboard::Event::KeyReleased { key, modifiers, .. }) => {
                state.ctrl_held =
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Control)) {
                        false
                    } else {
                        modifiers.control()
                    };
                state.shift_held =
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Shift)) {
                        false
                    } else {
                        modifiers.shift()
                    };
            }
            Event::Window(iced::window::Event::Unfocused) => {
                state.ctrl_held = false;
                state.shift_held = false;
                state.drag = None;
            }
            _ => {}
        }

        let was_captured = shell.is_event_captured();
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
        let child_captured = !was_captured && shell.is_event_captured();

        if !self.enabled {
            state.drag = None;
            return;
        }

        let bounds = layout.bounds();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !child_captured
                    && !shell.is_event_captured()
                    && let Some(pos) = cursor.position_over(bounds)
                {
                    let content_x = (pos.x - bounds.x).clamp(0.0, bounds.width);
                    let content_y = ((pos.y - bounds.y) + self.scroll_y).max(0.0);
                    state.drag = Some(MarqueeDrag {
                        anchor_screen: pos,
                        anchor_content_x: content_x,
                        anchor_content_y: content_y,
                        current_content_x: content_x,
                        current_content_y: content_y,
                        additive: state.ctrl_held || state.shift_held,
                        activated: false,
                    });
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = state.drag.as_mut() {
                    let content_x = (position.x - bounds.x).clamp(0.0, bounds.width);
                    let content_y = ((position.y - bounds.y) + self.scroll_y).max(0.0);
                    drag.current_content_x = content_x;
                    drag.current_content_y = content_y;
                    let dist = (position.x - drag.anchor_screen.x)
                        .hypot(position.y - drag.anchor_screen.y);
                    if drag.activated || dist >= 4.0 {
                        let is_initial = !drag.activated;
                        drag.activated = true;
                        let rect = ContentRect::from_points(
                            drag.anchor_content_x,
                            drag.anchor_content_y,
                            drag.current_content_x,
                            drag.current_content_y,
                        );
                        shell.publish(Message::MarqueeSelect {
                            pane: self.pane,
                            rect,
                            viewport_width: bounds.width,
                            additive: drag.additive,
                            is_initial,
                        });
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(drag) = state.drag.take() {
                    if drag.activated {
                        shell.publish(Message::MarqueeFinished(self.pane));
                    } else if !drag.additive {
                        shell.publish(Message::ClearSelection(self.pane));
                    } else {
                        shell.publish(Message::FocusSplitPane(self.pane));
                    }
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<MarqueeState>();
        if state.drag.is_some_and(|d| d.activated) {
            return mouse::Interaction::Crosshair;
        }
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
