//! Núcleo de simulación: orquesta economía, servicios, energía, suelo,
//! demografía, calendario e historial.
//!
//! Este módulo NO depende de egui; solo contiene datos y lógica.

pub mod demographics;
pub mod economy;
pub mod energy;
pub mod land;
pub mod population;
pub mod services;
pub mod settings;
pub mod time;

pub use demographics::DemographicsState;
pub use economy::EconomyState;
pub use energy::EnergyState;
pub use land::LandState;
pub use population::PopulationState;
pub use services::{ServiceKind, ServicesState};
pub use time::Calendar;

/// Motivo por el que una orden de construcción/demolición no se pudo aplicar.
/// La UI traduce cada variante a un mensaje (el core no conoce textos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    /// El presupuesto no cubre el coste de capital.
    InsufficientFunds,
    /// No queda suelo libre suficiente.
    InsufficientLand,
    /// No hay unidades públicas que demoler (o la cantidad es 0).
    NoCapacity,
}

/// Histórico de indicadores para las gráficas (ventana deslizante).
#[derive(Debug, Clone, Default)]
pub struct History {
    pub money: Vec<f64>,
    pub unemployment: Vec<f64>,
    pub demand: Vec<f64>,
    pub population: Vec<f64>,
}

impl History {
    pub const MAX_POINTS: usize = 240;

    pub fn push(&mut self, money: f64, unemployment: f64, demand: f64, population: f64) {
        self.money.push(money);
        self.unemployment.push(unemployment);
        self.demand.push(demand);
        self.population.push(population);
        if self.money.len() > Self::MAX_POINTS {
            self.money.remove(0);
            self.unemployment.remove(0);
            self.demand.remove(0);
            self.population.remove(0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Simulation {
    pub economy: EconomyState,
    pub services: ServicesState,
    pub land: LandState,
    pub demographics: DemographicsState,
    pub population: PopulationState,
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
            population: PopulationState::default(),
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
        // 3. Población: la migración depende de servicios, empleo y presión fiscal.
        self.population
            .tick(&self.demographics, &self.services, &mut self.economy);
        // La demanda cambia con la población: cobertura al día para la UI.
        self.services.recompute_coverage(self.economy.population);
        // 4. Economía usa el déficit energético del mes pasado.
        let deficit = self.energy.has_deficit();
        self.economy.tick(&self.demographics, deficit);
        // 5. La energía crece con la nueva actividad económica.
        self.energy.tick(&self.economy);
        // 6. Calendario.
        self.clock.advance_month();
        // 7. Histórico.
        self.history.push(
            self.economy.money,
            self.economy.unemployment_rate as f64,
            self.energy.demand_mw as f64,
            self.economy.population as f64,
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

    /// Construye `units` unidades públicas: cobra el capital al contado,
    /// exige suelo libre y devuelve lo pagado.
    pub fn build(&mut self, kind: ServiceKind, units: f32) -> Result<f64, BuildError> {
        if units <= 0.0 {
            return Err(BuildError::NoCapacity);
        }
        let spec = kind.spec();
        let cost = (units * spec.capital_cost_per_unit) as f64;
        if cost > self.economy.money {
            return Err(BuildError::InsufficientFunds);
        }
        if !self
            .land
            .can_build(&self.services, &self.energy, units * spec.land_per_unit)
        {
            return Err(BuildError::InsufficientLand);
        }
        self.economy.money -= cost;
        self.services.get_mut(kind).public_units += units;
        self.services.recompute_coverage(self.economy.population);
        // Refresca el gasto recurrente para que el balance reaccione al instante
        // (también en pausa), en vez de esperar al siguiente tick mensual.
        self.economy.service_upkeep = self.services.monthly_upkeep();
        Ok(cost)
    }

    /// Demuele hasta `units` unidades públicas: recupera parte del capital y
    /// libera el suelo. Devuelve lo recuperado.
    pub fn demolish(&mut self, kind: ServiceKind, units: f32) -> Result<f64, BuildError> {
        let spec = kind.spec();
        let available = self.services.get(kind).public_units;
        let units = units.min(available);
        if units <= 0.0 {
            return Err(BuildError::NoCapacity);
        }
        let refund = (units * spec.capital_cost_per_unit * services::DEMOLISH_REFUND) as f64;
        self.services.get_mut(kind).public_units -= units;
        self.economy.money += refund;
        self.services.recompute_coverage(self.economy.population);
        self.economy.service_upkeep = self.services.monthly_upkeep();
        Ok(refund)
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

    #[test]
    fn construir_cobra_capital_consume_suelo_y_actualiza_cobertura() {
        let mut sim = Simulation::default();
        let kind = ServiceKind::Health;
        let money_before = sim.economy.money;
        let units_before = sim.services.get(kind).public_units;
        let land_before = sim.land.used(&sim.services, &sim.energy);
        let cov_before = sim.services.get(kind).coverage;

        let paid = sim.build(kind, 10.0).expect("construir debe funcionar");

        assert!((paid - 240_000.0).abs() < 1e-6);
        assert!((sim.economy.money - (money_before - paid)).abs() < 1e-6);
        assert!((sim.services.get(kind).public_units - units_before - 10.0).abs() < 1e-4);
        assert!(sim.land.used(&sim.services, &sim.energy) > land_before);
        assert!(sim.services.get(kind).coverage >= cov_before);
        // El mantenimiento mensual se refresca sin esperar al tick.
        assert!((sim.economy.service_upkeep - sim.services.monthly_upkeep()).abs() < 1e-6);
        assert!(sim.economy.service_upkeep > 0.0);
    }

    #[test]
    fn construir_sin_presupuesto_falla_sin_cambios() {
        let mut sim = Simulation::default();
        sim.economy.money = 0.0;
        let before = sim.services.get(ServiceKind::Fire).public_units;
        assert_eq!(sim.build(ServiceKind::Fire, 10.0), Err(BuildError::InsufficientFunds));
        assert_eq!(sim.services.get(ServiceKind::Fire).public_units, before);
    }

    #[test]
    fn construir_sin_suelo_falla() {
        let mut sim = Simulation::default();
        // Deja libre menos de lo que ocupa una unidad de cárcel (2 ha).
        let used = sim.land.used(&sim.services, &sim.energy);
        sim.land.total_hectareas = used + 1.0;
        assert_eq!(sim.build(ServiceKind::Prisons, 1.0), Err(BuildError::InsufficientLand));
    }

    #[test]
    fn demoler_recupera_parte_del_capital_y_libera_suelo() {
        let mut sim = Simulation::default();
        let kind = ServiceKind::Education;
        let money_before = sim.economy.money;
        let units_before = sim.services.get(kind).public_units;
        let land_before = sim.land.used(&sim.services, &sim.energy);

        let refund = sim.demolish(kind, 5.0).expect("demoler debe funcionar");

        assert!((refund - 5.0 * 2_400.0 * 0.5).abs() < 1e-6);
        assert!((sim.economy.money - (money_before + refund)).abs() < 1e-6);
        assert!((sim.services.get(kind).public_units - units_before + 5.0).abs() < 1e-4);
        assert!(sim.land.used(&sim.services, &sim.energy) < land_before);
    }

    #[test]
    fn demoler_mas_de_lo_construido_solo_retira_lo_que_hay() {
        let mut sim = Simulation::default();
        let kind = ServiceKind::Waste;
        let units = sim.services.get(kind).public_units;
        assert!(sim.demolish(kind, units + 1_000.0).is_ok());
        assert!(sim.services.get(kind).public_units.abs() < 1e-4);
        assert_eq!(sim.demolish(kind, 1.0), Err(BuildError::NoCapacity));
    }
}