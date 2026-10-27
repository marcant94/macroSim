//! Delincuencia: indicador **emergente**, no un slider del jugador.
//!
//! La criminalidad es el termómetro de "si lo estás haciendo mal": sube con el
//! paro, la exclusión social (clase baja descontenta) y la quiebra, y baja con
//! policía, educación y cárceles. A su vez castiga a la ciudad: resta
//! satisfacción a todas las clases, espanta la inversión privada y empuja a la
//! gente a irse. Es un bucle de realimentación, así que el ajuste mensual es
//! parcial (el crimen no cambia de un mes para otro).

use serde::{Deserialize, Serialize};

use super::demographics::{DemographicsState, SocialClass};
use super::economy::EconomyState;
use super::services::{ServiceKind, ServicesState};

/// Delitos por 1.000 habitantes y mes (todo lo que se denuncia).
pub type CrimeRate = f32;

/// Componente fija de la tasa: la criminalidad de fondo de cualquier ciudad.
pub const BASELINE_RATE: CrimeRate = 3.0;
/// Tasa que la ciudad tolera sin que la inseguridad le pase factura. La partida
/// inicial arranca por debajo, así que solo empeorar tiene consecuencias.
pub const TOLERABLE_RATE: CrimeRate = 4.0;
/// A partir de esta tasa la presión social es máxima (0..1).
pub const HIGH_RATE: CrimeRate = 15.0;
/// Tasa de arranque: el equilibrio de una ciudad con servicios cubiertos.
pub const INITIAL_RATE: CrimeRate = 2.5;
/// Fracción de la brecha hacia el objetivo que se recorre cada mes.
const ADJUSTMENT: f32 = 0.25;

/// Aportación de cada factor a la gravedad (delitos/1000 hab antes de mitigar).
const UNEMPLOYMENT_SLOPE: f32 = 45.0;
const EXCLUSION_SLOPE: f32 = 8.0;
const BANKRUPTCY_ADD: f32 = 6.0;
/// Cuánto recorta cada servicio la gravedad (con cobertura total).
const SECURITY_MITIGATION: f32 = 0.35;
const EDUCATION_MITIGATION: f32 = 0.15;
const PRISONS_MITIGATION: f32 = 0.10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrimeState {
    /// Delitos por 1.000 habitantes y mes.
    pub rate: CrimeRate,
}

impl Default for CrimeState {
    fn default() -> Self {
        Self { rate: INITIAL_RATE }
    }
}

impl CrimeState {
    /// Presión social de la delincuencia (0..=1). Es lo que consumen la
    /// satisfacción, la inversión privada y la migración. Por debajo de la tasa
    /// tolerable es 0: una ciudad normal no sufre por su criminalidad de fondo.
    pub fn pressure(&self) -> f32 {
        ((self.rate - TOLERABLE_RATE) / (HIGH_RATE - TOLERABLE_RATE)).clamp(0.0, 1.0)
    }

    /// Tasa hacia la que tiende la ciudad este mes, dados servicios, economía
    /// y reparto de clases.
    pub fn target(
        &self,
        services: &ServicesState,
        economy: &EconomyState,
        demographics: &DemographicsState,
    ) -> CrimeRate {
        let pop = economy.population;
        let cobertura = |kind: ServiceKind| {
            let st = services.get(kind);
            let demand = ServicesState::demand(kind, pop).max(1e-6);
            ((st.public_units + st.private_units) / demand).min(1.0)
        };

        // El paro por encima del estructural (6 %) es el principal motor.
        let paro = ((economy.unemployment_rate - 6.0) / 100.0).max(0.0);
        // Exclusión social: la clase baja descontenta alimenta la delincuencia.
        let baja =
            demographics.satisfaction(SocialClass::Low, services, pop, economy, self.pressure());
        let exclusion = (1.0 - baja).clamp(0.0, 1.0);
        let quiebra = if economy.is_bankrupt(demographics) { 1.0 } else { 0.0 };

        let gravedad = BASELINE_RATE
            + paro * UNEMPLOYMENT_SLOPE
            + exclusion * EXCLUSION_SLOPE
            + quiebra * BANKRUPTCY_ADD;
        let mitigacion = (1.0
            - SECURITY_MITIGATION * cobertura(ServiceKind::Security)
            - EDUCATION_MITIGATION * cobertura(ServiceKind::Education)
            - PRISONS_MITIGATION * cobertura(ServiceKind::Prisons))
        .clamp(0.0, 1.0);
        (gravedad * mitigacion).max(0.0)
    }

    /// Avanza la delincuencia un mes hacia su objetivo. Devuelve la tasa nueva.
    pub fn tick(
        &mut self,
        services: &ServicesState,
        economy: &EconomyState,
        demographics: &DemographicsState,
    ) -> CrimeRate {
        let target = self.target(services, economy, demographics);
        self.rate += (target - self.rate) * ADJUSTMENT;
        self.rate = self.rate.clamp(0.0, HIGH_RATE * 2.0);
        self.rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Simulation;

    #[test]
    fn la_partida_inicial_es_una_ciudad_segura() {
        let mut sim = Simulation::default();
        for _ in 0..24 {
            sim.tick_month();
        }
        assert!(sim.crime.rate < 5.0, "tasa inicial {}", sim.crime.rate);
        assert!(sim.crime.pressure() < 0.35);
    }

    #[test]
    fn menos_policia_sube_la_delincuencia() {
        let mut seguro = Simulation::default();
        let mut inseguro = Simulation::default();
        inseguro.demolish(ServiceKind::Security, f32::MAX).ok();
        for _ in 0..24 {
            seguro.tick_month();
            inseguro.tick_month();
        }
        assert!(
            inseguro.crime.rate > seguro.crime.rate,
            "sin policía la tasa debe subir ({} > {})",
            inseguro.crime.rate,
            seguro.crime.rate
        );
    }

    #[test]
    fn el_paro_dispara_la_delincuencia() {
        let mut sim = Simulation::default();
        sim.economy.unemployment_rate = 30.0;
        let antes = sim.crime.rate;
        for _ in 0..24 {
            sim.tick_month();
            sim.economy.unemployment_rate = 30.0; // el paro se reajusta en el tick
        }
        assert!(sim.crime.rate > antes * 2.0);
    }

    #[test]
    fn la_delincuencia_esta_acotada() {
        let mut sim = Simulation::default();
        sim.economy.unemployment_rate = 40.0;
        for kind in ServiceKind::ALL {
            sim.demolish(kind, f32::MAX).ok();
        }
        for _ in 0..120 {
            sim.tick_month();
            sim.economy.unemployment_rate = 40.0;
        }
        assert!(sim.crime.rate <= HIGH_RATE * 2.0);
        assert!(sim.crime.pressure() <= 1.0);
    }
}
