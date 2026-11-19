pub mod canvas;
pub mod footer;
pub mod header;
pub mod inspector;
pub mod modals;
pub mod sidebar;
pub mod virtualized;

use fm_core::{
    DriveInfo, FileEntry, IconThemeMode, SystemIconBucket, SystemIconKey, SystemPlaceKind,
};
use fm_theme::{FONT_UI, ThemePalette, styles};
use iced::widget::{Text, container, image, opaque, text, tooltip};
use iced::{Alignment::Center, Color, ContentFit, Element, Length};
use lucide_icons::iced::{
    icon_file_archive, icon_file_code, icon_file_json, icon_file_text, icon_folder, icon_image,
    icon_terminal,
};

use crate::app::{App, Message};

/// Renderiza um ícone do sistema hospedeiro (quando ativo e já em cache na GPU) ou usa o fallback vetorial.
pub fn render_system_or_fallback<'a>(
    app: &'a App,
    key: &SystemIconKey,
    fallback: Text<'a, iced::Theme>,
    size: f32,
) -> Element<'a, Message> {
    if app.config.icon_theme_mode == IconThemeMode::HostSystem {
        let primary_bucket = SystemIconBucket::for_display_size(size);
        let alt_bucket = match primary_bucket {
            SystemIconBucket::Small24 => SystemIconBucket::Large64,
            SystemIconBucket::Large64 => SystemIconBucket::Small24,
        };
        if let Some(handle) = app
            .gpu_system_icons
            .get(&(key.clone(), primary_bucket))
            .or_else(|| app.gpu_system_icons.get(&(key.clone(), alt_bucket)))
        {
            return image(handle.clone())
                .width(size)
                .height(size)
                .content_fit(ContentFit::Contain)
                .into();
        }
    }
    fallback.into()
}

/// Renderiza o ícone de uma entrada de arquivo/pasta respeitando o modo de ícones configurado.
pub fn render_entry_icon<'a>(
    app: &'a App,
    entry: &FileEntry,
    palette: ThemePalette,
    size: f32,
) -> Element<'a, Message> {
    let (fallback, _) = entry_icon(entry, palette, size);
    if app.config.icon_theme_mode == IconThemeMode::Bundled {
        return fallback.into();
    }
    let key = SystemIconKey::for_entry(entry, fm_fs::is_archive_file(&entry.path));
    render_system_or_fallback(app, &key, fallback, size)
}

/// Renderiza o ícone de um local especial da barra lateral respeitando o modo de ícones configurado.
pub fn render_place_icon<'a>(
    app: &'a App,
    place: SystemPlaceKind,
    fallback: Text<'a, iced::Theme>,
    size: f32,
) -> Element<'a, Message> {
    if app.config.icon_theme_mode == IconThemeMode::Bundled {
        return fallback.into();
    }
    render_system_or_fallback(app, &SystemIconKey::SpecialPlace(place), fallback, size)
}

/// Renderiza o ícone de uma unidade/disco respeitando o modo de ícones configurado.
pub fn render_drive_icon<'a>(
    app: &'a App,
    drive: &DriveInfo,
    fallback: Text<'a, iced::Theme>,
    size: f32,
) -> Element<'a, Message> {
    if app.config.icon_theme_mode == IconThemeMode::Bundled {
        return fallback.into();
    }
    render_system_or_fallback(app, &SystemIconKey::Drive(drive.kind), fallback, size)
}

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

/// Sanitiza quebras de linha e caracteres de controle em nomes/caminhos para exibição em linha única.
pub fn sanitize_single_line(raw: &str) -> String {
    if !raw.chars().any(char::is_control) {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.chars() {
        if ch.is_control() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = ch == ' ';
        }
    }
    out.trim().to_owned()
}

/// Envolve um controle de interface com tooltip estilizado pelo tema ativo.
pub fn with_tooltip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    tip: &'a str,
    position: tooltip::Position,
    palette: ThemePalette,
) -> Element<'a, M> {
    tooltip(
        content,
        container(text(tip).size(11).font(FONT_UI).color(palette.text_primary)).padding([5, 9]),
        position,
    )
    .delay(std::time::Duration::from_millis(450))
    .gap(6.0)
    .padding(0.0)
    .style(styles::tooltip_container(palette))
    .into()
}

/// Centraliza um card modal sobre backdrop escurecido e captura eventos de mouse da camada inferior.
pub fn modal_overlay_backdrop<'a, M: 'a>(card: impl Into<Element<'a, M>>) -> Element<'a, M> {
    opaque(
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Center)
            .align_y(Center)
            .style(move |_| styles::modal_backdrop()(&iced::Theme::Light)),
    )
}

#[cfg(test)]
mod tests {
    use super::sanitize_single_line;

    #[test]
    fn sanitizes_embedded_newlines_into_single_spaces() {
        assert_eq!(
            sanitize_single_line("Crash Team Racing\nTurbocharged"),
            "Crash Team Racing Turbocharged"
        );
        assert_eq!(
            sanitize_single_line("/home/user/Crash Team Racing \r\nTurbocharged"),
            "/home/user/Crash Team Racing Turbocharged"
        );
        assert_eq!(sanitize_single_line("normal_dir"), "normal_dir");
    }
}
