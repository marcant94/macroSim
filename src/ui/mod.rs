use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::core::{EconomyState, EnergyState};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &App) {
        app.add_systems(Update, render_dashboard);
    }
}

fn render_dashboard(
    mut contexts: EguiContexts,
    mut economy: ResMut<EconomyState>,
    energy: Res<EnergyState>,
) {
    egui::SidePanel::left("dashboard_panel")
        .default_width(300.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("📊 Estado del Estado");
            ui.separator();

            ui.label(format!("Presupuesto: ${:.2}", economy.money));
            ui.label(format!("Población: {}", economy.population));
            ui.label(format!("Desempleo: {:.1}%", economy.unemployment_rate));

            ui.add_space(10.0);
            ui.heading("⚡ Red Eléctrica");
            ui.label(format!("Capacidad: {:.0} MW", energy.capacity_mw));
            ui.label(format!("Demanda: {:.0} MW", energy.demand_mw));

            ui.add_space(15.0);
            ui.heading("⚙️ Políticas");
            
            // Slider interactivo para ajustar impuestos
            ui.add(egui::Slider::new(&mut economy.corporate_tax, 0.0..=100.0).text("Impuesto Corp. (%)"));
        });
}