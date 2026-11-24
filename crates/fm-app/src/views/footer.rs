use fm_core::{ArchiveJobKind, CopyMode, Strings, TabState, format_bytes};
use fm_theme::{FONT_MONO, FONT_UI, Spacing, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{Space, button, container, row, space, text};
use iced::{Color, Element, Length, Padding};
use lucide_icons::iced::{icon_file_archive, icon_maximize_2, icon_search, icon_x};

use crate::app::{App, Message};
use crate::archive_jobs::ArchiveUiMsg;
use crate::duplicate_finder::DuplicateFinderMsg;
use crate::views::duplicate_finder::stage_badge_text;
use crate::views::modals::format_transfer_summary;
use crate::views::sanitize_single_line;

pub fn view<'a>(
    app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (items_label, selection_label, sha_label, disk_label) = match tab {
        Some(tab) => {
            let total = tab.visible_entries.len();
            let items_lbl = strings.format_items_count(total);

            let sel_count = tab.selection.count();
            let sel_lbl = if sel_count == 0 {
                strings.selected_none.to_owned()
            } else if sel_count == 1 {
                if let Some(entry) = tab.selection.single_selected(&tab.visible_entries) {
                    let clean_name = sanitize_single_line(entry.name.as_str());
                    if entry.kind.is_dir_like() {
                        format!("1 ({clean_name})")
                    } else {
                        format!("1 ({clean_name} • {})", format_bytes(entry.size_bytes))
                    }
                } else {
                    strings.selected_none.to_owned()
                }
            } else {
                let bytes = tab.selection.selected_bytes(&tab.visible_entries);
                strings.format_selected_multi(sel_count, format_bytes(bytes).as_str())
            };

            let sha = tab
                .selected_telemetry
                .as_ref()
                .and_then(|s| s.sha256_prefix.as_ref())
                .map(|p| format!("sha256: {p}"))
                .unwrap_or_default();

            let disk = tab
                .telemetry
                .volume_space
                .as_ref()
                .map(|v| {
                    format!(
                        "{}: {} {} {}",
                        v.label,
                        format_bytes(v.free_bytes),
                        strings.free_of,
                        format_bytes(v.total_bytes)
                    )
                })
                .unwrap_or_default();

            (items_lbl, sel_lbl, sha, disk)
        }
        None => (
            strings.format_items_count(0),
            strings.selected_none.to_owned(),
            String::new(),
            String::new(),
        ),
    };

    let dot = container(Space::new().width(6).height(6)).style(move |_| {
        styles::badge_container(
            palette.accent_primary,
            palette.accent_primary,
            Color::TRANSPARENT,
            9999.0,
        )(&iced::Theme::Light)
    });

    let left = row![
        dot,
        text(items_label)
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_secondary),
        text("•").size(11).color(palette.text_muted),
        text(selection_label)
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_primary),
        text("•").size(11).color(palette.text_muted),
        text(strings.quicklook_hint)
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_muted),
    ]
    .spacing(8)
    .align_y(Center);

    let mut right = row![].spacing(12).align_y(Center);
    for (&job_id, runtime) in &app.archive_jobs {
        let job = &runtime.progress;
        let pct = if job.bytes_total > 0 {
            ((job.bytes_done as f32 / job.bytes_total as f32).clamp(0.0, 1.0) * 100.0).round()
                as u32
        } else {
            0
        };
        let label = match job.kind {
            ArchiveJobKind::Compress { .. } => {
                format!("{}: {} ({pct}%)", strings.arch_compressing, job.title)
            }
            ArchiveJobKind::Extract { .. } => {
                format!("{}: {} ({pct}%)", strings.arch_extracting, job.title)
            }
        };
        let pill = container(
            row![
                icon_file_archive().size(12).color(palette.accent_primary),
                text(label)
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.accent_primary),
                button(
                    row![
                        icon_maximize_2().size(11).color(palette.text_primary),
                        text(strings.arch_restore)
                            .size(11)
                            .font(FONT_UI)
                            .color(palette.text_primary),
                    ]
                    .spacing(4)
                    .align_y(Center),
                )
                .padding([2, 6])
                .style(styles::ghost_button(palette, 4.0))
                .on_press(Message::Archive(ArchiveUiMsg::RestoreJob(job_id))),
                button(icon_x().size(11).color(palette.status_error))
                    .padding([2, 4])
                    .style(styles::ghost_button(palette, 4.0))
                    .on_press(Message::Archive(ArchiveUiMsg::CancelJob(job_id))),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([2, 8])
        .style(move |_| styles::panel_surface(palette.accent_surface, palette.accent_border, 6.0));
        right = right.push(pill);
    }
    if let Some(modal) = &app.disk_analyzer
        && modal.minimized
    {
        right = right.push(crate::views::disk_analyzer::disk_analyzer_minimized_pill(
            modal, palette, strings,
        ));
    }
    if let Some(modal) = &app.compress_modal
        && modal.minimized
        && modal.running_job_id.is_none()
    {
        let draft_pill = container(
            row![
                icon_file_archive().size(12).color(palette.text_secondary),
                text(modal.full_output_filename())
                    .size(11)
                    .font(FONT_MONO)
                    .color(palette.text_secondary),
                button(
                    text(strings.arch_restore)
                        .size(11)
                        .font(FONT_UI)
                        .color(palette.accent_primary)
                )
                .padding([2, 6])
                .style(styles::ghost_button(palette, 4.0))
                .on_press(Message::Archive(ArchiveUiMsg::RestoreModal)),
                button(icon_x().size(11).color(palette.text_muted))
                    .padding([2, 4])
                    .style(styles::ghost_button(palette, 4.0))
                    .on_press(Message::Archive(ArchiveUiMsg::CloseModal)),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([2, 8])
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 6.0));
        right = right.push(draft_pill);
    }
    if let Some(dup) = &app.duplicate_finder
        && dup.minimized
    {
        let pill_label = if dup.scanning {
            format!(
                "{}: {}",
                strings.dup_footer_prefix,
                stage_badge_text(dup, strings)
            )
        } else {
            format!(
                "{}: {} • {} {}",
                strings.dup_footer_prefix,
                stage_badge_text(dup, strings),
                dup.groups.len(),
                strings.dup_groups_found
            )
        };
        let accent_pill = dup.scanning;
        let dup_pill = container(
            row![
                icon_search().size(12).color(if accent_pill {
                    palette.accent_primary
                } else {
                    palette.text_secondary
                }),
                text(pill_label)
                    .size(11)
                    .font(FONT_MONO)
                    .color(if accent_pill {
                        palette.accent_primary
                    } else {
                        palette.text_secondary
                    }),
                button(
                    row![
                        icon_maximize_2().size(11).color(palette.text_primary),
                        text(strings.arch_restore)
                            .size(11)
                            .font(FONT_UI)
                            .color(palette.text_primary),
                    ]
                    .spacing(4)
                    .align_y(Center),
                )
                .padding([2, 6])
                .style(styles::ghost_button(palette, 4.0))
                .on_press(Message::DuplicateFinder(DuplicateFinderMsg::Restore)),
                button(icon_x().size(11).color(palette.status_error))
                    .padding([2, 4])
                    .style(styles::ghost_button(palette, 4.0))
                    .on_press(Message::DuplicateFinder(
                        DuplicateFinderMsg::CancelFromFooter,
                    )),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([2, 8])
        .style(move |_| {
            if accent_pill {
                styles::panel_surface(palette.accent_surface, palette.accent_border, 6.0)
            } else {
                styles::panel_surface(palette.surface_low, palette.border, 6.0)
            }
        });
        right = right.push(dup_pill);
    }
    if let Some(job) = &app.active_transfer {
        right = right.push(
            text(format!(
                "{} {}",
                strings.transferring,
                format_transfer_summary(job.bytes_done, job.bytes_total)
            ))
            .size(11)
            .font(FONT_MONO)
            .color(palette.accent_primary),
        );
    } else if let Some(clip) = &app.clipboard {
        let tag = match clip.mode {
            CopyMode::Copy => strings.clipboard_copy_tag,
            CopyMode::Move => strings.clipboard_cut_tag,
        };
        right = right.push(
            text(format!("{tag}: {}", clip.paths.len()))
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        );
    }
    if let Some(notice) = &app.status_notice {
        right = right.push(
            text(notice.as_str())
                .size(11)
                .font(FONT_MONO)
                .color(palette.status_error),
        );
    }
    if !sha_label.is_empty() {
        right = right.push(
            text(sha_label)
                .size(11)
                .font(FONT_MONO)
                .color(palette.accent_primary),
        );
    }
    if !disk_label.is_empty() {
        right = right.push(
            text(disk_label)
                .size(11)
                .font(FONT_MONO)
                .color(palette.text_secondary),
        );
    }

    container(row![left, space::horizontal(), right].align_y(Center))
        .width(Length::Fill)
        .height(Spacing::FOOTER_HEIGHT)
        .padding(Padding::from([0, 14]))
        .style(styles::footer_container(palette))
        .into()
}
