use serde::{Deserialize, Serialize};

// 1. Datos globales (antes Resources de Bevy, ahora structs simples)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    pub money: f64,
    pub population: u32,
    pub unemployment_rate: f32, // Porcentaje (ej: 10.5)
    pub corporate_tax: f32,     // Porcentaje (ej: 21.0)
    pub income_tax: f32,
    pub public_spending: f32, // gasto mensual por habitante
    pub month: u32,
    pub year: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyState {
    pub capacity_mw: f32,
    pub demand_mw: f32,
    pub price_per_mwh: f32,
}

#[derive(Debug, Clone, Default)]
pub struct History {
    pub money: Vec<f64>,
    pub unemployment: Vec<f64>,
    pub demand: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Simulation {
    pub economy: EconomyState,
    pub energy: EnergyState,
    pub history: History,
    pub paused: bool,
    pub speed: u32, // meses por segundo
    pub accumulator: f32,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            economy: EconomyState {
                money: 1_000_000.0,
                population: 50_000,
                unemployment_rate: 8.5,
                corporate_tax: 20.0,
                income_tax: 15.0,
                public_spending: 2.0,
                month: 1,
                year: 2026,
            },
            energy: EnergyState {
                capacity_mw: 500.0,
                demand_mw: 420.0,
                price_per_mwh: 65.0,
            },
            history: History::default(),
            paused: false,
            speed: 1,
            accumulator: 0.0,
        }
    }
}

impl Simulation {
    pub fn tick_month(&mut self) {
        let eco = &mut self.economy;
        let en = &self.energy;

        // Ingresos: impuesto sociedades + IRPF simplificados
        let monthly_income =
            eco.population as f64 * (eco.corporate_tax as f64 * 0.5 + eco.income_tax as f64 * 0.8);
        let monthly_expenses = eco.population as f64 * eco.public_spending as f64;

        eco.money += monthly_income - monthly_expenses;

        // Energía: si la demanda supera la capacidad, sube el paro
        if en.demand_mw > en.capacity_mw {
            eco.unemployment_rate += 0.2;
        } else if eco.unemployment_rate > 3.0 {
            eco.unemployment_rate -= 0.05;
        }
        eco.unemployment_rate = eco.unemployment_rate.clamp(0.0, 40.0);

        // Demanda energética crece con población y actividad
        // (modelo simple placeholder estilo SimCity sin mapa)
        let activity = 1.0 - (eco.unemployment_rate / 100.0);

        // Avance calendario
        eco.month += 1;
        if eco.month > 12 {
            eco.month = 1;
            eco.year += 1;
        }

        self.history.money.push(eco.money);
        self.history
            .unemployment
            .push(eco.unemployment_rate as f64);
        self.history.demand.push(en.demand_mw as f64);
        if self.history.money.len() > 240 {
            self.history.money.remove(0);
            self.history.unemployment.remove(0);
            self.history.demand.remove(0);
        }

        let _ = activity;
    }

    pub fn update(&mut self, dt_seconds: f32) {
        if self.paused {
            return;
        }
        self.accumulator += dt_seconds * self.speed as f32;
        while self.accumulator >= 1.0 {
            self.accumulator -= 1.0;
            self.tick_month();
        }
    }
}