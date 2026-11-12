#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod code_view;
mod data_view;
mod graph;
mod insight;

fn main() -> eframe::Result {
    let compact = std::env::args().any(|arg| arg == "--compact");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("DIVULGE | Investigation workbench")
            .with_inner_size(if compact {
                [1280.0, 720.0]
            } else {
                [1440.0, 900.0]
            })
            .with_min_inner_size([1000.0, 620.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "DIVULGE",
        options,
        Box::new(|cc| Ok(Box::new(app::DivulgeApp::new(cc)))),
    )
}
