use fm_core::{DriveKind, Strings, format_bytes};
use fm_theme::{FONT_MONO, FONT_UI, FONT_UI_SEMIBOLD, Spacing, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, space, text, tooltip,
};
use iced::{Element, Length, Padding};
use lucide_icons::iced::{
    icon_arrow_up_from_line, icon_chevron_down, icon_chevron_right, icon_database, icon_download,
    icon_file_text, icon_folder, icon_hard_drive, icon_home, icon_image, icon_monitor, icon_music,
    icon_panel_left, icon_plug, icon_refresh_cw, icon_settings, icon_star, icon_trash_2,
    icon_video,
};

use crate::app::{App, Message, SidebarTarget};
use crate::views::{sanitize_single_line, with_tooltip};

pub const SIDEBAR_WIDTH_MIN: f32 = 180.0;
pub const SIDEBAR_WIDTH_MAX: f32 = 560.0;

pub fn clamp_sidebar_width(width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX)
    } else {
        Spacing::SIDEBAR_WIDTH
    }
}

fn format_drive_label(drive: &fm_core::DriveInfo, strings: Strings) -> String {
    if drive.is_mounted {
        if let Some(space_info) = &drive.space {
            format!("{} ({})", drive.label, format_bytes(space_info.free_bytes))
        } else {
            drive.label.to_string()
        }
    } else if let Some(total) = drive.total_bytes_hint {
        format!(
            "{} ({} • {})",
            drive.label,
            format_bytes(total),
            strings.unmounted_label
        )
    } else {
        format!("{} ({})", drive.label, strings.unmounted_label)
    }
}

/// Calcula a largura efetiva da barra lateral: respeita o tamanho salvo pelo usuário ou ajusta ao maior disco/item.
pub fn effective_sidebar_width(app: &App, strings: Strings) -> f32 {
    if app.config.sidebar_collapsed {
        return Spacing::SIDEBAR_COLLAPSED_WIDTH;
    }
    if let Some(custom) = app.config.sidebar_width {
        return clamp_sidebar_width(custom);
    }

    let mut max_drive_px = 0.0_f32;
    for drive in &app.drives {
        let lbl = format_drive_label(drive, strings);
        let chars = lbl.chars().count() as f32;
        let has_action_btn = !drive.is_mounted || drive.can_unmount;
        let chrome = if has_action_btn { 108.0 } else { 76.0 };
        max_drive_px = max_drive_px.max(chars * 7.6 + chrome);
    }

    let quick_labels = [
        strings.home,
        strings.desktop,
        strings.documents,
        strings.downloads,
        strings.music,
        strings.pictures,
        strings.videos,
        strings.public_dir,
        strings.templates,
        strings.trash,
        strings.preferences,
    ];
    let mut max_place_px = 0.0_f32;
    for lbl in quick_labels {
        max_place_px = max_place_px.max(lbl.chars().count() as f32 * 7.6 + 88.0);
    }

    max_drive_px
        .max(max_place_px)
        .clamp(Spacing::SIDEBAR_WIDTH, 390.0)
        .round()
}

pub fn view<'a>(app: &'a App, palette: ThemePalette, strings: Strings) -> Element<'a, Message> {
    let collapsed = app.config.sidebar_collapsed;
    let width = effective_sidebar_width(app, strings);

    let quick_candidates = [
        (
            SidebarTarget::Desktop,
            strings.desktop,
            icon_monitor().size(16),
        ),
        (
            SidebarTarget::Documents,
            strings.documents,
            icon_folder().size(16),
        ),
        (
            SidebarTarget::Downloads,
            strings.downloads,
            icon_download().size(16),
        ),
        (SidebarTarget::Music, strings.music, icon_music().size(16)),
        (
            SidebarTarget::Pictures,
            strings.pictures,
            icon_image().size(16),
        ),
        (SidebarTarget::Videos, strings.videos, icon_video().size(16)),
        (
            SidebarTarget::PublicDir,
            strings.public_dir,
            icon_folder().size(16),
        ),
        (
            SidebarTarget::Templates,
            strings.templates,
            icon_file_text().size(16),
        ),
    ];

    let mut sections = column![].spacing(14).width(Length::Fill);

    if !collapsed {
        sections = sections.push(section_header(strings.quick_access, palette));
    }

    let mut quick_col = column![].spacing(2);
    let home_path = app.places.home.clone();
    let home_active = app
        .current_tab()
        .is_some_and(|t| t.current_path == home_path);
    let home_btn = sidebar_btn_owned(
        icon_home().size(16),
        strings.home.to_owned(),
        collapsed,
        home_active,
        palette,
        Message::SelectSidebar(SidebarTarget::Home),
        Some(Message::OpenSidebarContextMenu {
            path: home_path,
            is_favorite: false,
        }),
    );

    if collapsed {
        quick_col = quick_col.push(home_btn);
    } else {
        let expand_icon = if app.sidebar_home_expanded {
            icon_chevron_down().size(13).color(palette.text_muted)
        } else {
            icon_chevron_right().size(13).color(palette.text_muted)
        };
        let expand_btn = with_tooltip(
            button(expand_icon)
                .padding([5, 5])
                .style(styles::ghost_button(palette, 6.0))
                .on_press(Message::ToggleSidebarHomeExpanded),
            strings.tip_expand_home,
            tooltip::Position::Right,
            palette,
        );
        quick_col = quick_col.push(
            row![
                expand_btn,
                container(home_btn).width(Length::Fill).clip(true)
            ]
            .spacing(2)
            .align_y(Center),
        );

        if app.sidebar_home_expanded {
            for sub_path in &app.home_subdirs {
                let raw_name = sub_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| sub_path.to_string_lossy().into_owned());
                let label = sanitize_single_line(&raw_name);
                let sub_active = app
                    .current_tab()
                    .is_some_and(|t| &t.current_path == sub_path);
                let sub_btn = sidebar_btn_owned(
                    icon_folder().size(14),
                    label,
                    false,
                    sub_active,
                    palette,
                    Message::NavigateTo(sub_path.clone()),
                    Some(Message::OpenSidebarContextMenu {
                        path: sub_path.clone(),
                        is_favorite: false,
                    }),
                );
                quick_col = quick_col.push(
                    container(sub_btn)
                        .padding(Padding {
                            top: 0.0,
                            right: 0.0,
                            bottom: 0.0,
                            left: 18.0,
                        })
                        .width(Length::Fill),
                );
            }
        }
    }

    for (target, label, icon) in quick_candidates {
        if let Some(resolved) = target.resolve_path(&app.places) {
            let active = app
                .current_tab()
                .is_some_and(|t| t.current_path == resolved);
            quick_col = quick_col.push(sidebar_btn_owned(
                icon,
                label.to_owned(),
                collapsed,
                active,
                palette,
                Message::SelectSidebar(target),
                Some(Message::OpenSidebarContextMenu {
                    path: resolved,
                    is_favorite: false,
                }),
            ));
        }
    }
    sections = sections.push(quick_col);

    if !app.drives.is_empty() {
        if !collapsed {
            let refresh_btn = with_tooltip(
                button(icon_refresh_cw().size(12).color(palette.text_muted))
                    .padding([2, 5])
                    .style(styles::ghost_button(palette, 6.0))
                    .on_press(Message::RefreshDrives),
                strings.refresh_drives,
                tooltip::Position::Right,
                palette,
            );
            sections = sections.push(
                row![
                    section_header(strings.drives, palette),
                    space::horizontal(),
                    refresh_btn
                ]
                .align_y(Center)
                .width(Length::Fill),
            );
        }

        let mut drives_col = column![].spacing(4);
        for drive in &app.drives {
            let base_icon = match drive.kind {
                DriveKind::InternalSsd => icon_hard_drive().size(16),
                _ => icon_database().size(16),
            };
            let icon = if drive.is_mounted {
                base_icon
            } else {
                base_icon.color(palette.text_muted)
            };
            let label = if collapsed {
                String::new()
            } else {
                format_drive_label(drive, strings)
            };
            let active = drive.is_mounted
                && app
                    .current_tab()
                    .is_some_and(|t| t.current_path == drive.mount_path);
            let primary_msg = if drive.is_mounted {
                Message::SelectDrive(drive.mount_path.clone())
            } else if let Some(dev) = &drive.device_path {
                Message::MountAndOpenDrive(dev.clone())
            } else {
                Message::RefreshDrives
            };
            let ctx_msg = Message::OpenDriveContextMenu {
                label: drive.label.to_string(),
                mount_path: drive.mount_path.clone(),
                device_path: drive.device_path.clone(),
                is_mounted: drive.is_mounted,
                can_unmount: drive.can_unmount,
            };
            let btn = sidebar_btn_owned(
                icon,
                label,
                collapsed,
                active,
                palette,
                primary_msg,
                Some(ctx_msg),
            );

            let row_widget: Element<'a, Message> =
                if !collapsed && drive.is_mounted && drive.can_unmount {
                    let unmount_btn = with_tooltip(
                        button(
                            icon_arrow_up_from_line()
                                .size(13)
                                .color(palette.text_secondary),
                        )
                        .padding([5, 6])
                        .style(styles::ghost_button(palette, 6.0))
                        .on_press(Message::UnmountDrive {
                            device_path: drive.device_path.clone(),
                            mount_path: Some(drive.mount_path.clone()),
                            label: drive.label.to_string(),
                        }),
                        strings.unmount_drive,
                        tooltip::Position::Right,
                        palette,
                    );
                    row![container(btn).width(Length::Fill).clip(true), unmount_btn]
                        .spacing(2)
                        .align_y(Center)
                        .into()
                } else if !collapsed && !drive.is_mounted {
                    if let Some(dev) = &drive.device_path {
                        let mount_btn = with_tooltip(
                            button(icon_plug().size(13).color(palette.accent_primary))
                                .padding([5, 6])
                                .style(styles::ghost_button(palette, 6.0))
                                .on_press(Message::MountAndOpenDrive(dev.clone())),
                            strings.mount_drive,
                            tooltip::Position::Right,
                            palette,
                        );
                        row![container(btn).width(Length::Fill).clip(true), mount_btn]
                            .spacing(2)
                            .align_y(Center)
                            .into()
                    } else {
                        btn
                    }
                } else {
                    btn
                };

            if !collapsed
                && drive.is_mounted
                && let Some(space_info) = &drive.space
                && space_info.total_bytes > 0
            {
                let used = space_info.total_bytes.saturating_sub(space_info.free_bytes);
                let ratio = ((used as f64) / (space_info.total_bytes as f64)).clamp(0.0, 1.0);
                let filled = ((ratio * 1000.0).round() as u16).clamp(1, 1000);
                let remaining = 1000u16.saturating_sub(filled);
                let fill_color = if ratio > 0.9 {
                    palette.status_error
                } else if ratio > 0.75 {
                    palette.status_warning
                } else {
                    palette.accent_primary
                };
                let bar = container(
                    row![
                        container(Space::new().width(Length::Fill).height(3))
                            .width(Length::FillPortion(filled))
                            .style(move |_| styles::panel_surface(fill_color, fill_color, 9999.0)),
                        container(Space::new().width(Length::Fill).height(3))
                            .width(Length::FillPortion(remaining.max(1)))
                            .style(move |_| styles::panel_surface(
                                palette.surface_high,
                                palette.surface_high,
                                9999.0
                            )),
                    ]
                    .width(Length::Fill),
                )
                .padding(Padding::from([0, 10]));

                drives_col = drives_col.push(column![row_widget, bar].spacing(2));
            } else {
                drives_col = drives_col.push(row_widget);
            }
        }
        sections = sections.push(drives_col);
    }

    let has_tags_or_trash = !app.config.favorites.is_empty() || app.places.trash.is_some();
    if has_tags_or_trash && !collapsed {
        sections = sections.push(section_header(strings.tags_and_system, palette));
    }

    if !app.config.favorites.is_empty() {
        let mut fav_col = column![].spacing(2);
        for fav_path in &app.config.favorites {
            let raw_label = fav_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| fav_path.to_string_lossy().into_owned());
            let label = sanitize_single_line(&raw_label);
            let active = app
                .current_tab()
                .is_some_and(|t| &t.current_path == fav_path);
            fav_col = fav_col.push(sidebar_btn_owned(
                icon_star().size(15).color(palette.accent_tertiary),
                label,
                collapsed,
                active,
                palette,
                Message::NavigateTo(fav_path.clone()),
                Some(Message::OpenSidebarContextMenu {
                    path: fav_path.clone(),
                    is_favorite: true,
                }),
            ));
        }
        sections = sections.push(fav_col);
    }

    if let Some(trash_path) = &app.places.trash {
        let active = app
            .current_tab()
            .is_some_and(|t| &t.current_path == trash_path);
        sections = sections.push(sidebar_btn_owned(
            icon_trash_2().size(16),
            strings.trash.to_owned(),
            collapsed,
            active,
            palette,
            Message::SelectSidebar(SidebarTarget::Trash),
            Some(Message::OpenSidebarContextMenu {
                path: trash_path.clone(),
                is_favorite: false,
            }),
        ));
    }

    let footer_actions = row![
        with_tooltip(
            sidebar_btn_owned(
                icon_settings().size(16),
                strings.preferences.to_owned(),
                collapsed,
                false,
                palette,
                Message::OpenPreferences,
                Some(Message::OpenPreferencesContextMenu),
            ),
            strings.tip_preferences,
            tooltip::Position::Top,
            palette,
        ),
        with_tooltip(
            button(icon_panel_left().size(15))
                .padding([6, 8])
                .style(styles::ghost_button(palette, 8.0))
                .on_press(Message::ToggleSidebar),
            strings.tip_toggle_sidebar,
            tooltip::Position::Top,
            palette,
        ),
    ]
    .spacing(4)
    .align_y(Center);

    container(
        column![
            scrollable(sections)
                .height(Length::Fill)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new()
                        .width(6)
                        .scroller_width(6)
                        .spacing(6),
                )),
            footer_actions,
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(width))
    .height(Length::Fill)
    .padding(Padding {
        top: 12.0,
        right: 4.0,
        bottom: 12.0,
        left: 8.0,
    })
    .clip(true)
    .style(styles::sidebar_container(palette))
    .into()
}

fn section_header<'a>(label: &'a str, palette: ThemePalette) -> Element<'a, Message> {
    container(
        text(label)
            .size(11)
            .font(FONT_MONO)
            .color(palette.text_muted),
    )
    .padding(Padding::from([2, 10]))
    .into()
}

fn sidebar_btn_owned<'a>(
    icon: iced::widget::Text<'a, iced::Theme>,
    label: String,
    collapsed: bool,
    active: bool,
    palette: ThemePalette,
    msg: Message,
    right_click: Option<Message>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = if collapsed {
        row![space::horizontal(), icon, space::horizontal()]
            .align_y(Center)
            .into()
    } else {
        row![
            icon,
            container(
                text(label)
                    .size(13)
                    .font(if active { FONT_UI_SEMIBOLD } else { FONT_UI })
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    };

    let btn = button(content)
        .width(Length::Fill)
        .padding([6, 10])
        .style(styles::sidebar_item_button(palette, active))
        .on_press(msg);
    if let Some(ctx_msg) = right_click {
        mouse_area(btn).on_right_press(ctx_msg).into()
    } else {
        btn.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;
    use fm_core::{DriveInfo, DriveKind};
    use std::path::PathBuf;

    #[test]
    fn clamps_sidebar_width_within_allowed_bounds() {
        assert_eq!(clamp_sidebar_width(120.0), SIDEBAR_WIDTH_MIN);
        assert_eq!(clamp_sidebar_width(640.0), SIDEBAR_WIDTH_MAX);
        assert_eq!(clamp_sidebar_width(305.0), 305.0);
        assert_eq!(clamp_sidebar_width(f32::NAN), Spacing::SIDEBAR_WIDTH);
    }

    #[test]
    fn formats_unmounted_drive_label_with_capacity_and_status() {
        let strings = Strings::for_language(fm_core::Language::PtBr);
        let drive = DriveInfo {
            label: CompactString::new("JOGOS"),
            mount_path: PathBuf::new(),
            device_path: Some(PathBuf::from("/dev/sda1")),
            fs_type: Some(CompactString::new("ntfs")),
            kind: DriveKind::SecondaryDisk,
            is_mounted: false,
            can_unmount: false,
            total_bytes_hint: Some(480_102_055_936),
            space: None,
        };
        let label = format_drive_label(&drive, strings);
        assert!(label.contains("JOGOS"));
        assert!(label.contains(strings.unmounted_label));
    }
}
