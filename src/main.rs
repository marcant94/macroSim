mod core;
mod i18n;
mod ui;

use ui::MacroSimApp;

#[cfg(not(target_arch = "wasm32"))]
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

/// Arranque en navegador: `wasm-bindgen` ejecuta `main` al llamar a `init()`
/// desde `web/index.html` (el canvas ya existe en el DOM).
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("macrosim_canvas"))
        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok());

    let Some(canvas) = canvas else {
        // Sin canvas no hay nada que dibujar: se avisa por consola y se sale.
        web_sys::console::error_1(&"MacroSim: no se encontró el canvas '#macrosim_canvas'".into());
        return;
    };

    wasm_bindgen_futures::spawn_local(async move {
        let runner = eframe::WebRunner::new();
        let result = runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    setup_fonts(&cc.egui_ctx);
                    Ok(Box::new(MacroSimApp::default()))
                }),
            )
            .await;
        if let Err(err) = result {
            web_sys::console::error_1(&format!("MacroSim: error al iniciar: {err:?}").into());
        }
    });
}

/// Registra fuentes del sistema como fallback de las embebidas de egui.
/// egui trae solo un subconjunto (Ubuntu-Light + NotoEmoji parcial), por lo que
/// sin esto fallan emojis y símbolos como `¤`, `€`, `§` o `⚠`.
#[cfg(not(target_arch = "wasm32"))]
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

/// En web no hay sistema de ficheros que leer: además de las fuentes
/// embebidas en egui, se añaden dos subsets mínimos de Noto (OFL, ver
/// `assets/fonts/OFL.txt`) para los glifos que no cubren: `₿` y `🏛️`.
#[cfg(target_arch = "wasm32")]
fn setup_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};

    const EXTRA: [(&str, &[u8]); 2] = [
        (
            "MacroSimBtc",
            include_bytes!("../assets/fonts/macrosim-btc.ttf"),
        ),
        (
            "MacroSimSymbols",
            include_bytes!("../assets/fonts/macrosim-symbols.ttf"),
        ),
    ];

    let mut fonts = FontDefinitions::default();
    for (name, data) in EXTRA {
        fonts
            .font_data
            .insert(name.to_owned(), FontData::from_static(data).into());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push(name.to_owned());
        }
    }
    ctx.set_fonts(fonts);
}
