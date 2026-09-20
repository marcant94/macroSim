mod core;
mod ui;

use ui::MacroSimApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Simulador de Economía y Energía"),
        ..Default::default()
    };
    eframe::run_native(
        "MacroSim",
        options,
        Box::new(|_cc| Ok(Box::new(MacroSimApp::default()))),
    )
}