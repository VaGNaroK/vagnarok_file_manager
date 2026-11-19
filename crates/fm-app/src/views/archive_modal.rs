use std::fmt;

use fm_core::{
    ArchiveFormat, ArchiveJobKind, CompressionLevel, ConflictPolicy, Strings, format_bytes,
};
use fm_theme::{FONT_MONO, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, mouse_area, pick_list, row, space, text, text_input,
};
use iced::{Color, Element, Length, mouse};
use lucide_icons::iced::{icon_file_archive, icon_minus, icon_x};

use crate::app::{App, Message};
use crate::archive_jobs::{
    ArchiveUiMsg, COMPRESS_MODAL_W, CompressModalState, effective_compress_modal_pos,
};
use crate::views::{modal_overlay_backdrop_at, sanitize_single_line, with_tooltip};

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

/// Renderiza a janela flutuante "Compactar como..." (arrastável e minimizável).
pub fn compress_modal_overlay<'a>(
    app: &'a App,
    modal: &'a CompressModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (x, y) = effective_compress_modal_pos(app.window_size, modal);
    let modal_w = COMPRESS_MODAL_W.min((app.window_size.width - 16.0).max(360.0));

    let title_str = match modal.running_job_id {
        Some(job_id) => match app.archive_jobs.get(&job_id) {
            Some(job) => match job.progress.kind {
                ArchiveJobKind::Compress { .. } => {
                    format!("{}: {}", strings.arch_compressing, job.progress.title)
                }
                ArchiveJobKind::Extract { .. } => {
                    format!("{}: {}", strings.arch_extracting, job.progress.title)
                }
            },
            None => strings.arch_modal_title.to_owned(),
        },
        None => strings.arch_modal_title.to_owned(),
    };

    let drag_header = mouse_area(
        row![
            container(icon_file_archive().size(15).color(palette.accent_primary))
                .padding(6)
                .style(move |_| styles::panel_surface(
                    palette.accent_surface,
                    palette.accent_border,
                    8.0
                )),
            container(
                text(title_str)
                    .size(14)
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
    .interaction(if modal.dragging_modal.is_some() {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    })
    .on_press(Message::Archive(ArchiveUiMsg::DragStart));

    let minimize_btn = with_tooltip(
        button(icon_minus().size(14))
            .padding([5, 7])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::Archive(ArchiveUiMsg::MinimizeModal)),
        strings.arch_minimize,
        iced::widget::tooltip::Position::Bottom,
        palette,
    );

    let close_btn = button(icon_x().size(14))
        .padding([5, 7])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::Archive(ArchiveUiMsg::CloseModal));

    let header = row![
        container(drag_header).width(Length::Fill),
        minimize_btn,
        close_btn,
    ]
    .spacing(6)
    .align_y(Center);

    let body = if let Some(job_id) = modal.running_job_id {
        running_progress_body(app, job_id, palette, strings)
    } else {
        config_form_body(modal, palette, strings)
    };

    let card = container(
        column![
            header,
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            body,
        ]
        .spacing(12)
        .padding(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(modal_w))
    .style(styles::modal_card(palette));

    modal_overlay_backdrop_at(card, x, y)
}

fn config_form_body<'a>(
    modal: &'a CompressModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let summary_text = if modal.total_source_bytes > 0 {
        format!(
            "{} • {}",
            strings.format_items_count(modal.sources.len()),
            format_bytes(modal.total_source_bytes)
        )
    } else {
        strings.format_items_count(modal.sources.len())
    };
    let dest_display = sanitize_single_line(&modal.dest_dir.to_string_lossy());

    let info_banner = container(
        column![
            row![
                text(summary_text)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_primary),
                space::horizontal(),
                container(
                    text(modal.full_output_filename())
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.accent_primary)
                )
                .padding([2, 8])
                .style(styles::badge_container(
                    palette.accent_surface,
                    palette.accent_primary,
                    palette.accent_border,
                    6.0,
                )),
            ]
            .align_y(Center),
            text(format!("{}: {dest_display}", strings.arch_destination))
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_muted)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(4),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .clip(true)
    .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0));

    let name_field = column![
        text(strings.arch_output_name)
            .size(11)
            .font(FONT_UI_BOLD)
            .color(palette.text_secondary),
        container(
            text_input(strings.arch_output_name, &modal.archive_name)
                .on_input(|s| Message::Archive(ArchiveUiMsg::NameChanged(s)))
                .on_submit(Message::Archive(ArchiveUiMsg::StartCompress))
                .size(13)
                .padding([6, 10])
                .style(styles::search_input_style(palette)),
        )
        .padding([1, 4])
        .width(Length::Fill)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 8.0)),
    ]
    .spacing(4);

    let format_opts = vec![
        LabeledOption {
            value: ArchiveFormat::SevenZ,
            label: strings.arch_format_7z,
        },
        LabeledOption {
            value: ArchiveFormat::Zip,
            label: strings.arch_format_zip,
        },
        LabeledOption {
            value: ArchiveFormat::Tar,
            label: strings.arch_format_tar,
        },
        LabeledOption {
            value: ArchiveFormat::TarGz,
            label: strings.arch_format_targz,
        },
    ];
    let selected_format = format_opts
        .iter()
        .copied()
        .find(|o| o.value == modal.format);

    let format_field = column![
        text(strings.arch_format_label)
            .size(11)
            .font(FONT_UI_BOLD)
            .color(palette.text_secondary),
        pick_list(format_opts, selected_format, |opt| {
            Message::Archive(ArchiveUiMsg::SetFormat(opt.value))
        })
        .text_size(12)
        .font(FONT_UI)
        .padding([7, 10])
        .width(Length::Fill)
        .style(styles::pick_list_style(palette))
        .menu_style(styles::dropdown_menu_style(palette)),
    ]
    .spacing(4);

    // Ordem obrigatória: Armazenar -> Mínimo -> Normal -> Máximo (última escolha).
    let level_opts = vec![
        LabeledOption {
            value: CompressionLevel::Store,
            label: strings.arch_level_store,
        },
        LabeledOption {
            value: CompressionLevel::Minimum,
            label: strings.arch_level_min,
        },
        LabeledOption {
            value: CompressionLevel::Normal,
            label: strings.arch_level_normal,
        },
        LabeledOption {
            value: CompressionLevel::Maximum,
            label: strings.arch_level_max,
        },
    ];

    let level_control: Element<'a, Message> = if modal.format.supports_compression_level() {
        let selected_level = level_opts.iter().copied().find(|o| o.value == modal.level);
        pick_list(level_opts, selected_level, |opt| {
            Message::Archive(ArchiveUiMsg::SetLevel(opt.value))
        })
        .text_size(12)
        .font(FONT_UI)
        .padding([7, 10])
        .width(Length::Fill)
        .style(styles::pick_list_style(palette))
        .menu_style(styles::dropdown_menu_style(palette))
        .into()
    } else {
        container(
            column![
                text(strings.arch_level_store)
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(palette.text_muted),
                text(strings.arch_tar_store_hint)
                    .size(11)
                    .color(palette.text_muted),
            ]
            .spacing(2),
        )
        .padding([7, 10])
        .width(Length::Fill)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0))
        .into()
    };

    let level_field = column![
        text(strings.arch_level_label)
            .size(11)
            .font(FONT_UI_BOLD)
            .color(palette.text_secondary),
        level_control,
    ]
    .spacing(4);

    let conflict_opts = vec![
        LabeledOption {
            value: ConflictPolicy::RenameAuto,
            label: strings.arch_conflict_rename,
        },
        LabeledOption {
            value: ConflictPolicy::Overwrite,
            label: strings.arch_conflict_overwrite,
        },
        LabeledOption {
            value: ConflictPolicy::Skip,
            label: strings.arch_conflict_skip,
        },
    ];
    let selected_conflict = conflict_opts
        .iter()
        .copied()
        .find(|o| o.value == modal.conflict);

    let conflict_field = column![
        text(strings.arch_conflict_label)
            .size(11)
            .font(FONT_UI_BOLD)
            .color(palette.text_secondary),
        pick_list(conflict_opts, selected_conflict, |opt| {
            Message::Archive(ArchiveUiMsg::SetConflict(opt.value))
        })
        .text_size(12)
        .font(FONT_UI)
        .padding([7, 10])
        .width(Length::Fill)
        .style(styles::pick_list_style(palette))
        .menu_style(styles::dropdown_menu_style(palette)),
    ]
    .spacing(4);

    let can_start = !modal.archive_name.trim().is_empty() && !modal.sources.is_empty();
    let mut start_btn = button(
        text(strings.arch_start_compress)
            .size(12)
            .font(FONT_UI_SEMIBOLD),
    )
    .padding([7, 16])
    .style(styles::pill_primary_button(palette));
    if can_start {
        start_btn = start_btn.on_press(Message::Archive(ArchiveUiMsg::StartCompress));
    }

    let actions = row![
        button(text(strings.arch_minimize).size(12))
            .padding([7, 12])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::Archive(ArchiveUiMsg::MinimizeModal)),
        space::horizontal(),
        button(text(strings.cancel).size(12))
            .padding([7, 14])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::Archive(ArchiveUiMsg::CloseModal)),
        start_btn,
    ]
    .spacing(8)
    .align_y(Center);

    column![
        info_banner,
        name_field,
        format_field,
        level_field,
        conflict_field,
        actions,
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn running_progress_body<'a>(
    app: &'a App,
    job_id: u64,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let Some(runtime) = app.archive_jobs.get(&job_id) else {
        return column![
            text(strings.loading).size(12).color(palette.text_secondary),
        ]
        .into();
    };
    let job = &runtime.progress;
    let ratio = if job.bytes_total == 0 {
        0.0
    } else {
        (job.bytes_done as f32 / job.bytes_total as f32).clamp(0.0, 1.0)
    };
    let pct = (ratio * 100.0).round() as u32;
    let filled = (ratio * 1000.0).round() as u16;
    let remaining = 1000u16.saturating_sub(filled);

    let bar = container(
        row![
            container(Space::new().width(Length::Fill).height(8))
                .width(Length::FillPortion(filled.max(1)))
                .style(move |_| styles::panel_surface(
                    palette.accent_primary,
                    Color::TRANSPARENT,
                    9999.0
                )),
            Space::new()
                .width(Length::FillPortion(remaining.max(1)))
                .height(8),
        ]
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(1)
    .style(move |_| styles::panel_surface(palette.surface_high, palette.border, 9999.0));

    let bytes_summary = if job.bytes_total > 0 {
        format!(
            "{pct}%  ({} / {})",
            format_bytes(job.bytes_done),
            format_bytes(job.bytes_total)
        )
    } else {
        format!("{} ({} / {})", strings.loading, job.items_done, job.items_total)
    };

    let current_display = sanitize_single_line(&job.current_item);

    column![
        container(
            column![
                row![
                    text(bytes_summary)
                        .size(13)
                        .font(FONT_MONO)
                        .color(palette.accent_primary),
                    space::horizontal(),
                    text(format!("{} / {}", job.items_done, job.items_total))
                        .size(11)
                        .font(FONT_MONO)
                        .color(palette.text_secondary),
                ]
                .align_y(Center),
                bar,
                text(current_display)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_muted)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(8),
        )
        .padding([10, 12])
        .width(Length::Fill)
        .clip(true)
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 8.0)),
        row![
            button(
                row![icon_minus().size(12), text(strings.arch_minimize).size(12)]
                    .spacing(6)
                    .align_y(Center),
            )
            .padding([7, 14])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::Archive(ArchiveUiMsg::MinimizeModal)),
            space::horizontal(),
            button(text(strings.cancel).size(12))
                .padding([7, 14])
                .style(styles::pill_secondary_button(palette))
                .on_press(Message::Archive(ArchiveUiMsg::CancelJob(job_id))),
        ]
        .align_y(Center),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}
