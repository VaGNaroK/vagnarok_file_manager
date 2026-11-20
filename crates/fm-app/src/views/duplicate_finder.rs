use std::fmt;
use std::time::{Duration, UNIX_EPOCH};

use fm_core::{
    DuplicateConfirmLevel, DuplicateGroup, DuplicateMinSizePreset, DuplicateScanStage,
    KeepStrategy, Strings, format_bytes,
};
use fm_theme::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles,
};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, pick_list, row, scrollable, space, text,
};
use iced::{Color, Element, Length, Padding, mouse};
use lucide_icons::iced::{
    icon_chevron_down, icon_chevron_right, icon_minus, icon_search, icon_trash_2, icon_x,
};

use crate::app::{App, Message};
use crate::duplicate_finder::{
    DuplicateFinderMsg, DuplicateFinderState, DuplicateListRow, effective_duplicate_modal_rect,
};
use crate::views::{modal_overlay_backdrop_at, sanitize_single_line, with_tooltip};

const LIST_ROW_HEIGHT: f32 = 34.0;
const LIST_OVERSCAN_ROWS: usize = 8;

#[derive(Debug, Clone, Copy)]
struct LabeledOption<T: PartialEq + Copy> {
    value: T,
    label: &'static str,
}

impl<T: PartialEq + Copy> PartialEq for LabeledOption<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: PartialEq + Copy> Eq for LabeledOption<T> {}

impl<T: PartialEq + Copy> fmt::Display for LabeledOption<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

/// Overlay modal flutuante, arrastável, redimensionável e minimizável do Buscador de Duplicados.
///
/// Segue o modelo visual da Seção 4.5 (`specs/mockups/duplicate_finder_mockup.html`).
pub fn duplicate_finder_modal_overlay<'a>(
    app: &'a App,
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (x, y, modal_w, modal_h) = effective_duplicate_modal_rect(app.window_size, modal);
    let card = container(modal_content(modal, palette, strings))
        .width(Length::Fixed(modal_w))
        .height(Length::Fixed(modal_h))
        .clip(true)
        .style(styles::modal_card(palette));

    modal_overlay_backdrop_at(card, x, y)
}

fn modal_content<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let header = header_bar(modal, palette, strings);
    let controls = config_panel(modal, palette, strings);
    let quick_select = quick_select_bar(modal, palette, strings);
    let results = results_body(modal, palette, strings);
    let footer = footer_bar(modal, palette, strings);

    let mut content = column![
        header,
        divider(palette),
        controls,
        divider(palette),
        quick_select,
        divider(palette),
        results,
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    if modal.any_group_all_selected() {
        let warn_bg = Color::from_rgba(
            palette.status_error.r,
            palette.status_error.g,
            palette.status_error.b,
            0.14,
        );
        let warn_border = Color::from_rgba(
            palette.status_error.r,
            palette.status_error.g,
            palette.status_error.b,
            0.45,
        );
        content = content.push(
            container(
                text(strings.dup_all_selected_warning)
                    .size(11)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.status_error),
            )
            .padding([7, 14])
            .width(Length::Fill)
            .style(move |_| styles::panel_surface(warn_bg, warn_border, 0.0)),
        );
    }

    content = content.push(divider(palette)).push(footer);
    content.into()
}

fn divider<'a>(palette: ThemePalette) -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(1))
        .style(styles::divider_line(palette))
        .into()
}

fn header_bar<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let root_display = sanitize_single_line(&modal.root.to_string_lossy());

    let md5_badge = container(
        text("MD5")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(palette.accent_primary),
    )
    .padding([4, 8])
    .style(styles::badge_container(
        palette.accent_surface,
        palette.accent_primary,
        palette.accent_border,
        8.0,
    ));

    let drag_area = mouse_area(
        row![
            md5_badge,
            column![
                text(strings.dup_modal_title)
                    .size(14)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary)
                    .wrapping(text::Wrapping::None),
                text(root_display)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(1)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Center)
        .width(Length::Fill),
    )
    .interaction(if modal.dragging_modal.is_some() {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    })
    .on_press(Message::DuplicateFinder(DuplicateFinderMsg::DragStart));

    let minimize_btn = with_tooltip(
        button(icon_minus().size(14))
            .padding([5, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::DuplicateFinder(DuplicateFinderMsg::Minimize)),
        strings.arch_minimize,
        iced::widget::tooltip::Position::Bottom,
        palette,
    );

    let close_btn = button(icon_x().size(14))
        .padding([5, 7])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::DuplicateFinder(DuplicateFinderMsg::Close));

    container(
        row![
            container(drag_area).width(Length::Fill).clip(true),
            minimize_btn,
            close_btn,
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([10, 14])
    .width(Length::Fill)
    .into()
}

fn config_panel<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let level_opts = vec![
        LabeledOption {
            value: DuplicateConfirmLevel::QuickPartialMd5,
            label: strings.dup_level_quick,
        },
        LabeledOption {
            value: DuplicateConfirmLevel::StandardFullMd5,
            label: strings.dup_level_standard,
        },
        LabeledOption {
            value: DuplicateConfirmLevel::StrictMd5AndSha256,
            label: strings.dup_level_sha256,
        },
        LabeledOption {
            value: DuplicateConfirmLevel::ParanoidMd5AndByteCompare,
            label: strings.dup_level_byte,
        },
    ];
    let selected_level = level_opts
        .iter()
        .copied()
        .find(|o| o.value == modal.options.confirm_level);

    let size_opts = vec![
        LabeledOption {
            value: DuplicateMinSizePreset::Any,
            label: strings.dup_min_size_any,
        },
        LabeledOption {
            value: DuplicateMinSizePreset::OneKb,
            label: strings.dup_min_size_1kb,
        },
        LabeledOption {
            value: DuplicateMinSizePreset::OneMb,
            label: strings.dup_min_size_1mb,
        },
        LabeledOption {
            value: DuplicateMinSizePreset::TenMb,
            label: strings.dup_min_size_10mb,
        },
    ];
    let selected_size = size_opts
        .iter()
        .copied()
        .find(|o| o.value == modal.min_size_preset);

    let scan_action_btn: Element<'a, Message> = if modal.scanning {
        button(
            row![
                icon_x().size(13).color(palette.status_error),
                text(strings.dup_stop_scan)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.status_error),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([7, 14])
        .style(styles::pill_secondary_button(palette))
        .on_press(Message::DuplicateFinder(DuplicateFinderMsg::CancelScan))
        .into()
    } else {
        let lbl = if modal.has_completed_scan {
            strings.dup_rescan
        } else {
            strings.dup_start_scan
        };
        let mut btn = button(
            row![
                icon_search().size(13).color(palette.on_accent),
                text(lbl).size(12).font(FONT_UI_SEMIBOLD),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([7, 16])
        .style(styles::pill_primary_button(palette));
        if !modal.deleting {
            btn = btn.on_press(Message::DuplicateFinder(DuplicateFinderMsg::StartScan));
        }
        btn.into()
    };

    let row_top = row![
        column![
            text(strings.dup_confirm_level_label)
                .size(11)
                .font(FONT_UI_BOLD)
                .color(palette.text_secondary),
            pick_list(level_opts, selected_level, |opt| {
                Message::DuplicateFinder(DuplicateFinderMsg::SetConfirmLevel(opt.value))
            })
            .text_size(12)
            .font(FONT_UI)
            .padding([6, 10])
            .width(Length::Fill)
            .style(styles::pick_list_style(palette))
            .menu_style(styles::dropdown_menu_style(palette)),
        ]
        .spacing(4)
        .width(Length::FillPortion(6)),
        column![
            text(strings.dup_min_size_label)
                .size(11)
                .font(FONT_UI_BOLD)
                .color(palette.text_secondary),
            pick_list(size_opts, selected_size, |opt| {
                Message::DuplicateFinder(DuplicateFinderMsg::SetMinSize(opt.value))
            })
            .text_size(12)
            .font(FONT_UI)
            .padding([6, 10])
            .width(Length::Fill)
            .style(styles::pick_list_style(palette))
            .menu_style(styles::dropdown_menu_style(palette)),
        ]
        .spacing(4)
        .width(Length::FillPortion(2)),
        column![Space::new().height(16), scan_action_btn,].align_x(Center),
    ]
    .spacing(10)
    .align_y(Center);

    let stage_badge_label = stage_badge_text(modal, strings);
    let stage_badge = container(
        text(stage_badge_label)
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_secondary),
    )
    .padding([3, 8])
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 6.0));

    let opt_check = |checked: bool, label: &'static str, msg: DuplicateFinderMsg| {
        let mut cb = checkbox(checked).label(label).size(15).text_size(12);
        if !modal.scanning {
            cb = cb.on_toggle(move |_| Message::DuplicateFinder(msg.clone()));
        }
        cb
    };

    let row_filters = row![
        opt_check(
            modal.options.recursive,
            strings.dup_opt_subfolders,
            DuplicateFinderMsg::ToggleSubfolders,
        ),
        opt_check(
            modal.options.include_hidden,
            strings.dup_opt_hidden,
            DuplicateFinderMsg::ToggleHidden,
        ),
        opt_check(
            modal.options.ignore_empty_files,
            strings.dup_opt_ignore_empty,
            DuplicateFinderMsg::ToggleIgnoreEmpty,
        ),
        opt_check(
            modal.options.ignore_hardlinks,
            strings.dup_opt_ignore_hardlinks,
            DuplicateFinderMsg::ToggleIgnoreHardlinks,
        ),
        space::horizontal(),
        stage_badge,
    ]
    .spacing(12)
    .align_y(Center);

    let mut panel = column![row_top, row_filters].spacing(8).width(Length::Fill);

    if modal.options.confirm_level == DuplicateConfirmLevel::QuickPartialMd5 {
        let warn_bg = Color::from_rgba(
            palette.status_warning.r,
            palette.status_warning.g,
            palette.status_warning.b,
            0.12,
        );
        let warn_border = Color::from_rgba(
            palette.status_warning.r,
            palette.status_warning.g,
            palette.status_warning.b,
            0.40,
        );
        panel = panel.push(
            container(
                text(strings.dup_quick_warning)
                    .size(11)
                    .font(FONT_UI)
                    .color(palette.status_warning),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(move |_| styles::panel_surface(warn_bg, warn_border, 8.0)),
        );
    }

    container(panel)
        .padding([10, 14])
        .width(Length::Fill)
        .into()
}

fn quick_select_bar<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let has_groups = !modal.groups.is_empty() && !modal.scanning && !modal.deleting;
    let keep_btn = |label: &'static str, strategy: KeepStrategy| {
        let mut b = button(text(label).size(11).font(FONT_UI))
            .padding([4, 10])
            .style(styles::pill_secondary_button(palette));
        if has_groups {
            b = b.on_press(Message::DuplicateFinder(
                DuplicateFinderMsg::ApplyKeepStrategy(strategy),
            ));
        }
        b
    };

    let selected_count = modal.total_selected_count();
    let selected_bytes = modal.total_selected_reclaimable_bytes();
    let summary_right = format!(
        "{selected_count} {} ({})",
        strings.dup_selected_word,
        format_bytes(selected_bytes)
    );

    container(
        row![
            text(strings.dup_quick_select_label)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_secondary),
            keep_btn(strings.dup_keep_oldest, KeepStrategy::KeepOldest),
            keep_btn(strings.dup_keep_newest, KeepStrategy::KeepNewest),
            keep_btn(strings.dup_keep_shortest, KeepStrategy::KeepShortestPath),
            keep_btn(strings.dup_clear_selection, KeepStrategy::ClearAll),
            space::horizontal(),
            text(summary_right)
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(palette.accent_primary),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([7, 14])
    .width(Length::Fill)
    .into()
}

fn results_body<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    if modal.scanning {
        return scanning_body(modal, palette, strings);
    }

    if modal.flat_rows.is_empty() {
        let msg = if modal.has_completed_scan {
            strings.dup_empty_none
        } else {
            strings.dup_empty_idle
        };
        return container(text(msg).size(13).font(FONT_UI).color(palette.text_muted))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Center)
            .align_y(Center)
            .padding(24)
            .into();
    }

    // Lista 100% virtualizada sobre `flat_rows` com altura fixa por linha (`LIST_ROW_HEIGHT`).
    let total = modal.flat_rows.len();
    let viewport_h = modal.viewport_height.max(160.0);
    let scroll_y = modal.scroll_y.max(0.0);
    let first_visible = ((scroll_y / LIST_ROW_HEIGHT).floor() as usize)
        .saturating_sub(LIST_OVERSCAN_ROWS)
        .min(total);
    let visible_count =
        ((viewport_h / LIST_ROW_HEIGHT).ceil() as usize) + (LIST_OVERSCAN_ROWS * 2) + 2;
    let end_visible = (first_visible + visible_count).min(total);

    let top_pad = (first_visible as f32) * LIST_ROW_HEIGHT;
    let bottom_pad = ((total.saturating_sub(end_visible)) as f32) * LIST_ROW_HEIGHT;

    let mut list_col = column![].width(Length::Fill);
    if top_pad > 0.0 {
        list_col = list_col.push(Space::new().width(Length::Fill).height(top_pad));
    }

    for &row_kind in &modal.flat_rows[first_visible..end_visible] {
        match row_kind {
            DuplicateListRow::GroupHeader(group_idx) => {
                if let Some(group) = modal.groups.get(group_idx) {
                    let collapsed = modal.collapsed_groups.contains(&group.group_id);
                    list_col = list_col.push(group_header_row(group, collapsed, palette, strings));
                }
            }
            DuplicateListRow::FileItem {
                group_idx,
                item_idx,
            } => {
                if let Some(group) = modal.groups.get(group_idx)
                    && let Some(item) = group.items.get(item_idx)
                {
                    let is_oldest = group
                        .oldest_mtime_secs()
                        .is_some_and(|min_t| item.modified_secs == Some(min_t));
                    list_col = list_col.push(file_item_row(
                        item,
                        is_oldest,
                        modal.deleting,
                        palette,
                        strings,
                    ));
                }
            }
        }
    }

    if bottom_pad > 0.0 {
        list_col = list_col.push(Space::new().width(Length::Fill).height(bottom_pad));
    }

    let scroll = scrollable(list_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(|vp| {
            Message::DuplicateFinder(DuplicateFinderMsg::ListScrolled {
                offset_y: vp.absolute_offset().y,
                viewport_height: vp.bounds().height,
            })
        });

    container(scroll)
        .padding([8, 12])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn scanning_body<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (stage_label, detail_text, ratio, current_file) = match &modal.progress {
        Some(p) => {
            let sname = stage_name(p.stage, strings);
            let title = format!("{} ({}/{})", sname, p.stage_index, p.stage_total);
            let (det, r) = if p.total > 0 {
                let pct =
                    ((p.processed as f32 / p.total as f32).clamp(0.0, 1.0) * 100.0).round() as u32;
                (
                    format!("{} / {} ({pct}%)", p.processed, p.total),
                    (p.processed as f32 / p.total as f32).clamp(0.0, 1.0),
                )
            } else {
                (format!("{} ...", p.processed), 0.25)
            };
            (title, det, r, sanitize_single_line(&p.current_path))
        }
        None => (
            strings.dup_stage_walking.to_owned(),
            strings.loading.to_owned(),
            0.1,
            String::new(),
        ),
    };

    let filled = ((ratio * 1000.0).round() as u16).max(1);
    let remaining = 1000u16.saturating_sub(filled).max(1);

    let bar = container(
        row![
            container(Space::new().width(Length::Fill).height(8))
                .width(Length::FillPortion(filled))
                .style(move |_| styles::panel_surface(
                    palette.accent_primary,
                    Color::TRANSPARENT,
                    9999.0
                )),
            Space::new().width(Length::FillPortion(remaining)).height(8),
        ]
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(1)
    .style(move |_| styles::panel_surface(palette.surface_high, palette.border, 9999.0));

    let card = container(
        column![
            row![
                text(stage_label)
                    .size(13)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                text(detail_text)
                    .size(12)
                    .font(FONT_MONO)
                    .color(palette.accent_primary),
            ]
            .align_y(Center),
            bar,
            text(current_file)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_muted)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(10)
        .width(Length::Fill),
    )
    .padding(16)
    .width(Length::Fixed(480.0))
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .into()
}

fn group_header_row<'a>(
    group: &'a DuplicateGroup,
    collapsed: bool,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let chevron: Element<'a, Message> = if collapsed {
        icon_chevron_right()
            .size(13)
            .color(palette.text_secondary)
            .into()
    } else {
        icon_chevron_down()
            .size(13)
            .color(palette.text_secondary)
            .into()
    };

    let (verify_label, verify_color) = match group.verified_level {
        DuplicateConfirmLevel::QuickPartialMd5 => {
            (strings.dup_verify_partial, palette.status_warning)
        }
        DuplicateConfirmLevel::StandardFullMd5 => (strings.dup_verify_md5, palette.status_success),
        DuplicateConfirmLevel::StrictMd5AndSha256 => {
            (strings.dup_verify_sha256, palette.status_success)
        }
        DuplicateConfirmLevel::ParanoidMd5AndByteCompare => {
            (strings.dup_verify_byte, palette.status_success)
        }
    };
    let verify_bg = Color::from_rgba(verify_color.r, verify_color.g, verify_color.b, 0.12);
    let verify_border = Color::from_rgba(verify_color.r, verify_color.g, verify_color.b, 0.40);

    let hash_text = match &group.sha256_prefix {
        Some(sha) => format!("md5: {} • sha256: {}", group.md5_short, sha),
        None => format!("md5: {}", group.md5_short),
    };

    let meta_text = format!(
        "{} {} • {} {}",
        group.items.len(),
        strings.dup_copies_word,
        format_bytes(group.size_bytes),
        strings.dup_each_word
    );

    let header_btn = button(
        row![
            chevron,
            text(format!("{} #{}", strings.dup_group_prefix, group.group_id))
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary),
            text(meta_text)
                .size(11)
                .font(FONT_UI)
                .color(palette.text_secondary),
            container(
                text(verify_label)
                    .size(10)
                    .font(FONT_MONO)
                    .color(verify_color),
            )
            .padding([2, 6])
            .style(styles::badge_container(
                verify_bg,
                verify_color,
                verify_border,
                5.0,
            )),
            space::horizontal(),
            text(hash_text)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_muted),
        ]
        .spacing(8)
        .align_y(Center)
        .width(Length::Fill),
    )
    .padding([4, 10])
    .width(Length::Fill)
    .height(Length::Fixed(LIST_ROW_HEIGHT - 4.0))
    .style(styles::ghost_button(palette, 6.0))
    .on_press(Message::DuplicateFinder(
        DuplicateFinderMsg::ToggleGroupExpanded(group.group_id),
    ));

    container(
        container(header_btn)
            .width(Length::Fill)
            .style(move |_| styles::panel_surface(palette.surface_high, palette.border, 6.0)),
    )
    .padding(Padding {
        top: 4.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    })
    .height(Length::Fixed(LIST_ROW_HEIGHT))
    .width(Length::Fill)
    .into()
}

fn file_item_row<'a>(
    item: &'a fm_core::DuplicateFileItem,
    is_oldest: bool,
    deleting: bool,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let path_for_msg = item.path.clone();
    let mut cb = checkbox(item.selected_for_delete).size(15);
    if !deleting {
        cb = cb.on_toggle(move |_| {
            Message::DuplicateFinder(DuplicateFinderMsg::ToggleItemSelected(path_for_msg.clone()))
        });
    }

    let path_display = sanitize_single_line(&item.relative_display);

    let mut left_row = row![
        cb,
        container(
            text(path_display)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None),
        )
        .width(Length::Fill)
        .clip(true),
    ]
    .spacing(8)
    .align_y(Center)
    .width(Length::Fill);

    if is_oldest {
        left_row = left_row.push(
            container(
                text(strings.dup_badge_oldest)
                    .size(10)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.accent_primary),
            )
            .padding([1, 6])
            .style(styles::badge_container(
                palette.accent_surface,
                palette.accent_primary,
                palette.accent_border,
                4.0,
            )),
        );
    }

    if item.is_hardlink_sibling {
        let hl_color = palette.accent_tertiary;
        let hl_bg = Color::from_rgba(hl_color.r, hl_color.g, hl_color.b, 0.14);
        let hl_border = Color::from_rgba(hl_color.r, hl_color.g, hl_color.b, 0.40);
        left_row = left_row.push(
            container(
                text(strings.dup_badge_hardlink)
                    .size(10)
                    .font(FONT_UI_SEMIBOLD)
                    .color(hl_color),
            )
            .padding([1, 6])
            .style(styles::badge_container(hl_bg, hl_color, hl_border, 4.0)),
        );
    }

    let date_label = item
        .modified_secs
        .and_then(|secs| u64::try_from(secs).ok())
        .map(|secs| strings.format_timestamp_short(UNIX_EPOCH + Duration::from_secs(secs)))
        .unwrap_or_else(|| "--".to_owned());

    let right_meta = format!("{} • {date_label}", format_bytes(item.size_bytes));

    container(
        row![
            left_row,
            text(right_meta)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_muted),
        ]
        .spacing(10)
        .align_y(Center)
        .width(Length::Fill),
    )
    .padding([0, 12])
    .height(Length::Fixed(LIST_ROW_HEIGHT))
    .width(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_low, Color::TRANSPARENT, 0.0))
    .into()
}

fn footer_bar<'a>(
    modal: &'a DuplicateFinderState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let total_selected = modal.total_selected_count();
    let any_all_selected = modal.any_group_all_selected();
    let total_reclaimable = modal.total_reclaimable_bytes();

    let status_text = format!(
        "{} {} • {} {}",
        modal.groups.len(),
        strings.dup_groups_found,
        format_bytes(total_reclaimable),
        strings.dup_reclaimable
    );

    let can_trash = total_selected > 0 && !any_all_selected && !modal.scanning && !modal.deleting;
    let can_perm = can_trash && modal.options.confirm_level.allows_permanent_delete();

    let actions: Element<'a, Message> = if modal.confirm_perm_delete {
        row![
            text(strings.dup_confirm_perm_delete)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.status_error),
            button(text(strings.cancel).size(12))
                .padding([6, 12])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::DuplicateFinder(
                    DuplicateFinderMsg::CancelPermanentDeleteConfirm,
                )),
            button(
                text(strings.confirm)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.status_error),
            )
            .padding([6, 14])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::DuplicateFinder(
                DuplicateFinderMsg::ConfirmPermanentDelete,
            )),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    } else {
        let mut perm_btn = button(
            text(strings.dup_delete_permanent)
                .size(12)
                .font(FONT_UI)
                .color(if can_perm {
                    palette.status_error
                } else {
                    palette.text_muted
                }),
        )
        .padding([6, 12])
        .style(styles::pill_secondary_button(palette));
        if can_perm {
            perm_btn = perm_btn.on_press(Message::DuplicateFinder(
                DuplicateFinderMsg::RequestPermanentDelete,
            ));
        }

        let mut trash_btn = button(
            row![
                icon_trash_2().size(13),
                text(format!("{} ({total_selected})", strings.dup_move_to_trash))
                    .size(12)
                    .font(FONT_UI_SEMIBOLD),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([7, 14])
        .style(styles::pill_primary_button(palette));
        if can_trash {
            trash_btn = trash_btn.on_press(Message::DuplicateFinder(
                DuplicateFinderMsg::RequestMoveToTrash,
            ));
        }

        row![perm_btn, trash_btn].spacing(8).align_y(Center).into()
    };

    let resize_grip = mouse_area(
        container(text("◢").size(10).color(palette.text_muted))
            .padding([2, 4])
            .align_x(Center)
            .align_y(Center),
    )
    .interaction(mouse::Interaction::ResizingDiagonallyDown)
    .on_press(Message::DuplicateFinder(DuplicateFinderMsg::ResizeStart));

    container(
        row![
            text(status_text)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_secondary),
            space::horizontal(),
            actions,
            resize_grip,
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([9, 14])
    .width(Length::Fill)
    .into()
}

pub fn stage_badge_text(modal: &DuplicateFinderState, strings: Strings) -> String {
    if let Some(p) = &modal.progress {
        let sname = stage_name(p.stage, strings);
        if p.total > 0 {
            let pct =
                ((p.processed as f32 / p.total as f32).clamp(0.0, 1.0) * 100.0).round() as u32;
            return format!("{}/{}: {} ({pct}%)", p.stage_index, p.stage_total, sname);
        }
        return format!("{}/{}: {}", p.stage_index, p.stage_total, sname);
    }

    match modal.options.confirm_level {
        DuplicateConfirmLevel::QuickPartialMd5 => format!("2/2: {}", strings.dup_stage_partial),
        DuplicateConfirmLevel::StandardFullMd5 => format!("3/3: {}", strings.dup_stage_full),
        DuplicateConfirmLevel::StrictMd5AndSha256 => format!("4/4: {}", strings.dup_stage_sha256),
        DuplicateConfirmLevel::ParanoidMd5AndByteCompare => {
            format!("4/4: {}", strings.dup_stage_byte)
        }
    }
}

fn stage_name(stage: DuplicateScanStage, strings: Strings) -> &'static str {
    match stage {
        DuplicateScanStage::Walking => strings.dup_stage_walking,
        DuplicateScanStage::PartialMd5 => strings.dup_stage_partial,
        DuplicateScanStage::FullMd5 => strings.dup_stage_full,
        DuplicateScanStage::Sha256Verify => strings.dup_stage_sha256,
        DuplicateScanStage::ByteCompare => strings.dup_stage_byte,
        DuplicateScanStage::Finished => strings.dup_stage_full,
    }
}
