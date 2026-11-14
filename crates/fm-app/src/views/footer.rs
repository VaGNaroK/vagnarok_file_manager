use fm_core::{Strings, TabState, format_bytes};
use fm_theme::{FONT_MONO, Spacing, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{Space, container, row, space, text};
use iced::{Color, Element, Length, Padding};

use crate::app::App;
use crate::app::Message;

pub fn view<'a>(
    _app: &'a App,
    tab: Option<&'a TabState>,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (items_label, selection_label, sha_label, disk_label) = match tab {
        Some(tab) => {
            let total = tab.visible_entries.len();
            let items_lbl = format!("{total} {}", strings.items_unit);

            let sel_count = tab.selection.count();
            let sel_lbl = if sel_count == 0 {
                strings.selected_none.to_owned()
            } else if sel_count == 1 {
                if let Some(entry) = tab.selection.single_selected(&tab.visible_entries) {
                    if entry.kind.is_dir_like() {
                        format!("1 ({})", entry.name)
                    } else {
                        format!("1 ({} • {})", entry.name, format_bytes(entry.size_bytes))
                    }
                } else {
                    strings.selected_none.to_owned()
                }
            } else {
                let bytes = tab.selection.selected_bytes(&tab.visible_entries);
                format!(
                    "{sel_count} {} ({})",
                    strings.items_unit,
                    format_bytes(bytes)
                )
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
            format!("0 {}", strings.items_unit),
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
