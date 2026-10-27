//! Paletas de color de la interfaz.
//!
//! La elección del jugador vive en `core::settings::Theme` (datos puros); aquí
//! se traduce a `egui::Visuals`. Se aplica una sola vez por cambio de paleta,
//! no en cada frame.

use egui::{Color32, CursorIcon, Visuals};

use crate::core::settings::Theme;
use crate::core::EnergySource;

/// Visuals de cada paleta.
pub fn visuals(theme: Theme) -> Visuals {
    match theme {
        Theme::Midnight => midnight(),
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
        Theme::Sepia => sepia(),
    }
}

/// Aplica la paleta de colores.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    ctx.set_visuals(visuals(theme));
}

/// Cursor de mano para un widget clicable.
///
/// `Visuals::interact_cursor` existe en egui 0.32 pero **no lo lee nadie**
/// (comprobado en el código del crate), así que hay que avisar al contexto a
/// mano en cada widget interactivo. Gana el último `set_cursor_icon` del frame.
pub trait HandCursor {
    fn hand(self, ui: &egui::Ui) -> Self;
}

impl HandCursor for egui::Response {
    fn hand(self, ui: &egui::Ui) -> Self {
        if self.hovered() && ui.is_enabled() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        self
    }
}

/// Color del icono de cada fuente de energía.
///
/// epaint no soporta fuentes de color (CBDT/COLR/sbix): todos los emojis se
/// dibujan como contorno monocromo. El "color" se consigue tiñendo el glifo.
/// Son tonos medios, legibles tanto sobre fondos oscuros como claros o sepia.
pub fn color_fuente(source: EnergySource) -> Color32 {
    match source {
        EnergySource::Coal => Color32::from_rgb(158, 158, 158),
        EnergySource::Oil => Color32::from_rgb(196, 141, 86),
        EnergySource::Gas => Color32::from_rgb(214, 124, 72),
        EnergySource::Nuclear => Color32::from_rgb(96, 186, 120),
        EnergySource::Geothermal => Color32::from_rgb(206, 118, 74),
        EnergySource::Wind => Color32::from_rgb(132, 190, 220),
        EnergySource::Solar => Color32::from_rgb(214, 178, 64),
        EnergySource::Hydro => Color32::from_rgb(88, 150, 214),
    }
}

/// Papel cálido: fondo crema, texto marrón oscuro y selección terracota.
fn sepia() -> Visuals {
    let mut visuals = Visuals::light();
    visuals.panel_fill = Color32::from_rgb(244, 236, 216);
    visuals.window_fill = Color32::from_rgb(251, 246, 234);
    visuals.extreme_bg_color = Color32::from_rgb(232, 222, 199);
    visuals.faint_bg_color = Color32::from_rgb(238, 229, 208);
    visuals.override_text_color = Some(Color32::from_rgb(59, 48, 36));
    visuals.selection.bg_fill = Color32::from_rgb(196, 116, 71);
    visuals.hyperlink_color = Color32::from_rgb(150, 84, 44);
    visuals
}

/// Azul profundo tipo consola, con acento azul claro.
fn midnight() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(18, 24, 34);
    visuals.window_fill = Color32::from_rgb(24, 32, 44);
    visuals.extreme_bg_color = Color32::from_rgb(12, 17, 24);
    visuals.faint_bg_color = Color32::from_rgb(28, 36, 50);
    visuals.selection.bg_fill = Color32::from_rgb(46, 109, 180);
    visuals.hyperlink_color = Color32::from_rgb(120, 180, 255);
    visuals
}
