use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod core;
mod ui;

fn main() {
    App::new()
        // Plugins por defecto de Bevy (Render, Ventana, Input, etc.)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Simulador de Economía y Energía".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin) // Plugin para la UI tipo Dashboard
        // Plugins propios
        .add_plugins(core::CoreSimulationPlugin)
        .add_plugins(ui::UiPlugin)
        .run();
}