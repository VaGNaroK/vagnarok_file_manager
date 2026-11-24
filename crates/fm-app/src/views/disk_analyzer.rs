use fm_core::{DiskCategory, DiskMinSizeFilter, Strings, format_bytes, format_grouped_u64};
use fm_theme::{FONT_MONO, FONT_MONO_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles};
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, opaque, row, scrollable, text,
    text_input,
};
use iced::{Alignment::Center, Background, Border, Color, Element, Length, Padding};
use lucide_icons::iced::{
    icon_folder, icon_hard_drive, icon_minus, icon_pie_chart, icon_rotate_ccw, icon_search,
    icon_trash_2, icon_x,
};

use crate::app::{App, Message};
use crate::disk_analyzer::{
    DISK_MODAL_MIN_H, DISK_MODAL_MIN_W, DiskAnalyzerMsg, DiskAnalyzerState, DiskListMode,
};
use crate::views::virtualized::compute_fixed_window;
use crate::views::{modal_overlay_backdrop_at, sanitize_single_line, with_tooltip};

const ROW_STEP: f32 = 48.0;

pub fn disk_analyzer_modal_overlay<'a>(
    app: &'a App,
    modal: &'a DiskAnalyzerState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (x, y, modal_w, modal_h) = crate::bulk_rename::effective_modal_rect(
        modal.modal_size,
        modal.modal_offset,
        DISK_MODAL_MIN_W,
        DISK_MODAL_MIN_H,
        app.window_size,
    );

    let inner: Element<'a, Message> = if modal.scanning || modal.result.is_none() {
        render_scanning_view(modal, palette, strings)
    } else {
        render_results_view(modal, palette, strings)
    };

    let card = container(opaque(inner))
        .width(Length::Fixed(modal_w))
        .height(Length::Fixed(modal_h))
        .clip(true)
        .style(styles::modal_card(palette));

    modal_overlay_backdrop_at(card, x, y)
}

fn render_scanning_view<'a>(
    modal: &'a DiskAnalyzerState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let prog = modal.progress.as_ref();
    let files_scanned = prog.map(|p| p.files_scanned).unwrap_or(0);
    let bytes_scanned = prog.map(|p| p.bytes_scanned).unwrap_or(0);
    let elapsed_secs = prog.map(|p| p.elapsed_secs).unwrap_or(0);
    let current_path = prog
        .map(|p| sanitize_single_line(&p.current_path.to_string_lossy()))
        .or_else(|| {
            modal
                .roots
                .first()
                .map(|r| sanitize_single_line(&r.to_string_lossy()))
        })
        .unwrap_or_default();

    let header_drag = mouse_area(
        row![
            container(icon_search().size(22).color(palette.status_warning))
                .width(Length::Fixed(52.0))
                .height(Length::Fixed(52.0))
                .align_x(Center)
                .align_y(Center)
                .style(move |_| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.14,
                        ..palette.status_warning
                    })),
                    border: Border {
                        color: palette.status_warning,
                        width: 2.0,
                        radius: 26.0.into(),
                    },
                    ..Default::default()
                }),
            column![
                text(strings.disk_scan_eyebrow)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.status_warning),
                text(strings.disk_scan_title)
                    .size(18)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
            ]
            .spacing(2)
            .width(Length::Fill),
        ]
        .spacing(14)
        .align_y(Center),
    )
    .interaction(if modal.dragging_modal.is_some() {
        iced::mouse::Interaction::Grabbing
    } else {
        iced::mouse::Interaction::Grab
    })
    .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DragStart));

    let current_item_card = container(
        row![
            icon_folder().size(18).color(palette.text_secondary),
            column![
                row![
                    text(strings.disk_scan_current_item)
                        .size(11)
                        .color(palette.text_muted),
                    iced::widget::space::horizontal(),
                    text(format!("{elapsed_secs} seg"))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_muted),
                ]
                .width(Length::Fill),
                container(
                    text(current_path)
                        .size(12)
                        .font(FONT_MONO)
                        .color(palette.text_primary)
                        .wrapping(text::Wrapping::None),
                )
                .width(Length::Fill)
                .clip(true),
            ]
            .spacing(3)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0));

    let progress_strip = container(
        container(Space::new().width(Length::FillPortion(38)).height(5))
            .style(move |_| styles::panel_surface(palette.status_warning, Color::TRANSPARENT, 3.0)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(5.0))
    .style(move |_| styles::panel_surface(palette.surface_high, Color::TRANSPARENT, 3.0));

    let metrics_row = row![
        metric_tile(
            strings.disk_scan_files_checked,
            format_grouped_u64(files_scanned, '.').to_string(),
            palette
        ),
        metric_tile(
            strings.disk_scan_data_checked,
            format_bytes(bytes_scanned).to_string(),
            palette
        ),
        metric_tile(
            strings.disk_scan_elapsed,
            format!("{elapsed_secs} seg"),
            palette
        ),
    ]
    .spacing(12)
    .width(Length::Fill);

    let actions = row![
        button(text(strings.arch_minimize).size(12).font(FONT_UI_SEMIBOLD))
            .padding([8, 18])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::Minimize)),
        button(text(strings.cancel).size(12).font(FONT_UI_SEMIBOLD))
            .padding([8, 18])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::Close)),
    ]
    .spacing(10)
    .align_y(Center);

    container(
        column![
            row![
                header_drag,
                button(icon_x().size(14))
                    .padding([5, 7])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::Close)),
            ]
            .align_y(Center),
            Space::new().height(12),
            current_item_card,
            progress_strip,
            metrics_row,
            container(
                text(strings.disk_scan_hint)
                    .size(12)
                    .color(palette.text_muted),
            )
            .width(Length::Fill)
            .align_x(Center),
            container(actions).width(Length::Fill).align_x(Center),
        ]
        .spacing(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center),
    )
    .padding(24)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn metric_tile<'a>(label: &'a str, value: String, palette: ThemePalette) -> Element<'a, Message> {
    container(
        column![
            text(label).size(11).color(palette.text_muted),
            text(value)
                .size(16)
                .font(FONT_MONO_BOLD)
                .color(palette.text_primary),
        ]
        .spacing(4),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 10.0))
    .into()
}

fn render_results_view<'a>(
    modal: &'a DiskAnalyzerState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let Some(res) = &modal.result else {
        return Space::new().into();
    };

    let mut crumbs = row![
        button(
            text(format!(
                "{} ({})",
                strings.disk_root_crumb,
                format_bytes(res.total_bytes)
            ))
            .size(11)
            .font(FONT_UI_SEMIBOLD)
            .color(if modal.current_node_id.is_none() {
                palette.accent_primary
            } else {
                palette.text_secondary
            }),
        )
        .padding([3, 8])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DrillRoot)),
    ]
    .spacing(4)
    .align_y(Center);

    for (nid_opt, label) in modal.breadcrumb_nodes() {
        if let Some(nid) = nid_opt {
            let is_cur = modal.current_node_id == Some(nid);
            crumbs = crumbs
                .push(text("›").size(12).color(palette.text_muted))
                .push(
                    button(
                        text(sanitize_single_line(&label))
                            .size(11)
                            .font(FONT_UI_SEMIBOLD)
                            .color(if is_cur {
                                palette.accent_primary
                            } else {
                                palette.text_secondary
                            }),
                    )
                    .padding([3, 8])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DrillIntoNode(nid))),
                );
        }
    }

    let header_drag = mouse_area(
        row![
            container(icon_pie_chart().size(16).color(palette.status_warning))
                .padding(7)
                .style(move |_| styles::panel_surface(
                    Color {
                        a: 0.14,
                        ..palette.status_warning
                    },
                    palette.status_warning,
                    8.0
                )),
            column![
                text(format!(
                    "{} — {} ({} {})",
                    strings.disk_results_title,
                    format_bytes(res.total_bytes),
                    format_grouped_u64(res.total_files, '.'),
                    strings.files.to_ascii_lowercase()
                ))
                .size(14)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary),
                crumbs,
            ]
            .spacing(3)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .interaction(if modal.dragging_modal.is_some() {
        iced::mouse::Interaction::Grabbing
    } else {
        iced::mouse::Interaction::Grab
    })
    .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DragStart));

    let header = row![
        header_drag,
        with_tooltip(
            button(icon_rotate_ccw().size(13))
                .padding([6, 10])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::StartScan)),
            strings.disk_reanalyze,
            iced::widget::tooltip::Position::Bottom,
            palette,
        ),
        button(icon_minus().size(14))
            .padding([5, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::Minimize)),
        button(icon_x().size(14))
            .padding([5, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::Close)),
    ]
    .spacing(6)
    .align_y(Center);

    // Painel esquerdo: Treemap proporcional + Top consumidores + Barra por tipo
    let left_panel = render_charts_panel(modal, res, palette, strings);

    // Painel direito: Busca + Filtros + Lista virtualizada com checkboxes
    let right_panel = render_cleanup_list_panel(modal, palette, strings);

    let body = row![
        container(left_panel).width(Length::FillPortion(10)),
        container(right_panel).width(Length::FillPortion(12)),
    ]
    .spacing(14)
    .width(Length::Fill)
    .height(Length::Fill);

    // Rodapé com totalizador de espaço a liberar e botões de exclusão
    let sel_count = modal.selected_paths.len();
    let sel_bytes = modal.selected_bytes();
    let summary_text = format!(
        "{sel_count} {} • {} {}",
        if sel_count == 1 { "item" } else { "itens" },
        format_bytes(sel_bytes),
        strings.disk_reclaim_suffix
    );

    let footer_actions: Element<'a, Message> = if modal.confirm_perm_delete {
        row![
            text(strings.dup_confirm_perm_delete)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.status_error),
            button(text(strings.cancel).size(12))
                .padding([6, 12])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::DiskAnalyzer(
                    DiskAnalyzerMsg::CancelPermanentDeleteConfirm
                )),
            button(
                text(strings.delete_permanent)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
            )
            .padding([6, 14])
            .style(move |_, _| button::Style {
                background: Some(Background::Color(palette.status_error)),
                text_color: Color::WHITE,
                border: Border {
                    radius: 9999.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .on_press(Message::DiskAnalyzer(
                DiskAnalyzerMsg::ConfirmPermanentDelete
            )),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    } else {
        let can_act = sel_count > 0 && !modal.deleting;
        let mut clear_btn = button(text(strings.disk_clear_selection).size(12))
            .padding([6, 12])
            .style(styles::pill_secondary_button(palette));
        let mut perm_btn = button(text(strings.delete_permanent).size(12))
            .padding([6, 12])
            .style(styles::pill_secondary_button(palette));
        let mut trash_btn = button(
            row![
                icon_trash_2().size(13),
                text(strings.move_to_trash).size(12).font(FONT_UI_SEMIBOLD),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([6, 14])
        .style(styles::pill_primary_button(palette));

        if can_act {
            clear_btn = clear_btn.on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::ClearSelection));
            perm_btn = perm_btn.on_press(Message::DiskAnalyzer(
                DiskAnalyzerMsg::RequestPermanentDelete,
            ));
            trash_btn =
                trash_btn.on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::RequestMoveToTrash));
        }

        row![clear_btn, perm_btn, trash_btn]
            .spacing(8)
            .align_y(Center)
            .into()
    };

    let resize_grip = mouse_area(
        container(text("◢").size(12).color(palette.text_muted))
            .padding([2, 4])
            .align_x(Center)
            .align_y(Center),
    )
    .interaction(iced::mouse::Interaction::ResizingDiagonallyDown)
    .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::ResizeStart));

    let footer = row![
        text(summary_text)
            .size(13)
            .font(FONT_UI_SEMIBOLD)
            .color(if sel_count > 0 {
                palette.status_error
            } else {
                palette.text_secondary
            }),
        iced::widget::space::horizontal(),
        footer_actions,
        resize_grip,
    ]
    .spacing(10)
    .align_y(Center)
    .width(Length::Fill);

    container(
        column![
            header,
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            body,
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            footer,
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(18)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn render_charts_panel<'a>(
    modal: &'a DiskAnalyzerState,
    res: &'a fm_core::DiskScanResult,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let scope_bytes = modal.current_scope_bytes().max(1);
    let child_ids: &[usize] = if let Some(nid) = modal.current_node_id {
        res.nodes
            .get(nid)
            .map(|n| n.children.as_slice())
            .unwrap_or(&[])
    } else if res.root_node_ids.len() == 1 {
        res.nodes
            .get(res.root_node_ids[0])
            .map(|n| n.children.as_slice())
            .unwrap_or(&[])
    } else {
        res.root_node_ids.as_slice()
    };

    let chart_colors = [
        Color::from_rgb8(0x25, 0x63, 0xEB),
        Color::from_rgb8(0x7C, 0x3A, 0xED),
        Color::from_rgb8(0xD9, 0x77, 0x06),
        Color::from_rgb8(0x05, 0x96, 0x69),
        Color::from_rgb8(0x08, 0x91, 0xB2),
    ];

    // Treemap proporcional com as 3 maiores subpastas do nível atual
    let treemap_widget: Element<'a, Message> = if child_ids.is_empty() {
        container(
            text("Nenhuma subpasta neste nível")
                .size(12)
                .color(palette.text_muted),
        )
        .width(Length::Fill)
        .height(Length::Fixed(128.0))
        .align_x(Center)
        .align_y(Center)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 10.0))
        .into()
    } else {
        let tile = |cid: usize, color: Color, h: f32| -> Element<'a, Message> {
            let node = &res.nodes[cid];
            let pct = (node.total_bytes as f64 / scope_bytes as f64 * 100.0).clamp(0.0, 100.0);
            button(
                column![
                    text(sanitize_single_line(&node.name))
                        .size(12)
                        .font(FONT_UI_SEMIBOLD)
                        .color(Color::WHITE)
                        .wrapping(text::Wrapping::None),
                    iced::widget::space::vertical(),
                    row![
                        text(format_bytes(node.total_bytes).to_string())
                            .size(13)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE),
                        iced::widget::space::horizontal(),
                        text(format!("{pct:.0}%"))
                            .size(11)
                            .font(FONT_MONO)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.88)),
                    ]
                    .align_y(Center),
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fixed(h))
            .padding(10)
            .style(move |_, _| button::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 9.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DrillIntoNode(cid)))
            .into()
        };

        let first = tile(child_ids[0], chart_colors[0], 146.0);
        if child_ids.len() == 1 {
            first
        } else if child_ids.len() == 2 {
            row![
                container(first).width(Length::FillPortion(3)),
                container(tile(child_ids[1], chart_colors[1], 146.0)).width(Length::FillPortion(2)),
            ]
            .spacing(6)
            .into()
        } else {
            row![
                container(first).width(Length::FillPortion(3)),
                column![
                    tile(child_ids[1], chart_colors[1], 70.0),
                    tile(child_ids[2], chart_colors[2], 70.0),
                ]
                .spacing(6)
                .width(Length::FillPortion(2)),
            ]
            .spacing(6)
            .into()
        }
    };

    // Barras horizontais dos maiores consumidores
    let mut bars = column![].spacing(6).width(Length::Fill);
    for (idx, &cid) in child_ids.iter().take(5).enumerate() {
        let node = &res.nodes[cid];
        let pct = (node.total_bytes as f32 / scope_bytes as f32).clamp(0.0, 1.0);
        let fill_parts = ((pct * 100.0).round() as u16).max(2);
        let empty_parts = 100u16.saturating_sub(fill_parts);
        let bar_color = chart_colors[idx % chart_colors.len()];

        let track = if empty_parts > 0 {
            row![
                container(Space::new().width(Length::Fill).height(7))
                    .width(Length::FillPortion(fill_parts))
                    .style(move |_| styles::panel_surface(bar_color, Color::TRANSPARENT, 4.0)),
                Space::new()
                    .width(Length::FillPortion(empty_parts))
                    .height(7),
            ]
        } else {
            row![
                container(Space::new().width(Length::Fill).height(7))
                    .width(Length::Fill)
                    .style(move |_| styles::panel_surface(bar_color, Color::TRANSPARENT, 4.0)),
            ]
        };

        bars = bars.push(
            button(
                column![
                    row![
                        container(
                            text(sanitize_single_line(&node.name))
                                .size(12)
                                .font(FONT_UI_SEMIBOLD)
                                .color(palette.text_primary)
                                .wrapping(text::Wrapping::None),
                        )
                        .width(Length::Fill)
                        .clip(true),
                        text(format!(
                            "{} ({:.0}%)",
                            format_bytes(node.total_bytes),
                            pct * 100.0
                        ))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_secondary),
                    ]
                    .spacing(6)
                    .align_y(Center),
                    container(track)
                        .width(Length::Fill)
                        .style(move |_| styles::panel_surface(
                            palette.surface_high,
                            Color::TRANSPARENT,
                            4.0
                        )),
                ]
                .spacing(4),
            )
            .padding([5, 7])
            .width(Length::Fill)
            .style(styles::ghost_button(palette, 7.0))
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DrillIntoNode(cid))),
        );
    }

    // Distribuição por categoria
    let total_all = res.total_bytes.max(1);
    let mut strip = row![].spacing(2).width(Length::Fill);
    let mut legend_items: Vec<Element<'a, Message>> = Vec::new();
    for (idx, (cat, bytes)) in res.category_totals.iter().enumerate() {
        if *bytes == 0 {
            continue;
        }
        let color = chart_colors[idx % chart_colors.len()];
        let portion = ((*bytes as f64 / total_all as f64) * 100.0).round() as u16;
        if portion > 0 {
            strip = strip.push(
                container(Space::new().width(Length::Fill).height(8))
                    .width(Length::FillPortion(portion))
                    .style(move |_| styles::panel_surface(color, Color::TRANSPARENT, 4.0)),
            );
        }
        let cat_name = match cat {
            DiskCategory::IsoAndDisks => strings.disk_cat_iso,
            DiskCategory::BuildAndCache => strings.disk_cat_build,
            DiskCategory::Archives => strings.disk_cat_archives,
            DiskCategory::Media => strings.disk_cat_media,
            DiskCategory::Documents => strings.disk_cat_docs,
            DiskCategory::Other => strings.disk_cat_other,
        };
        legend_items.push(
            row![
                container(Space::new().width(8).height(8))
                    .style(move |_| { styles::panel_surface(color, Color::TRANSPARENT, 4.0) }),
                text(format!("{cat_name}: {}", format_bytes(*bytes)))
                    .size(11)
                    .color(palette.text_secondary),
            ]
            .spacing(4)
            .align_y(Center)
            .into(),
        );
    }
    let legend = iced::widget::row(legend_items).spacing(10).wrap();

    container(
        column![
            row![
                text(strings.disk_treemap_title)
                    .size(11)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_muted),
                iced::widget::space::horizontal(),
                text(format_bytes(modal.current_scope_bytes()).to_string())
                    .size(11)
                    .font(FONT_MONO_BOLD)
                    .color(palette.text_primary),
            ],
            treemap_widget,
            text(strings.disk_top_consumers)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_muted),
            scrollable(bars).height(Length::Fill),
            text(strings.disk_by_type)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_muted),
            strip,
            legend,
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(14)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 12.0))
    .into()
}

fn render_cleanup_list_panel<'a>(
    modal: &'a DiskAnalyzerState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let mode_btn = |mode: DiskListMode, label: &'a str| {
        let active = modal.list_mode == mode;
        button(
            text(label)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(if active {
                    palette.on_accent
                } else {
                    palette.text_secondary
                }),
        )
        .padding([5, 10])
        .style(move |theme, status| {
            if active {
                styles::pill_primary_button(palette)(theme, status)
            } else {
                styles::pill_secondary_button(palette)(theme, status)
            }
        })
        .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::SetListMode(mode)))
    };

    let size_chip = |filter: DiskMinSizeFilter, label: &'a str| {
        let active = modal.min_size == filter;
        button(
            text(label)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(if active {
                    palette.on_accent
                } else {
                    palette.text_secondary
                }),
        )
        .padding([4, 9])
        .style(move |theme, status| {
            if active {
                styles::pill_primary_button(palette)(theme, status)
            } else {
                styles::pill_secondary_button(palette)(theme, status)
            }
        })
        .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::SetMinSize(filter)))
    };

    let controls = column![
        row![
            mode_btn(DiskListMode::LargestFiles, strings.disk_mode_files),
            mode_btn(DiskListMode::Folders, strings.disk_mode_folders),
            iced::widget::space::horizontal(),
            text(format!("{} itens", modal.visible_rows.len()))
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_muted),
        ]
        .spacing(6)
        .align_y(Center),
        row![
            text_input(strings.search_placeholder, &modal.search_query)
                .size(12)
                .padding([6, 10])
                .width(Length::Fill)
                .style(styles::search_input_style(palette))
                .on_input(|s| Message::DiskAnalyzer(DiskAnalyzerMsg::SetSearchQuery(s))),
            size_chip(DiskMinSizeFilter::All, "Todos"),
            size_chip(DiskMinSizeFilter::Mb100, ">100MB"),
            size_chip(DiskMinSizeFilter::Gb1, ">1GB"),
            size_chip(DiskMinSizeFilter::Gb5, ">5GB"),
        ]
        .spacing(6)
        .align_y(Center),
    ]
    .spacing(8);

    let win = compute_fixed_window(
        modal.visible_rows.len(),
        ROW_STEP,
        modal.scroll_y,
        modal.viewport_height,
        6,
    );

    let mut list_col = column![].spacing(6).width(Length::Fill);
    if win.top_spacer > 0.0 {
        list_col = list_col.push(Space::new().height(win.top_spacer));
    }

    for row_item in &modal.visible_rows[win.range] {
        let checked = modal.selected_paths.contains(&row_item.path);
        let path_toggle = row_item.path.clone();
        let path_loc = row_item.path.clone();
        let node_id = row_item.node_id;

        let name_widget: Element<'a, Message> = if let Some(nid) = node_id {
            button(
                text(sanitize_single_line(&row_item.name))
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.accent_primary)
                    .wrapping(text::Wrapping::None),
            )
            .padding(0)
            .style(styles::ghost_button(palette, 4.0))
            .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::DrillIntoNode(nid)))
            .into()
        } else {
            text(sanitize_single_line(&row_item.name))
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None)
                .into()
        };

        let row_card = container(
            row![
                checkbox(checked)
                    .on_toggle(move |_| {
                        Message::DiskAnalyzer(DiskAnalyzerMsg::ToggleItemSelected(
                            path_toggle.clone(),
                        ))
                    })
                    .size(16),
                column![
                    container(name_widget).width(Length::Fill).clip(true),
                    container(
                        text(sanitize_single_line(&row_item.parent_display))
                            .size(10)
                            .font(FONT_MONO)
                            .color(palette.text_muted)
                            .wrapping(text::Wrapping::None),
                    )
                    .width(Length::Fill)
                    .clip(true),
                ]
                .spacing(2)
                .width(Length::Fill),
                text(format_bytes(row_item.size_bytes).to_string())
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(palette.text_primary),
                with_tooltip(
                    button(icon_folder().size(13).color(palette.text_secondary))
                        .padding([4, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::OpenItemLocation(
                            path_loc
                        ))),
                    strings.open_file_location,
                    iced::widget::tooltip::Position::Bottom,
                    palette,
                ),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([6, 10])
        .height(Length::Fixed(42.0))
        .width(Length::Fill)
        .style(move |_| {
            if checked {
                styles::panel_surface(
                    Color {
                        a: 0.12,
                        ..palette.status_error
                    },
                    palette.status_error,
                    8.0,
                )
            } else {
                styles::panel_surface(palette.surface_lowest, palette.border, 8.0)
            }
        });

        list_col = list_col.push(row_card);
    }

    if win.bottom_spacer > 0.0 {
        list_col = list_col.push(Space::new().height(win.bottom_spacer));
    }

    let scroll = scrollable(list_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(|vp| {
            Message::DiskAnalyzer(DiskAnalyzerMsg::ListScrolled {
                offset_y: vp.absolute_offset().y,
                viewport_height: vp.bounds().height,
            })
        });

    container(
        column![
            text(strings.disk_mark_cleanup)
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_muted),
            controls,
            scroll,
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(Padding::from([14, 14]))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 12.0))
    .into()
}

pub fn disk_analyzer_minimized_pill<'a>(
    modal: &'a DiskAnalyzerState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let label = if modal.scanning {
        let count = modal
            .progress
            .as_ref()
            .map(|p| p.files_scanned)
            .unwrap_or(0);
        format!(
            "{} ({})",
            strings.disk_scan_eyebrow,
            format_grouped_u64(count, '.')
        )
    } else {
        strings.disk_results_title.to_owned()
    };

    button(
        row![
            icon_hard_drive().size(13).color(palette.status_warning),
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
    .on_press(Message::DiskAnalyzer(DiskAnalyzerMsg::Restore))
    .into()
}
