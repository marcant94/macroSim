//! Estado y lógica de la red eléctrica: capacidad, demanda y precio.

use serde::{Deserialize, Serialize};

use super::economy::EconomyState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyState {
    pub capacity_mw: f32,
    pub demand_mw: f32,
    pub price_per_mwh: f32,
}

impl Default for EnergyState {
    fn default() -> Self {
        Self {
            capacity_mw: 500.0,
            demand_mw: 420.0,
            price_per_mwh: 65.0,
        }
    }
}

impl EnergyState {
    /// ¿La demanda supera la capacidad instalada?
    pub fn has_deficit(&self) -> bool {
        self.demand_mw > self.capacity_mw
    }

    /// Cobertura de la demanda en % (0 si no hay capacidad).
    pub fn coverage(&self) -> f32 {
        if self.capacity_mw > 0.0 {
            self.demand_mw / self.capacity_mw * 100.0
        } else {
            0.0
        }
    }

    /// Avanza la energía un mes: la demanda crece con la población y la
    /// actividad económica (más actividad = más consumo industrial/doméstico).
    pub fn tick(&mut self, economy: &EconomyState) {
        let activity = 1.0 - (economy.unemployment_rate / 100.0);
        // Crecimiento mensual modesto: base + factor de actividad.
        let growth_rate = 0.001 + 0.002 * activity;
        self.demand_mw *= 1.0 + growth_rate;
    }
}
