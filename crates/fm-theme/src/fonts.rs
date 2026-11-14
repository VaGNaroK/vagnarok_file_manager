use iced::font::{Family, Weight};
use iced::Font;

pub const INTER_REGULAR_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
pub const INTER_SEMIBOLD_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
pub const INTER_BOLD_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");
pub const JETBRAINS_REGULAR_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
pub const JETBRAINS_MEDIUM_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Medium.ttf");
pub const JETBRAINS_BOLD_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Bold.ttf");

pub const FONT_UI: Font = Font {
    family: Family::Name("Inter"),
    weight: Weight::Normal,
    ..Font::DEFAULT
};

pub const FONT_UI_SEMIBOLD: Font = Font {
    family: Family::Name("Inter"),
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

pub const FONT_UI_BOLD: Font = Font {
    family: Family::Name("Inter"),
    weight: Weight::Bold,
    ..Font::DEFAULT
};

pub const FONT_MONO: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Medium,
    ..Font::MONOSPACE
};

pub const FONT_MONO_BOLD: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Bold,
    ..Font::MONOSPACE
};

/// Lista de todas as fontes embutidas no binário para registro no boot do Iced.
pub fn embedded_fonts() -> [&'static [u8]; 6] {
    [
        INTER_REGULAR_BYTES,
        INTER_SEMIBOLD_BYTES,
        INTER_BOLD_BYTES,
        JETBRAINS_REGULAR_BYTES,
        JETBRAINS_MEDIUM_BYTES,
        JETBRAINS_BOLD_BYTES,
    ]
}
