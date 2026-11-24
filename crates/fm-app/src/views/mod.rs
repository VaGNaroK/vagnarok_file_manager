pub mod archive_modal;
pub mod bulk_rename;
pub mod canvas;
pub mod duplicate_finder;
pub mod footer;
pub mod header;
pub mod inspector;
pub mod modals;
pub mod sidebar;
pub mod virtualized;

use fm_core::{
    DriveInfo, EntryKind, FileEntry, IconThemeMode, SystemIconBucket, SystemIconKey,
    SystemPlaceKind,
};
use fm_theme::{FONT_UI, ThemePalette, styles};
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, overlay};
use iced::mouse;
use iced::widget::{Text, container, image, opaque, stack, text, text_input, tooltip};
use iced::{
    Alignment::Center, Background, Border, Color, ContentFit, Element, Event, Length, Padding,
    Rectangle, Size, Vector, window,
};
use lucide_icons::iced::{
    icon_file_archive, icon_file_code, icon_file_json, icon_file_text, icon_folder, icon_image,
    icon_redo_2, icon_terminal,
};

use crate::app::{App, Message, TextInputTarget};

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

/// Retorna `true` se a entrada representa um link simbólico ou atalho (`.lnk`).
#[must_use]
pub fn is_shortcut_entry(entry: &FileEntry) -> bool {
    matches!(entry.kind, EntryKind::Symlink { .. })
        || entry
            .extension
            .as_deref()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
}

/// Sobrepõe o emblema circular com seta no canto inferior direito do ícone para links/atalhos.
pub fn with_shortcut_emblem<'a>(
    base: Element<'a, Message>,
    size: f32,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let badge_d = (size * 0.36).round().clamp(10.0, 22.0);
    let arrow_px = (badge_d * 0.62).round().max(7.0);
    let border_w = if badge_d >= 14.0 { 1.5 } else { 1.0 };

    let badge = container(icon_redo_2().size(arrow_px).color(Color::WHITE))
        .width(Length::Fixed(badge_d))
        .height(Length::Fixed(badge_d))
        .align_x(Center)
        .align_y(Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0x43, 0x4C, 0x5E))),
            border: Border {
                color: palette.canvas,
                width: border_w,
                radius: (badge_d * 0.5).into(),
            },
            ..Default::default()
        });

    let overlay = container(badge)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Bottom);

    stack![base, overlay].into()
}

/// Renderiza o ícone de uma entrada de arquivo/pasta respeitando o modo de ícones configurado.
pub fn render_entry_icon<'a>(
    app: &'a App,
    entry: &FileEntry,
    palette: ThemePalette,
    size: f32,
) -> Element<'a, Message> {
    let (fallback, _) = entry_icon(entry, palette, size);
    let base: Element<'a, Message> = if app.config.icon_theme_mode == IconThemeMode::Bundled {
        fallback.into()
    } else {
        let key = SystemIconKey::for_entry(entry, fm_fs::is_archive_file(&entry.path));
        render_system_or_fallback(app, &key, fallback, size)
    };
    if is_shortcut_entry(entry) {
        with_shortcut_emblem(base, size, palette)
    } else {
        base
    }
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
    tip: impl iced::widget::text::IntoFragment<'a>,
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

/// Posiciona um card modal em `(x, y)` sobre backdrop escurecido e bloqueia cliques ao fundo.
pub fn modal_overlay_backdrop_at<'a, M: 'a>(
    card: impl Into<Element<'a, M>>,
    x: f32,
    y: f32,
) -> Element<'a, M> {
    opaque(
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: y.max(0.0),
                right: 0.0,
                bottom: 0.0,
                left: x.max(0.0),
            })
            .style(move |_| styles::modal_backdrop()(&iced::Theme::Light)),
    )
}

/// Posiciona uma janela flutuante não-bloqueante em `(x, y)` sem escurecer ou travar o fundo.
/// Cliques fora do card passam livremente para o gerenciador de arquivos atrás; cliques dentro
/// do card promovem a janela no z-order sem bloquear a interação interna.
pub fn floating_window_at<'a>(
    card: impl Into<Element<'a, Message>>,
    x: f32,
    y: f32,
    window_id: window::Id,
    is_topmost: bool,
) -> Element<'a, Message> {
    let wrapped = Element::new(FloatingWindowFocus {
        content: opaque(card),
        window_id,
        is_topmost,
    });
    container(wrapped)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: y.max(0.0),
            right: 0.0,
            bottom: 0.0,
            left: x.max(0.0),
        })
        .into()
}

struct FloatingWindowFocus<'a> {
    content: Element<'a, Message>,
    window_id: window::Id,
    is_topmost: bool,
}

impl Widget<Message, iced::Theme, iced::Renderer> for FloatingWindowFocus<'_> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if !self.is_topmost
            && matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
            && cursor.is_over(layout.bounds())
        {
            shell.publish(Message::FocusPropertiesWindow(self.window_id));
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// Envolve um `text_input` para abrir menu de contexto (Recortar/Copiar/Colar/Selecionar tudo)
/// com botão direito preservando exatamente a seleção e a posição do cursor internas do widget.
pub fn wrap_text_input_context_menu<'a>(
    input: Element<'a, Message>,
    window_id: window::Id,
    target: TextInputTarget,
    current_value: String,
) -> Element<'a, Message> {
    Element::new(ContextMenuTextInput {
        content: input,
        window_id,
        target,
        current_value,
    })
}

struct ContextMenuTextInput<'a> {
    content: Element<'a, Message>,
    window_id: window::Id,
    target: TextInputTarget,
    current_value: String,
}

impl Widget<Message, iced::Theme, iced::Renderer> for ContextMenuTextInput<'_> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event
            && !shell.is_event_captured()
            && let Some(pos) = cursor.position_over(layout.bounds())
        {
            type Paragraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph;
            let value = text_input::Value::new(&self.current_value);
            let (selection, cursor_index) =
                if tree.children[0].tag == tree::Tag::of::<text_input::State<Paragraph>>() {
                    let state = tree.children[0]
                        .state
                        .downcast_ref::<text_input::State<Paragraph>>();
                    match state.cursor().state(&value) {
                        text_input::cursor::State::Selection { start, end } => {
                            (Some((start.min(end), start.max(end))), end)
                        }
                        text_input::cursor::State::Index(idx) => (None, idx),
                    }
                } else {
                    (None, value.len())
                };

            shell.publish(Message::OpenTextInputContextMenu {
                window_id: self.window_id,
                target: self.target.clone(),
                position: pos,
                selection,
                cursor_index,
            });
            shell.capture_event();
            return;
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{is_shortcut_entry, sanitize_single_line};
    use compact_str::CompactString;
    use fm_core::{EntryId, EntryKind, FileEntry, PermissionsMeta};
    use std::path::PathBuf;

    fn dummy_entry(kind: EntryKind, ext: Option<&str>) -> FileEntry {
        FileEntry {
            id: EntryId(1),
            name: CompactString::from("item"),
            path: PathBuf::from("/tmp/item"),
            kind,
            size_bytes: 0,
            child_count: None,
            modified_at: None,
            created_at: None,
            permissions: PermissionsMeta::PosixMode(0o755),
            is_hidden: false,
            extension: ext.map(CompactString::from),
            mime_badge: None,
        }
    }

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

    #[test]
    fn detects_symlinks_and_lnk_shortcuts_for_emblem_overlay() {
        assert!(is_shortcut_entry(&dummy_entry(
            EntryKind::Symlink {
                target_is_dir: true
            },
            None
        )));
        assert!(is_shortcut_entry(&dummy_entry(
            EntryKind::Symlink {
                target_is_dir: false
            },
            None
        )));
        assert!(is_shortcut_entry(&dummy_entry(
            EntryKind::File,
            Some("lnk")
        )));
        assert!(!is_shortcut_entry(&dummy_entry(EntryKind::Directory, None)));
        assert!(!is_shortcut_entry(&dummy_entry(
            EntryKind::File,
            Some("txt")
        )));
    }
}
