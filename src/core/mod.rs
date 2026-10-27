//! Núcleo de simulación: orquesta economía, servicios, energía, suelo,
//! demografía, calendario e historial.
//!
//! Este módulo NO depende de egui; solo contiene datos y lógica.

pub mod crime;
pub mod demographics;
pub mod economy;
pub mod energy;
pub mod land;
pub mod population;
pub mod services;
pub mod settings;
pub mod time;

pub use crime::CrimeState;
pub use demographics::DemographicsState;
pub use economy::EconomyState;
pub use energy::{EnergySource, EnergyState};
pub use land::LandState;
pub use population::PopulationState;
pub use services::{ServiceKind, ServicesState};
pub use time::Calendar;

/// Motivo por el que una orden del jugador (construir, demoler, préstamo…) no
/// se pudo aplicar. La UI traduce cada variante a un mensaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionError {
    /// El presupuesto no cubre el coste de capital.
    InsufficientFunds,
    /// No queda suelo libre suficiente.
    InsufficientLand,
    /// No hay unidades públicas que demoler (o la cantidad es 0).
    NoCapacity,
    /// El préstamo supera el crédito disponible antes de la quiebra.
    CreditLimitExceeded,
    /// No hay deuda que amortizar.
    NoDebt,
    /// La cantidad pedida no tiene sentido (0 o negativa).
    InvalidAmount,
}

/// Una muestra mensual del histórico (todas las series avanzan juntas).
#[derive(Debug, Clone, Copy, Default)]
pub struct HistorySample {
    pub money: f64,
    pub unemployment: f64,
    pub demand: f64,
    pub population: f64,
    pub debt: f64,
    pub crime: f64,
}

/// Histórico de indicadores para las gráficas (ventana deslizante).
#[derive(Debug, Clone, Default)]
pub struct History {
    pub money: Vec<f64>,
    pub unemployment: Vec<f64>,
    pub demand: Vec<f64>,
    pub population: Vec<f64>,
    pub debt: Vec<f64>,
    pub crime: Vec<f64>,
}

impl History {
    pub const MAX_POINTS: usize = 240;

    pub fn push(&mut self, sample: HistorySample) {
        self.money.push(sample.money);
        self.unemployment.push(sample.unemployment);
        self.demand.push(sample.demand);
        self.population.push(sample.population);
        self.debt.push(sample.debt);
        self.crime.push(sample.crime);
        if self.money.len() > Self::MAX_POINTS {
            self.money.remove(0);
            self.unemployment.remove(0);
            self.demand.remove(0);
            self.population.remove(0);
            self.debt.remove(0);
            self.crime.remove(0);
        }
    }

    /// Número de muestras (todas las series tienen la misma longitud).
    pub fn len(&self) -> usize {
        self.money.len()
    }
}

#[derive(Debug, Clone)]
pub struct Simulation {
    pub economy: EconomyState,
    pub services: ServicesState,
    pub land: LandState,
    pub demographics: DemographicsState,
    pub population: PopulationState,
    pub crime: CrimeState,
    pub energy: EnergyState,
    pub clock: Calendar,
    pub history: History,
    pub paused: bool,
    pub speed: u32, // meses por segundo
    pub accumulator: f32,
}

impl Default for Simulation {
    fn default() -> Self {
        let mut sim = Self {
            economy: EconomyState::default(),
            services: ServicesState::default(),
            land: LandState::default(),
            demographics: DemographicsState::default(),
            population: PopulationState::default(),
            crime: CrimeState::default(),
            energy: EnergyState::default(),
            clock: Calendar::default(),
            history: History::default(),
            paused: true, // arranca en pausa: el jugador decide cuándo avanza el tiempo
            speed: 1,
            accumulator: 0.0,
        };
        // Gastos recurrentes y producción del mes al día desde el primer frame:
        // el balance que ve el jugador antes de avanzar el tiempo debe cuadrar.
        sim.energy.set_date(sim.clock.month, sim.clock.year);
        sim.refresh_costs();
        sim
    }
}

impl Simulation {
    /// Avanza la simulación un mes completo.
    pub fn tick_month(&mut self) {
        // 1. Servicios: el sector privado crece según el clima de inversión y
        //    se calcula el gasto mensual público en servicios.
        let climate = services::investment_climate(&self.economy, self.crime.pressure());
        self.economy.service_upkeep = self.services.tick(self.economy.population, climate);
        // 2. Demografía: el reparto de clases deriva con la satisfacción del mes.
        //    Se usa la inseguridad del mes pasado (un mes de retardo es realista).
        let inseguridad = self.crime.pressure();
        self.demographics
            .tick(&self.services, &self.economy, inseguridad);
        // 3. Población: la migración depende de servicios, empleo, impuestos
        //    y también de la inseguridad.
        self.population.tick(
            &self.demographics,
            &self.services,
            &mut self.economy,
            inseguridad,
        );
        // La demanda cambia con la población: cobertura al día para la UI.
        self.services.recompute_coverage(self.economy.population);
        // 4. Delincuencia: emerge del paro, la exclusión y los servicios.
        self.crime
            .tick(&self.services, &self.economy, &self.demographics);
        // 5. Energía: la demanda sigue a la población y se cobran los costes
        //    del mix (mantenimiento + emisiones) en el presupuesto.
        self.energy.tick(&self.economy);
        // La producción se calcula con el mes en curso: las renovables varían
        // con la estación y el clima.
        self.energy.set_date(self.clock.month, self.clock.year);
        let deficit = self.energy.has_deficit();
        self.economy.energy_upkeep = self.energy.monthly_upkeep();
        self.economy.pollution_cost = self.energy.emissions_cost();
        // 6. Economía usa el déficit energético del mes pasado.
        self.economy.tick(&self.demographics, deficit);
        // 7. Calendario.
        self.clock.advance_month();
        // 8. Histórico.
        self.history.push(HistorySample {
            money: self.economy.money,
            unemployment: self.economy.unemployment_rate as f64,
            demand: self.energy.demand_mw as f64,
            population: self.economy.population as f64,
            debt: self.economy.debt,
            crime: self.crime.rate as f64,
        });
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
    pub fn build(&mut self, kind: ServiceKind, units: f32) -> Result<f64, ActionError> {
        if units <= 0.0 {
            return Err(ActionError::InvalidAmount);
        }
        let spec = kind.spec();
        let cost = (units * spec.capital_cost_per_unit) as f64;
        if cost > self.economy.money {
            return Err(ActionError::InsufficientFunds);
        }
        if !self
            .land
            .can_build(&self.services, &self.energy, units * spec.land_per_unit)
        {
            return Err(ActionError::InsufficientLand);
        }
        self.economy.money -= cost;
        self.services.get_mut(kind).public_units += units;
        self.services.recompute_coverage(self.economy.population);
        self.refresh_costs();
        Ok(cost)
    }

    /// Demuele hasta `units` unidades públicas: recupera parte del capital y
    /// libera el suelo. Devuelve lo recuperado.
    pub fn demolish(&mut self, kind: ServiceKind, units: f32) -> Result<f64, ActionError> {
        let spec = kind.spec();
        let available = self.services.get(kind).public_units;
        let units = units.min(available);
        if units <= 0.0 {
            return Err(ActionError::NoCapacity);
        }
        let refund = (units * spec.capital_cost_per_unit * services::DEMOLISH_REFUND) as f64;
        self.services.get_mut(kind).public_units -= units;
        self.economy.money += refund;
        self.services.recompute_coverage(self.economy.population);
        self.refresh_costs();
        Ok(refund)
    }

    /// Recalcula los gastos recurrentes (servicios, energía y emisiones) para
    /// que el balance esté al día sin esperar al siguiente tick mensual.
    pub fn refresh_costs(&mut self) {
        self.economy.service_upkeep = self.services.monthly_upkeep();
        self.economy.energy_upkeep = self.energy.monthly_upkeep();
        self.economy.pollution_cost = self.energy.emissions_cost();
    }

    /// Construye una instalación de `source`: cobra el capital, exige suelo
    /// libre y devuelve lo pagado.
    pub fn build_energy(&mut self, source: EnergySource) -> Result<f64, ActionError> {
        let spec = source.spec();
        let cost = spec.capital_per_plant() as f64;
        if cost > self.economy.money {
            return Err(ActionError::InsufficientFunds);
        }
        if !self
            .land
            .can_build(&self.services, &self.energy, spec.land_per_plant())
        {
            return Err(ActionError::InsufficientLand);
        }
        self.economy.money -= cost;
        self.energy.get_mut(source).plants += 1;
        self.refresh_costs();
        Ok(cost)
    }

    /// Desmantela una instalación de `source`: devuelve parte del capital y
    /// libera el suelo. Devuelve lo recuperado.
    pub fn demolish_energy(&mut self, source: EnergySource) -> Result<f64, ActionError> {
        if self.energy.get(source).plants == 0 {
            return Err(ActionError::NoCapacity);
        }
        let spec = source.spec();
        let refund = (spec.capital_per_plant() * services::DEMOLISH_REFUND) as f64;
        self.energy.get_mut(source).plants -= 1;
        self.economy.money += refund;
        self.refresh_costs();
        Ok(refund)
    }

    /// Pide un préstamo: entra en caja hoy y queda como deuda con intereses.
    /// Lo máximo que se puede pedir es el crédito disponible antes de la quiebra.
    pub fn borrow(&mut self, amount: f64) -> Result<(), ActionError> {
        if amount <= 0.0 {
            return Err(ActionError::InvalidAmount);
        }
        if amount > self.economy.available_credit(&self.demographics) {
            return Err(ActionError::CreditLimitExceeded);
        }
        self.economy.debt += amount;
        self.economy.money += amount;
        Ok(())
    }

    /// Amortiza deuda con la caja disponible. Devuelve lo amortizado.
    pub fn repay(&mut self, amount: f64) -> Result<f64, ActionError> {
        if self.economy.debt <= 0.0 {
            return Err(ActionError::NoDebt);
        }
        if amount <= 0.0 {
            return Err(ActionError::InvalidAmount);
        }
        if amount > self.economy.money {
            return Err(ActionError::InsufficientFunds);
        }
        let amount = amount.min(self.economy.debt);
        self.economy.debt -= amount;
        self.economy.money -= amount;
        Ok(amount)
    }

    /// ¿La ciudad está en quiebra (deuda por encima del límite de crédito)?
    pub fn is_bankrupt(&self) -> bool {
        self.economy.is_bankrupt(&self.demographics)
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
        assert_eq!(sim.build(ServiceKind::Fire, 10.0), Err(ActionError::InsufficientFunds));
        assert_eq!(sim.services.get(ServiceKind::Fire).public_units, before);
    }

    #[test]
    fn construir_sin_suelo_falla() {
        let mut sim = Simulation::default();
        // Deja libre menos de lo que ocupa una unidad de cárcel (2 ha).
        let used = sim.land.used(&sim.services, &sim.energy);
        sim.land.total_hectareas = used + 1.0;
        assert_eq!(sim.build(ServiceKind::Prisons, 1.0), Err(ActionError::InsufficientLand));
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
        assert_eq!(sim.demolish(kind, 1.0), Err(ActionError::NoCapacity));
    }
}
#[cfg(test)]
mod debt_tests {
    use super::*;

    /// El balance que ve el jugador debe incluir los gastos recurrentes desde
    /// el primer frame, no solo tras el primer tick.
    #[test]
    fn el_balance_inicial_incluye_servicios_y_energia() {
        let sim = Simulation::default();
        assert!(sim.economy.service_upkeep > 0.0);
        assert!(sim.economy.energy_upkeep > 0.0);
        assert!(sim.economy.pollution_cost > 0.0);
        let gastos = sim.economy.monthly_expenses(&sim.demographics);
        assert!(gastos > sim.economy.population as f64 * sim.economy.public_spending as f64);
    }

    #[test]
    fn pedir_un_prestamo_entra_en_caja_y_crea_deuda() {
        let mut sim = Simulation::default();
        let money = sim.economy.money;
        let disponible = sim.economy.available_credit(&sim.demographics);

        sim.borrow(disponible / 2.0).expect("debe poder pedir");

        assert!((sim.economy.debt - disponible / 2.0).abs() < 1e-6);
        assert!((sim.economy.money - (money + disponible / 2.0)).abs() < 1e-6);
        assert!(!sim.is_bankrupt());
    }

    #[test]
    fn no_se_puede_pedir_mas_que_el_credito_disponible() {
        let mut sim = Simulation::default();
        let disponible = sim.economy.available_credit(&sim.demographics);
        assert_eq!(
            sim.borrow(disponible + 1.0),
            Err(ActionError::CreditLimitExceeded)
        );
        assert_eq!(sim.economy.debt, 0.0);
        assert_eq!(sim.borrow(0.0), Err(ActionError::InvalidAmount));
    }

    #[test]
    fn amortizar_baja_la_deuda_y_la_caja() {
        let mut sim = Simulation::default();
        sim.borrow(100_000.0).expect("préstamo");
        let money = sim.economy.money;

        let pagado = sim.repay(40_000.0).expect("amortizar");

        assert!((pagado - 40_000.0).abs() < 1e-6);
        assert!((sim.economy.debt - 60_000.0).abs() < 1e-6);
        assert!((sim.economy.money - (money - 40_000.0)).abs() < 1e-6);
    }

    #[test]
    fn amortizar_sin_deuda_o_sin_caja_falla() {
        let mut sim = Simulation::default();
        assert_eq!(sim.repay(1_000.0), Err(ActionError::NoDebt));

        sim.borrow(1_000.0).expect("préstamo");
        let deuda = sim.economy.debt;
        sim.economy.money = 0.0;
        assert_eq!(sim.repay(1_000.0), Err(ActionError::InsufficientFunds));
        assert_eq!(sim.economy.debt, deuda);
    }

    #[test]
    fn el_deficit_se_financia_con_deuda_y_la_caja_nunca_es_negativa() {
        let mut sim = Simulation::default();
        sim.economy.public_spending = 20.0; // gasto insostenible
        sim.economy.money = 10_000.0;

        for _ in 0..6 {
            sim.tick_month();
            assert!(sim.economy.money >= 0.0);
        }
        assert!(sim.economy.debt > 0.0, "el déficit debe generar deuda");
    }

    #[test]
    fn los_intereses_crecen_con_la_deuda_y_aprietan_el_balance() {
        let mut sim = Simulation::default();
        let balance_sin_deuda = sim.economy.monthly_balance(&sim.demographics);
        let intereses_cero = sim.economy.monthly_interest(&sim.demographics);
        let tipo_cero = sim.economy.interest_rate(&sim.demographics);

        sim.borrow(500_000.0).expect("préstamo");

        assert_eq!(intereses_cero, 0.0);
        assert!(sim.economy.interest_rate(&sim.demographics) > tipo_cero);
        assert!(sim.economy.monthly_interest(&sim.demographics) > 0.0);
        assert!(sim.economy.monthly_balance(&sim.demographics) < balance_sin_deuda);
    }

    #[test]
    fn la_quiebra_dispara_delincuencia_y_frena_la_inversion() {
        let mut sano = Simulation::default();
        let mut quebrado = Simulation::default();
        // Deuda por encima del límite de crédito.
        let limite = quebrado.economy.credit_limit(&quebrado.demographics);
        quebrado.economy.debt = limite * 1.5;
        assert!(quebrado.is_bankrupt());

        for _ in 0..24 {
            sano.tick_month();
            quebrado.tick_month();
        }
        assert!(
            quebrado.crime.rate > sano.crime.rate,
            "la quiebra debe alimentar la delincuencia ({} > {})",
            quebrado.crime.rate,
            sano.crime.rate
        );
        assert!(
            quebrado.population.net_rate < sano.population.net_rate,
            "con quiebra la gente se va"
        );
    }
}
