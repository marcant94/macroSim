mod core;
mod i18n;
mod ui;

use ui::MacroSimApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Simulador de Política y Economía"),
        ..Default::default()
    };
    eframe::run_native(
        "MacroSim",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(MacroSimApp::default()))
        }),
    )
}

/// Registra fuentes del sistema como fallback de las embebidas de egui.
/// egui trae solo un subconjunto (Ubuntu-Light + NotoEmoji parcial), por lo que
/// sin esto fallan emojis y símbolos como `¤`, `€`, `§` o `⚠`.
fn setup_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};

    let mut fonts = FontDefinitions::default();

    // (nombre, rutas candidatas por distribución Linux)
    let candidates: &[(&str, &[&str])] = &[
        (
            "NotoSans",
            &[
                "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            ],
        ),
        (
            "DejaVuSans",
            &[
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            ],
        ),
        (
            "NotoSansSymbols2",
            &["/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf"],
        ),
        (
            "NotoColorEmoji",
            &[
                "/usr/share/fonts/noto/NotoColorEmoji.ttf",
                "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
            ],
        ),
    ];

    for (name, paths) in candidates {
        let mut loaded = false;
        for path in *paths {
            if let Ok(data) = std::fs::read(path) {
                println!("fonts: cargada '{}' desde {}", name, path);
                loaded = true;
                fonts
                    .font_data
                    .insert(name.to_string(), FontData::from_owned(data).into());
                // Se añaden al final de cada familia: actúan como fallback
                // para los glifos que no tengan las fuentes por defecto.
                if let Some(proportional) = fonts.families.get_mut(&FontFamily::Proportional) {
                    proportional.push(name.to_string());
                }
                if let Some(monospace) = fonts.families.get_mut(&FontFamily::Monospace) {
                    monospace.push(name.to_string());
                }
                break;
            }
        }
        if !loaded {
            println!("fonts: '{}' no encontrada en las rutas candidatas", name);
        }
    }

    ctx.set_fonts(fonts);
}