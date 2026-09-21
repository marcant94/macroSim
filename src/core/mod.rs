//! Núcleo de simulación: orquesta economía, energía, calendario e historial.
//!
//! Este módulo NO depende de egui; solo contiene datos y lógica.

pub mod economy;
pub mod energy;
pub mod settings;
pub mod time;

pub use economy::EconomyState;
pub use energy::EnergyState;
pub use time::Calendar;

/// Histórico de indicadores para las gráficas (ventana deslizante).
#[derive(Debug, Clone, Default)]
pub struct History {
    pub money: Vec<f64>,
    pub unemployment: Vec<f64>,
    pub demand: Vec<f64>,
}

impl History {
    pub const MAX_POINTS: usize = 240;

    pub fn push(&mut self, money: f64, unemployment: f64, demand: f64) {
        self.money.push(money);
        self.unemployment.push(unemployment);
        self.demand.push(demand);
        if self.money.len() > Self::MAX_POINTS {
            self.money.remove(0);
            self.unemployment.remove(0);
            self.demand.remove(0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Simulation {
    pub economy: EconomyState,
    pub energy: EnergyState,
    pub clock: Calendar,
    pub history: History,
    pub paused: bool,
    pub speed: u32, // meses por segundo
    pub accumulator: f32,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            economy: EconomyState::default(),
            energy: EnergyState::default(),
            clock: Calendar::default(),
            history: History::default(),
            paused: true, // arranca en pausa: el jugador decide cuándo avanza el tiempo
            speed: 1,
            accumulator: 0.0,
        }
    }
}

impl Simulation {
    /// Avanza la simulación un mes completo.
    pub fn tick_month(&mut self) {
        // 1. Economía usa el déficit energético del mes pasado.
        let deficit = self.energy.has_deficit();
        self.economy.tick(deficit);
        // 2. La energía crece con la nueva actividad económica.
        self.energy.tick(&self.economy);
        // 3. Calendario.
        self.clock.advance_month();
        // 4. Histórico.
        self.history.push(
            self.economy.money,
            self.economy.unemployment_rate as f64,
            self.energy.demand_mw as f64,
        );
    }

    /// Bucle de tiempo real: acumula `dt` y ejecuta ticks mensuales discretos.
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