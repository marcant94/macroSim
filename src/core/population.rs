//! Dinámica de población: la "regla de oro" del juego.
//!
//! La población no es un slider: es el marcador que resume si el jugador lo está
//! haciendo bien. Cada mes se calcula una **tasa neta de migración** a partir de
//! tres factores y se aplica sobre la población total:
//!
//! - **Servicios**: la satisfacción media (cobertura pública/privada por clase)
//!   frente a un punto neutro.
//! - **Empleo**: paro por encima del nivel neutro expulsa habitantes; por debajo
//!   los atrae.
//! - **Presión fiscal**: subir impuestos por encima de la referencia ahuyenta
//!   residentes; bajarlos los atrae (pero reduce ingresos).
//! - **Inseguridad**: la delincuencia empuja a la gente a irse (además de
//!   restar satisfacción, que ya entra por el primer factor).
//!
//! La tasa está acotada para que ningún factor pueda provocar espirales
//! incontrolables: la demografía se mueve despacio, como en la realidad.

use serde::{Deserialize, Serialize};

use super::demographics::DemographicsState;
use super::economy::EconomyState;
use super::services::ServicesState;

/// Satisfacción media con la que la ciudad ni atrae ni pierde población.
/// Calibrada para que la partida inicial (≈80 % de cobertura) esté en equilibrio.
pub const NEUTRAL_SATISFACTION: f32 = 0.45;
/// Desempleo que no influye en la migración (el de la partida inicial).
pub const NEUTRAL_UNEMPLOYMENT: f32 = 8.0;

/// Cuánto pesa cada punto de desviación en la tasa mensual de migración.
const SERVICES_SLOPE: f32 = 0.02;
const JOBS_SLOPE: f32 = 0.0006;
/// Un +100 % de carga fiscal (el doble de recaudación por habitante) resta
/// 0,6 puntos porcentuales de población al mes.
const TAXES_SLOPE: f32 = 0.006;
/// Con la delincuencia al máximo, la ciudad pierde 0,6 % de población al mes.
const CRIME_SLOPE: f32 = 0.006;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PopulationState {
    /// Tasa neta del último mes, como fracción de la población
    /// (positiva = llegan habitantes; negativa = se van).
    #[serde(default)]
    pub net_rate: f32,
}

/// Desglose de los factores que mueven la población (informes y UI).
#[derive(Debug, Clone, Copy, Default)]
pub struct PopulationOutlook {
    /// Satisfacción media ponderada de la población (0..=1).
    pub satisfaction: f32,
    /// Tasa neta mensual resultante (fracción de la población).
    pub net_rate: f32,
    /// Aportación de los servicios públicos/privados.
    pub from_services: f32,
    /// Aportación del empleo.
    pub from_jobs: f32,
    /// Aportación de la presión fiscal.
    pub from_taxes: f32,
    /// Aportación de la inseguridad.
    pub from_crime: f32,
}

impl PopulationState {
    /// Cotas de la tasa mensual: ±2 % / +1,5 % como máximo.
    pub const MIN_RATE: f32 = -0.02;
    pub const MAX_RATE: f32 = 0.015;

    /// Calcula el desglose del mes sin aplicar ningún cambio.
    pub fn outlook(
        &self,
        demographics: &DemographicsState,
        services: &ServicesState,
        economy: &EconomyState,
        crime_pressure: f32,
    ) -> PopulationOutlook {
        let satisfaction =
            demographics.average_satisfaction(services, economy.population, economy, crime_pressure);
        let from_services = (satisfaction - NEUTRAL_SATISFACTION) * SERVICES_SLOPE;
        let from_jobs = (NEUTRAL_UNEMPLOYMENT - economy.unemployment_rate) * JOBS_SLOPE;
        let from_taxes = -economy.tax_burden(demographics) * TAXES_SLOPE;
        let from_crime = -crime_pressure.clamp(0.0, 1.0) * CRIME_SLOPE;
        let net_rate = (from_services + from_jobs + from_taxes + from_crime)
            .clamp(Self::MIN_RATE, Self::MAX_RATE);

        PopulationOutlook {
            satisfaction,
            net_rate,
            from_services,
            from_jobs,
            from_taxes,
            from_crime,
        }
    }

    /// Avanza la población un mes aplicando la migración calculada.
    pub fn tick(
        &mut self,
        demographics: &DemographicsState,
        services: &ServicesState,
        economy: &mut EconomyState,
        crime_pressure: f32,
    ) -> PopulationOutlook {
        let outlook = self.outlook(demographics, services, economy, crime_pressure);
        let next = (economy.population as f32 * (1.0 + outlook.net_rate))
            .round()
            .clamp(0.0, u32::MAX as f32);
        economy.population = next as u32;
        self.net_rate = outlook.net_rate;
        outlook
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ServiceKind, Simulation};

    #[test]
    fn partida_inicial_queda_en_equilibrio() {
        let mut sim = Simulation::default();
        let before = sim.economy.population;
        sim.tick_month();
        let delta = sim.economy.population as i64 - before as i64;
        assert!(
            delta.abs() < 100,
            "la ciudad inicial no debe dispararse ni vaciarse (delta = {delta})"
        );
    }

    #[test]
    fn buenos_servicios_y_empleo_atraen_poblacion() {
        let mut sim = Simulation::default();
        sim.economy.unemployment_rate = 3.0;
        for kind in ServiceKind::ALL {
            let step = super::super::services::unit_step(kind, 50_000);
            let _ = sim.build(kind, step * 10.0); // cobertura pública de sobra
        }
        let before = sim.economy.population;
        for _ in 0..12 {
            sim.tick_month();
        }
        assert!(sim.economy.population > before);
        assert!(sim.population.net_rate > 0.0);
    }

    #[test]
    fn impuestos_altos_y_paro_expulsan_poblacion() {
        let mut sim = Simulation::default();
        sim.economy.corporate_tax = 80.0;
        sim.economy.income_tax_low = 80.0;
        sim.economy.income_tax_mid = 80.0;
        sim.economy.income_tax_high = 80.0;
        sim.economy.unemployment_rate = 30.0;
        let before = sim.economy.population;
        for _ in 0..12 {
            sim.tick_month();
        }
        assert!(sim.economy.population < before);
        assert!(sim.population.net_rate < 0.0);
    }

    #[test]
    fn la_tasa_de_migracion_esta_acotada() {
        let mut sim = Simulation::default();
        sim.economy.corporate_tax = 100.0;
        sim.economy.income_tax_low = 100.0;
        sim.economy.income_tax_mid = 100.0;
        sim.economy.income_tax_high = 100.0;
        sim.economy.unemployment_rate = 40.0;
        for kind in ServiceKind::ALL {
            let _ = sim.demolish(kind, f32::MAX); // sin servicios públicos
        }
        let outlook = sim
            .population
            .outlook(&sim.demographics, &sim.services, &sim.economy, sim.crime.pressure());
        assert!(outlook.net_rate >= PopulationState::MIN_RATE);
        assert!(outlook.net_rate <= PopulationState::MAX_RATE);
    }
}
