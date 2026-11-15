mod app;
mod views;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut app_builder = iced::application(app::App::boot, app::App::update, app::App::view)
        .title(app::App::title)
        .theme(app::App::iced_theme)
        .subscription(app::App::subscription)
        .default_font(fm_theme::FONT_UI)
        .window_size((1320.0, 860.0))
        .centered()
        .font(lucide_icons::LUCIDE_FONT_BYTES);

    for font_bytes in fm_theme::embedded_fonts() {
        app_builder = app_builder.font(font_bytes);
    }

    app_builder.run()
}
