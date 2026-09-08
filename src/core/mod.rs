use bevy::prelude::*;

// 1. Recursos (Datos globales)
#[derive(Resource)]
pub struct EconomyState {
    pub money: f64,
    pub population: u32,
    pub unemployment_rate: f32, // Porcentaje (ej: 10.5)
    pub corporate_tax: f32,     // Porcentaje (ej: 21.0)
}

#[derive(Resource)]
pub struct EnergyState {
    pub capacity_mw: f32,
    pub demand_mw: f32,
}

// 2. Plugin principal de lógica
pub struct CoreSimulationPlugin;

impl Plugin for CoreSimulationPlugin {
    fn build(&self, app: &App) {
        // Inicializamos los datos
        app.insert_resource(EconomyState {
            money: 1_000_000.0,
            population: 50_000,
            unemployment_rate: 8.5,
            corporate_tax: 20.0,
        })
        .insert_resource(EnergyState {
            capacity_mw: 500.0,
            demand_mw: 420.0,
        })
        // Bucles de tiempo fijo para la economía (se ejecuta cada 1 segundo = 1 mes simulado)
        .add_systems(FixedUpdate, monthly_simulation_tick);
    }
}

// 3. Sistema que simula el paso del tiempo
fn monthly_simulation_tick(mut economy: ResMut<EconomyState>, energy: Res<EnergyState>) {
    // Ejemplo simplificado de lógica macroeconómica
    let monthly_income = economy.population as f64 * (economy.corporate_tax as f64 * 0.5);
    let monthly_expenses = economy.population as f64 * 2.0;

    economy.money += monthly_income - monthly_expenses;

    // Si la demanda supera la capacidad, el paro sube un poco por falta de energía
    if energy.demand_mw > energy.capacity_mw {
        economy.unemployment_rate += 0.2;
    }
}