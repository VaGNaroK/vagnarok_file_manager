use iced::border::Radius;
use iced::overlay::menu;
use iced::widget::{button, container, pick_list, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::palette::ThemePalette;

pub fn panel_surface(bg: Color, border_color: Color, radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: Radius::from(radius),
        },
        ..Default::default()
    }
}

pub fn canvas_container(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.canvas)),
        text_color: Some(p.text_primary),
        ..Default::default()
    }
}

pub fn header_container(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.header_bg)),
        border: Border {
            color: p.border,
            width: 0.0,
            radius: Radius::from(0.0),
        },
        ..Default::default()
    }
}

pub fn sidebar_container(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.sidebar_bg)),
        ..Default::default()
    }
}

pub fn inspector_container(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.surface_lowest)),
        ..Default::default()
    }
}

pub fn footer_container(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.surface_lowest)),
        ..Default::default()
    }
}

pub fn divider_line(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.border)),
        ..Default::default()
    }
}

pub fn hero_card(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.surface_lowest)),
        border: Border {
            color: p.border,
            width: 1.0,
            radius: Radius::from(12.0),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.04),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    }
}

pub fn badge_container(
    bg: Color,
    text_color: Color,
    border_color: Color,
    radius: f32,
) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(bg)),
        text_color: Some(text_color),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: Radius::from(radius),
        },
        ..Default::default()
    }
}

pub fn modal_card(p: ThemePalette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(p.surface_lowest)),
        text_color: Some(p.text_primary),
        border: Border {
            color: p.border_strong,
            width: 1.0,
            radius: Radius::from(14.0),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

pub fn modal_backdrop() -> impl Fn(&Theme) -> container::Style {
    |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.48))),
        ..Default::default()
    }
}

pub fn pill_primary_button(p: ThemePalette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => p.accent_primary_hover,
            _ => p.accent_primary,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: p.on_accent,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::from(9999.0),
            },
            ..Default::default()
        }
    }
}

pub fn pill_secondary_button(p: ThemePalette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => p.surface_low,
            _ => p.surface_lowest,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: p.text_secondary,
            border: Border {
                color: p.border,
                width: 1.0,
                radius: Radius::from(9999.0),
            },
            ..Default::default()
        }
    }
}

pub fn ghost_button(
    p: ThemePalette,
    radius: f32,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let (bg, text_color) = match status {
            button::Status::Hovered | button::Status::Pressed => {
                (Some(Background::Color(p.surface_low)), p.text_primary)
            }
            button::Status::Disabled => (None, p.text_muted),
            _ => (None, p.text_secondary),
        };
        button::Style {
            background: bg,
            text_color,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::from(radius),
            },
            ..Default::default()
        }
    }
}

pub fn tab_button(
    p: ThemePalette,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if active {
            button::Style {
                background: Some(Background::Color(p.surface_lowest)),
                text_color: p.accent_primary,
                border: Border {
                    color: p.border,
                    width: 1.0,
                    radius: Radius::from(8.0),
                },
                ..Default::default()
            }
        } else {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(Background::Color(p.surface_low))
                }
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: p.text_secondary,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: Radius::from(8.0),
                },
                ..Default::default()
            }
        }
    }
}

pub fn sidebar_item_button(
    p: ThemePalette,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if active {
            button::Style {
                background: Some(Background::Color(p.surface_lowest)),
                text_color: p.accent_primary,
                border: Border {
                    color: p.border,
                    width: 1.0,
                    radius: Radius::from(8.0),
                },
                ..Default::default()
            }
        } else {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(Background::Color(p.surface_lowest))
                }
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: p.text_secondary,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: Radius::from(8.0),
                },
                ..Default::default()
            }
        }
    }
}

pub fn entry_card_button(
    p: ThemePalette,
    selected: bool,
    radius: f32,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if selected {
            button::Style {
                background: Some(Background::Color(p.accent_surface)),
                text_color: p.text_primary,
                border: Border {
                    color: p.accent_border,
                    width: 1.0,
                    radius: Radius::from(radius),
                },
                ..Default::default()
            }
        } else {
            let (bg, border_color) = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    (Some(Background::Color(p.surface_lowest)), p.border)
                }
                _ => (None, Color::TRANSPARENT),
            };
            button::Style {
                background: bg,
                text_color: p.text_primary,
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: Radius::from(radius),
                },
                ..Default::default()
            }
        }
    }
}

pub fn search_input_style(
    p: ThemePalette,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let border_color = match status {
            text_input::Status::Focused { .. } => p.accent_primary,
            text_input::Status::Hovered => p.border_strong,
            _ => p.border,
        };
        let bg = match status {
            text_input::Status::Focused { .. } => p.surface_lowest,
            _ => p.surface_low,
        };
        text_input::Style {
            background: Background::Color(bg),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: Radius::from(9999.0),
            },
            icon: p.text_muted,
            placeholder: p.text_muted,
            value: p.text_primary,
            selection: p.accent_surface,
        }
    }
}

pub fn pick_list_style(p: ThemePalette) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_, status| {
        let (bg, border_color) = match status {
            pick_list::Status::Opened { .. } => (p.surface_lowest, p.accent_primary),
            pick_list::Status::Hovered => (p.surface_high, p.border_strong),
            pick_list::Status::Active => (p.surface_lowest, p.border),
        };
        pick_list::Style {
            text_color: p.text_primary,
            placeholder_color: p.text_muted,
            handle_color: p.text_secondary,
            background: Background::Color(bg),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: Radius::from(8.0),
            },
        }
    }
}

pub fn dropdown_menu_style(p: ThemePalette) -> impl Fn(&Theme) -> menu::Style {
    move |_| menu::Style {
        background: Background::Color(p.surface_lowest),
        border: Border {
            color: p.border_strong,
            width: 1.0,
            radius: Radius::from(10.0),
        },
        text_color: p.text_primary,
        selected_text_color: p.on_accent,
        selected_background: Background::Color(p.accent_primary),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 16.0,
        },
    }
}
