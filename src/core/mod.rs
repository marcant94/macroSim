//! Núcleo de simulación: orquesta economía, servicios, energía, suelo,
//! demografía, calendario e historial.
//!
//! Este módulo NO depende de egui; solo contiene datos y lógica.

pub mod demographics;
pub mod economy;
pub mod energy;
pub mod land;
pub mod services;
pub mod settings;
pub mod time;

pub use demographics::DemographicsState;
pub use economy::EconomyState;
pub use energy::EnergyState;
pub use land::LandState;
pub use services::ServicesState;
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
    pub services: ServicesState,
    // Suelo: se expondrá en la UI cuando exista el panel de servicios.
    #[allow(dead_code)]
    pub land: LandState,
    pub demographics: DemographicsState,
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
            services: ServicesState::default(),
            land: LandState::default(),
            demographics: DemographicsState::default(),
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
        // 1. Servicios: el sector privado crece según el clima de inversión y
        //    se calcula el gasto mensual público en servicios.
        let climate = services::investment_climate(&self.economy);
        self.economy.service_upkeep = self.services.tick(self.economy.population, climate);
        // 2. Demografía: el reparto de clases deriva con la satisfacción del mes.
        self.demographics.tick(&self.services, &self.economy);
        // 3. Economía usa el déficit energético del mes pasado.
        let deficit = self.energy.has_deficit();
        self.economy.tick(deficit);
        // 4. La energía crece con la nueva actividad económica.
        self.energy.tick(&self.economy);
        // 5. Calendario.
        self.clock.advance_month();
        // 6. Histórico.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Un año de simulación no debe romper invariantes básicas del esqueleto.
    #[test]
    fn un_anho_de_ticks_mantiene_invariantes() {
        let mut sim = Simulation::default();
        for _ in 0..12 {
            sim.tick_month();
        }
        assert_eq!(sim.clock.year, 2027);
        // La cobertura de cada servicio está en 0..=1.
        for st in &sim.services.services {
            assert!((0.0..=1.0).contains(&st.coverage));
            assert!(st.private_units.is_finite() && st.public_units.is_finite());
        }
        // El suelo ocupado no puede superar el límite (los servicios parten
        // por debajo del total y crecen acotados a la demanda).
        let used = sim.land.used(&sim.services, &sim.energy);
        assert!(used <= sim.land.total_hectareas);
        // Las cuotas de clase suman 1.
        let d = &sim.demographics;
        assert!((d.low_share + d.mid_share + d.high_share - 1.0).abs() < 1e-4);
    }

    /// El sector privado cubre demanda insatisfecha cuando el clima es bueno.
    #[test]
    fn sector_privado_crece_con_buen_clima() {
        let mut sim = Simulation::default();
        sim.economy.corporate_tax = 5.0; // clima de inversión muy favorable
        let before = sim.services.get(crate::core::services::ServiceKind::Health).private_units;
        for _ in 0..6 {
            sim.tick_month();
        }
        let after = sim.services.get(crate::core::services::ServiceKind::Health).private_units;
        assert!(after > before);
    }
}