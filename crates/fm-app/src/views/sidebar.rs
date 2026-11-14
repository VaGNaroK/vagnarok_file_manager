use fm_core::{DriveKind, Strings, format_bytes};
use fm_theme::{FONT_MONO, FONT_UI, FONT_UI_SEMIBOLD, Spacing, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Element, Length, Padding};
use lucide_icons::iced::{
    icon_bookmark, icon_database, icon_download, icon_folder, icon_hard_drive, icon_home,
    icon_image, icon_monitor, icon_panel_left, icon_settings, icon_terminal, icon_trash_2,
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

    let quick_items = [
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
    for (target, label, icon) in quick_items {
        let active = app.active_sidebar == Some(target);
        quick_col = quick_col.push(sidebar_btn(
            icon,
            label,
            collapsed,
            active,
            palette,
            Message::SelectSidebar(target),
        ));
    }
    sections = sections.push(quick_col);

    if !collapsed {
        sections = sections.push(section_header(strings.drives, palette));
    }

    let mut drives_col = column![].spacing(2);
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
        drives_col = drives_col.push(sidebar_btn_owned(
            icon,
            label,
            collapsed,
            active,
            palette,
            Message::SelectDrive(drive.mount_path.clone()),
        ));
    }
    sections = sections.push(drives_col);

    if !collapsed {
        sections = sections.push(section_header(strings.tags_and_system, palette));
    }

    let tags_col = column![
        sidebar_btn(
            icon_bookmark().size(16).color(palette.status_warning),
            strings.starred,
            collapsed,
            app.active_sidebar == Some(SidebarTarget::Favorites),
            palette,
            Message::SelectSidebar(SidebarTarget::Favorites),
        ),
        sidebar_btn(
            icon_trash_2().size(16),
            strings.trash,
            collapsed,
            app.active_sidebar == Some(SidebarTarget::Trash),
            palette,
            Message::SelectSidebar(SidebarTarget::Trash),
        ),
    ]
    .spacing(2);
    sections = sections.push(tags_col);

    let footer_actions = row![
        sidebar_btn(
            icon_settings().size(16),
            strings.preferences,
            collapsed,
            false,
            palette,
            Message::ToggleHeroCard,
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
