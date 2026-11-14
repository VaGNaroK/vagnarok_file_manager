//! Design system do Vagnarok File Manager (`AetherLight` e `ObsidianFlow`).
//!
//! Centraliza paletas semânticas, fontes embutidas (`Inter` e `JetBrains Mono`) e estilos Iced.

pub mod fonts;
pub mod palette;
pub mod styles;

pub use fonts::{
    FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, embedded_fonts,
};
pub use palette::{Spacing, ThemePalette};
