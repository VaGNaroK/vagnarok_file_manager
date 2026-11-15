use fm_core::{DriveKind, Strings, format_bytes};
use fm_theme::{FONT_MONO, FONT_UI, FONT_UI_SEMIBOLD, Spacing, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{Space, button, column, container, row, scrollable, space, text};
use iced::{Element, Length, Padding};
use lucide_icons::iced::{
    icon_database, icon_download, icon_folder, icon_hard_drive, icon_home, icon_image,
    icon_monitor, icon_panel_left, icon_settings, icon_star, icon_terminal, icon_trash_2,
};

use crate::app::{App, Message, SidebarTarget};

pub fn view<'a>(
    app: &'a App,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let collapsed = app.config.sidebar_collapsed;
    let width = if collapsed {
        Spacing::SIDEBAR_COLLAPSED_WIDTH
    } else {
        Spacing::SIDEBAR_WIDTH
    };

    let quick_candidates = [
        (SidebarTarget::Documents, strings.documents, icon_folder().size(16)),
        (SidebarTarget::Home, strings.home, icon_home().size(16)),
        (SidebarTarget::Desktop, strings.desktop, icon_monitor().size(16)),
        (SidebarTarget::Downloads, strings.downloads, icon_download().size(16)),
        (SidebarTarget::Pictures, strings.pictures, icon_image().size(16)),
        (SidebarTarget::CodeDev, strings.code_dev, icon_terminal().size(16)),
    ];

    let mut sections = column![].spacing(14).width(Length::Fill);

    if !collapsed {
        sections = sections.push(section_header(strings.quick_access, palette));
    }

    let mut quick_col = column![].spacing(2);
    for (target, label, icon) in quick_candidates {
        if let Some(resolved) = target.resolve_path(&app.places) {
            let active = app
                .current_tab()
                .is_some_and(|t| t.current_path == resolved);
            quick_col = quick_col.push(sidebar_btn(
                icon,
                label,
                collapsed,
                active,
                palette,
                Message::SelectSidebar(target),
            ));
        }
    }
    sections = sections.push(quick_col);

    if !app.drives.is_empty() {
        if !collapsed {
            sections = sections.push(section_header(strings.drives, palette));
        }

        let mut drives_col = column![].spacing(4);
        for drive in &app.drives {
            let icon = match drive.kind {
                DriveKind::InternalSsd => icon_hard_drive().size(16),
                _ => icon_database().size(16),
            };
            let label = if collapsed {
                String::new()
            } else if let Some(space_info) = &drive.space {
                format!("{} ({})", drive.label, format_bytes(space_info.free_bytes))
            } else {
                drive.label.to_string()
            };
            let active = app
                .current_tab()
                .is_some_and(|t| t.current_path == drive.mount_path);
            let btn = sidebar_btn_owned(
                icon,
                label,
                collapsed,
                active,
                palette,
                Message::SelectDrive(drive.mount_path.clone()),
            );

            if !collapsed
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
                            .style(move |_| styles::panel_surface(
                                fill_color,
                                fill_color,
                                9999.0
                            )),
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

                drives_col = drives_col.push(column![btn, bar].spacing(2));
            } else {
                drives_col = drives_col.push(btn);
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
            let label = fav_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| fav_path.to_string_lossy().into_owned());
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
            ));
        }
        sections = sections.push(fav_col);
    }

    if let Some(trash_path) = &app.places.trash {
        let active = app
            .current_tab()
            .is_some_and(|t| &t.current_path == trash_path);
        sections = sections.push(sidebar_btn(
            icon_trash_2().size(16),
            strings.trash,
            collapsed,
            active,
            palette,
            Message::SelectSidebar(SidebarTarget::Trash),
        ));
    }

    let footer_actions = row![
        sidebar_btn(
            icon_settings().size(16),
            strings.preferences,
            collapsed,
            false,
            palette,
            Message::OpenPreferences,
        ),
        button(icon_panel_left().size(15))
            .padding([6, 8])
            .style(styles::ghost_button(palette, 8.0))
            .on_press(Message::ToggleSidebar),
    ]
    .spacing(4)
    .align_y(Center);

    container(
        column![
            scrollable(sections).height(Length::Fill),
            footer_actions,
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(width))
    .height(Length::Fill)
    .padding(Padding::from([12, 8]))
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

fn sidebar_btn<'a>(
    icon: iced::widget::Text<'a, iced::Theme>,
    label: &'a str,
    collapsed: bool,
    active: bool,
    palette: ThemePalette,
    msg: Message,
) -> Element<'a, Message> {
    sidebar_btn_owned(icon, label.to_owned(), collapsed, active, palette, msg)
}

fn sidebar_btn_owned<'a>(
    icon: iced::widget::Text<'a, iced::Theme>,
    label: String,
    collapsed: bool,
    active: bool,
    palette: ThemePalette,
    msg: Message,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = if collapsed {
        row![space::horizontal(), icon, space::horizontal()]
            .align_y(Center)
            .into()
    } else {
        row![
            icon,
            text(label)
                .size(13)
                .font(if active { FONT_UI_SEMIBOLD } else { FONT_UI }),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    };

    button(content)
        .width(Length::Fill)
        .padding([6, 10])
        .style(styles::sidebar_item_button(palette, active))
        .on_press(msg)
        .into()
}
