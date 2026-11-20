use fm_core::ThemePreset;
use iced::Color;

/// Constantes de espaçamento e dimensões estruturais (base 4px/8px da Spec 02).
pub struct Spacing;

impl Spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;

    pub const TITLEBAR_HEIGHT: f32 = 38.0;
    pub const NAVBAR_HEIGHT: f32 = 42.0;
    pub const SIDEBAR_WIDTH: f32 = 240.0;
    pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 64.0;
    pub const INSPECTOR_WIDTH: f32 = 280.0;
    pub const FOOTER_HEIGHT: f32 = 30.0;
}

/// Paleta semântica completa para renderização consistente em tema claro e escuro.
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub preset: ThemePreset,
    pub canvas: Color,
    pub header_bg: Color,
    pub sidebar_bg: Color,
    pub surface_lowest: Color,
    pub surface_low: Color,
    pub surface_high: Color,
    pub border: Color,
    pub border_strong: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent_primary: Color,
    pub accent_primary_hover: Color,
    pub accent_surface: Color,
    pub accent_border: Color,
    pub accent_tertiary: Color,
    pub status_success: Color,
    pub status_success_bg: Color,
    pub status_warning: Color,
    pub status_error: Color,
    pub on_accent: Color,
}

impl ThemePalette {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::AetherLight => Self::aether_light(),
            ThemePreset::ObsidianFlow => Self::obsidian_flow(),
        }
    }

    /// Preset claro fiel ao `code.html` e `screen.png`.
    pub const fn aether_light() -> Self {
        Self {
            preset: ThemePreset::AetherLight,
            canvas: hex(0xF8FAFC),
            header_bg: rgba(0xFF, 0xFF, 0xFF, 0.92),
            sidebar_bg: rgba(0xF4, 0xF6, 0xFB, 0.96),
            surface_lowest: hex(0xFFFFFF),
            surface_low: hex(0xF1F5F9),
            surface_high: hex(0xE2E8F0),
            border: hex(0xE2E8F0),
            border_strong: hex(0xCBD5E1),
            text_primary: hex(0x0F172A),
            text_secondary: hex(0x334155),
            text_muted: hex(0x475569),
            accent_primary: hex(0x2563EB),
            accent_primary_hover: hex(0x1D4ED8),
            accent_surface: rgba(0x25, 0x63, 0xEB, 0.10),
            accent_border: hex(0x93C5FD),
            accent_tertiary: hex(0x6366F1),
            status_success: hex(0x10B981),
            status_success_bg: hex(0xECFDF5),
            status_warning: hex(0xF59E0B),
            status_error: hex(0xEF4444),
            on_accent: hex(0xFFFFFF),
        }
    }

    /// Preset escuro reconciliado a partir de `DESIGN.md`.
    pub const fn obsidian_flow() -> Self {
        Self {
            preset: ThemePreset::ObsidianFlow,
            canvas: hex(0x0F1117),
            header_bg: rgba(0x19, 0x1B, 0x22, 0.90),
            sidebar_bg: rgba(0x11, 0x13, 0x19, 0.94),
            surface_lowest: hex(0x191B22),
            surface_low: hex(0x1E1F26),
            surface_high: hex(0x282A30),
            border: hex(0x2B303C),
            border_strong: hex(0x424754),
            text_primary: hex(0xE2E2EB),
            text_secondary: hex(0xC2C6D6),
            text_muted: hex(0x8C909F),
            accent_primary: hex(0x3B82F6),
            accent_primary_hover: hex(0x60A5FA),
            accent_surface: rgba(0x3B, 0x82, 0xF6, 0.16),
            accent_border: rgba(0x60, 0xA5, 0xFA, 0.45),
            accent_tertiary: hex(0x8B5CF6),
            status_success: hex(0x10B981),
            status_success_bg: rgba(0x10, 0xB9, 0x81, 0.14),
            status_warning: hex(0xF59E0B),
            status_error: hex(0xFFB4AB),
            on_accent: hex(0xFFFFFF),
        }
    }

    /// Converte para o `iced::Theme` customizado usado na raiz do aplicativo.
    pub fn to_iced_theme(self) -> iced::Theme {
        let name = match self.preset {
            ThemePreset::AetherLight => "Aether Light",
            ThemePreset::ObsidianFlow => "Obsidian Flow",
        };
        iced::Theme::custom(
            name.to_owned(),
            iced::theme::Palette {
                background: self.canvas,
                text: self.text_primary,
                primary: self.accent_primary,
                success: self.status_success,
                warning: self.status_warning,
                danger: self.status_error,
            },
        )
    }
}

const fn hex(rgb: u32) -> Color {
    let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
    let b = (rgb & 0xFF) as f32 / 255.0;
    Color::from_rgb(r, g, b)
}

const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relative_luminance(c: Color) -> f32 {
        fn linear(channel: f32) -> f32 {
            if channel <= 0.03928 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }

    fn contrast_ratio(fg: Color, bg: Color) -> f32 {
        let l1 = relative_luminance(fg);
        let l2 = relative_luminance(bg);
        let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn palettes_have_distinct_backgrounds_and_valid_channels() {
        let light = ThemePalette::aether_light();
        let dark = ThemePalette::obsidian_flow();
        assert_ne!(light.canvas, dark.canvas);
        assert!(light.canvas.r > 0.9);
        assert!(dark.canvas.r < 0.1);
    }

    #[test]
    fn text_tokens_meet_wcag_aa_contrast_on_surfaces() {
        let light = ThemePalette::aether_light();
        for surface in [light.canvas, light.surface_lowest, light.surface_low] {
            assert!(contrast_ratio(light.text_primary, surface) >= 10.0);
            assert!(contrast_ratio(light.text_secondary, surface) >= 7.0);
            assert!(contrast_ratio(light.text_muted, surface) >= 4.5);
        }

        let dark = ThemePalette::obsidian_flow();
        for surface in [dark.canvas, dark.surface_lowest, dark.surface_low] {
            assert!(contrast_ratio(dark.text_primary, surface) >= 10.0);
            assert!(contrast_ratio(dark.text_secondary, surface) >= 7.0);
            assert!(contrast_ratio(dark.text_muted, surface) >= 4.5);
        }
    }
}
