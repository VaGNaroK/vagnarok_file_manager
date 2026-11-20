#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod archive_jobs;
mod bulk_rename;
mod duplicate_finder;
mod views;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new(
                    "warn,fm_app=info,fm_fs=info,fm_index=info,fm_telemetry=info,fm_preview=info",
                )
            }),
        )
        .init();

    let mut app_builder = iced::daemon(app::App::boot, app::App::update, app::App::view)
        .title(app::App::title)
        .theme(app::App::iced_theme)
        .subscription(app::App::subscription)
        .default_font(fm_theme::FONT_UI)
        .font(lucide_icons::LUCIDE_FONT_BYTES);

    for font_bytes in fm_theme::embedded_fonts() {
        app_builder = app_builder.font(font_bytes);
    }

    app_builder.run()
}
