//! Suelo de la ciudad-estado: límite de hectáreas y consumo por construcción.
//!
//! Como en los juegos de ciudad, no se puede construir infinitamente: cada
//! unidad de servicio, la red eléctrica y (en el futuro) viviendas e industria
//! consumen suelo. La expansión territorial (conquista/compra) es una
//! decisión de Fase 3/4 y solo aumentará `total_hectareas`.

use serde::{Deserialize, Serialize};

use super::energy::EnergyState;
use super::services::{ServiceKind, ServicesState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandState {
    /// Hectáreas totales gobernable. Solo crece con expansión territorial.
    pub total_hectareas: f32,
}

impl Default for LandState {
    fn default() -> Self {
        // Ciudad-estado inicial: 4.000 ha (40 km²). La partida arranca usando
        // ~19 %, para que el suelo sea una restricción de medio plazo y no un
        // muro desde el primer año; agotarlo obliga a reciclar, densificar o
        // expandirse (Fase 3/4: compra/conquista de territorio).
        Self {
            total_hectareas: 4_000.0,
        }
    }
}

impl LandState {
    /// Suelo ocupado por las unidades de todos los servicios (públicos y privados:
    /// el privado también construye sobre el territorio).
    pub fn used_by_services(&self, services: &ServicesState) -> f32 {
        services
            .services
            .iter()
            .zip(ServiceKind::ALL)
            .map(|(st, kind)| (st.public_units + st.private_units) * kind.spec().land_per_unit)
            .sum()
    }

    /// Suelo ocupado por las centrales eléctricas (cada fuente tiene su
    /// consumo por MW: una hidráulica ocupa mucho más que un ciclo combinado).
    pub fn used_by_energy(&self, energy: &EnergyState) -> f32 {
        energy.land_hectareas()
    }

    /// Suelo total ocupado (servicios + energía; vivienda/industria: futuro).
    pub fn used(&self, services: &ServicesState, energy: &EnergyState) -> f32 {
        self.used_by_services(services) + self.used_by_energy(energy)
    }

    /// Suelo libre para construir.
    pub fn free(&self, services: &ServicesState, energy: &EnergyState) -> f32 {
        (self.total_hectareas - self.used(services, energy)).max(0.0)
    }

    /// Ocupación en % (para indicadores y avisos).
    pub fn usage_percent(&self, services: &ServicesState, energy: &EnergyState) -> f32 {
        if self.total_hectareas > 0.0 {
            (self.used(services, energy) / self.total_hectareas * 100.0).min(100.0)
        } else {
            100.0
        }
    }

    /// ¿Hay suelo libre suficiente para construir `hectareas` más?
    pub fn can_build(&self, services: &ServicesState, energy: &EnergyState, hectareas: f32) -> bool {
        self.free(services, energy) >= hectareas
    }
}
