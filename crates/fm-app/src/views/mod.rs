pub mod canvas;
pub mod footer;
pub mod header;
pub mod inspector;
pub mod modals;
pub mod sidebar;
pub mod virtualized;

use fm_core::FileEntry;
use fm_theme::ThemePalette;
use iced::Color;
use iced::widget::Text;
use lucide_icons::iced::{
    icon_file_archive, icon_file_code, icon_file_json, icon_file_text, icon_folder, icon_image,
    icon_terminal,
};

/// Mapeia uma entrada de arquivo/pasta para o ícone Lucide e cor semântica correspondentes.
pub fn entry_icon<'a>(
    entry: &FileEntry,
    palette: ThemePalette,
    size: f32,
) -> (Text<'a, iced::Theme>, Color) {
    if entry.kind.is_dir_like() {
        let color = match entry.name.as_str() {
            "Design System" => palette.accent_tertiary,
            "Wallpapers" | "Pictures" | "Imagens" => Color::from_rgb8(0x02, 0x84, 0xC7),
            "Backups" => palette.text_secondary,
            "Scripts" => Color::from_rgb8(0x7C, 0x3A, 0xED),
            _ => palette.accent_primary,
        };
        return (icon_folder().size(size).color(color), color);
    }

    let ext_lower = entry
        .extension
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match ext_lower.as_str() {
        "rs" | "toml" | "py" | "js" | "ts" | "tsx" | "jsx" | "c" | "cpp" | "h" | "hpp" | "go"
        | "fig" | "html" | "css" => {
            let color = if ext_lower == "fig" {
                palette.accent_tertiary
            } else {
                palette.accent_primary
            };
            (icon_file_code().size(size).color(color), color)
        }
        "json" => {
            let color = palette.status_warning;
            (icon_file_json().size(size).color(color), color)
        }
        "sh" | "bash" | "zsh" | "fish" | "bat" | "ps1" => {
            let color = palette.status_success;
            (icon_terminal().size(size).color(color), color)
        }
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" | "ico" => {
            let color = Color::from_rgb8(0x02, 0x84, 0xC7);
            (icon_image().size(size).color(color), color)
        }
        "tar" | "gz" | "zip" | "xz" | "zst" | "7z" | "rar" => {
            let color = Color::from_rgb8(0xD9, 0x77, 0x06);
            (icon_file_archive().size(size).color(color), color)
        }
        "pdf" => {
            let color = palette.status_error;
            (icon_file_text().size(size).color(color), color)
        }
        _ => {
            let color = palette.text_secondary;
            (icon_file_text().size(size).color(color), color)
        }
    }
}
