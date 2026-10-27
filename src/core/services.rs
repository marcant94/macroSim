//! Servicios (sanidad, educación, seguridad…): registro genérico y escalable.
//!
//! **Patrón de extensión:** añadir un servicio nuevo son solo 2 pasos:
//! 1. Añadir una variante a `ServiceKind`.
//! 2. Devolver su `ServiceSpec` en `ServiceKind::spec()`.
//!
//! Nada más: cobertura, gasto mensual, crecimiento del sector privado y consumo
//! de suelo se calculan en bucles genéricos sobre el registro. Ninguna otra
//! parte del código necesita conocer los servicios uno a uno.

use serde::{Deserialize, Serialize};

use super::economy::EconomyState;

/// Identificador de servicio. El orden de las variantes debe coincidir con `ALL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceKind {
    Health,
    Education,
    Security,
    Fire,
    Prisons,
    Waste,
    Roads,
    Transport,
}

impl ServiceKind {
    /// Registro completo de servicios. Añadir aquí toda variante nueva.
    pub const ALL: [ServiceKind; 8] = [
        ServiceKind::Health,
        ServiceKind::Education,
        ServiceKind::Security,
        ServiceKind::Fire,
        ServiceKind::Prisons,
        ServiceKind::Waste,
        ServiceKind::Roads,
        ServiceKind::Transport,
    ];

    /// Índice del estado dentro de `ServicesState::services` (mismo orden que `ALL`).
    pub fn index(self) -> usize {
        match self {
            ServiceKind::Health => 0,
            ServiceKind::Education => 1,
            ServiceKind::Security => 2,
            ServiceKind::Fire => 3,
            ServiceKind::Prisons => 4,
            ServiceKind::Waste => 5,
            ServiceKind::Roads => 6,
            ServiceKind::Transport => 7,
        }
    }

    /// Configuración (balance) del servicio. Datos, no lógica.
    ///
    /// `capital` ronda 40 meses de `upkeep`: construir es una inversión que se
    /// amortiza en algo más de tres años de juego.
    pub fn spec(self) -> ServiceSpec {
        match self {
            //                 demanda   upkeep  capital   suelo    drive
            //                 /1000 hab /unidad /unidad   (ha)     (privado)
            ServiceKind::Health => ServiceSpec::new(4.0, 600.0, 24_000.0, 1.5, 0.40),
            ServiceKind::Education => ServiceSpec::new(20.0, 60.0, 2_400.0, 0.4, 0.35),
            ServiceKind::Security => ServiceSpec::new(0.5, 900.0, 36_000.0, 0.5, 0.05),
            ServiceKind::Fire => ServiceSpec::new(0.2, 1_500.0, 60_000.0, 0.8, 0.05),
            ServiceKind::Prisons => ServiceSpec::new(0.8, 700.0, 28_000.0, 2.0, 0.02),
            ServiceKind::Waste => ServiceSpec::new(0.5, 600.0, 24_000.0, 3.0, 0.20),
            ServiceKind::Roads => ServiceSpec::new(2.0, 100.0, 4_000.0, 0.2, 0.30),
            ServiceKind::Transport => ServiceSpec::new(1.0, 120.0, 4_800.0, 0.5, 0.25),
        }
    }

    /// Clave i18n con el nombre del servicio (la UI no conoce textos literales).
    pub fn name_key(self) -> &'static str {
        match self {
            ServiceKind::Health => "services.kind.health",
            ServiceKind::Education => "services.kind.education",
            ServiceKind::Security => "services.kind.security",
            ServiceKind::Fire => "services.kind.fire",
            ServiceKind::Prisons => "services.kind.prisons",
            ServiceKind::Waste => "services.kind.waste",
            ServiceKind::Roads => "services.kind.roads",
            ServiceKind::Transport => "services.kind.transport",
        }
    }
}

/// Fracción del capital que se recupera al demoler (venta de instalaciones).
pub const DEMOLISH_REFUND: f32 = 0.5;

/// Unidades que se construyen o demuelen por clic: ~5 % de la demanda actual,
/// de modo que el paso crece con la población y nunca es 0.
pub fn unit_step(kind: ServiceKind, population: u32) -> f32 {
    (ServicesState::demand(kind, population) * 0.05).max(1.0).round()
}

/// Configuración estática de un servicio (unidades arbitrarias por servicio:
/// una "unidad" es lo que defina el servicio: cama, aula, comisaría, km de red…).
#[derive(Debug, Clone, Copy)]
pub struct ServiceSpec {
    /// Unidades demandadas por cada 1.000 habitantes.
    pub demand_per_1000: f32,
    /// Gasto mensual recurrente (salarios + mantenimiento) por unidad pública.
    /// El sector privado se autofinancia: no genera gasto público.
    pub upkeep_per_unit: f32,
    /// Coste único de construir una unidad pública (inversión de capital).
    pub capital_cost_per_unit: f32,
    /// Hectáreas de suelo que ocupa cada unidad.
    pub land_per_unit: f32,
    /// Propensión del sector privado a cubrir demanda insatisfecha (0..=1).
    /// Bajo para servicios que el mercado apenas presta (cárceles, bomberos);
    /// alto para sanidad/educación/carreteras de peaje.
    pub private_drive: f32,
}

impl ServiceSpec {
    const fn new(demand: f32, upkeep: f32, capital: f32, land: f32, drive: f32) -> Self {
        Self {
            demand_per_1000: demand,
            upkeep_per_unit: upkeep,
            capital_cost_per_unit: capital,
            land_per_unit: land,
            private_drive: drive,
        }
    }
}

/// Estado mutable de un servicio: capacidad pública + privada.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceState {
    /// Capacidad pública construida (la paga el estado).
    pub public_units: f32,
    /// Capacidad privada (crece sola según el clima de inversión).
    pub private_units: f32,
    /// Última cobertura total calculada (0..=1), para UI e indicadores.
    pub coverage: f32,
}

/// Clima de inversión privada (0..=1): impuestos bajos y empleo sano atraen capital.
/// La fuga de capital del tramo alto también lo penaliza: el dinero que se va
/// no invierte aquí.
pub fn investment_climate(economy: &EconomyState) -> f32 {
    let tax_ok = 1.0 - economy.corporate_tax / 100.0; // 0..1
    let jobs_ok = 1.0 - economy.unemployment_rate / 100.0; // 0..1
    let base = tax_ok * 0.6 + jobs_ok * 0.4;
    (base * (1.0 - 0.6 * economy.capital_flight())).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesState {
    pub services: [ServiceState; ServiceKind::ALL.len()],
}

impl Default for ServicesState {
    /// Partida inicial: la ciudad ya cubre ~70% de la demanda de 50.000 hab
    /// con servicios públicos (el resto lo traerá el sector privado).
    fn default() -> Self {
        let services = std::array::from_fn(|i| {
            let kind = ServiceKind::ALL[i];
            let demand = Self::demand(kind, 50_000);
            ServiceState {
                public_units: demand * 0.7,
                private_units: demand * 0.1,
                coverage: 0.0,
            }
        });
        Self { services }
    }
}

impl ServicesState {
    pub fn get(&self, kind: ServiceKind) -> &ServiceState {
        &self.services[kind.index()]
    }

    pub fn get_mut(&mut self, kind: ServiceKind) -> &mut ServiceState {
        &mut self.services[kind.index()]
    }

    /// Demanda total del servicio para la población dada.
    pub fn demand(kind: ServiceKind, population: u32) -> f32 {
        kind.spec().demand_per_1000 * (population as f32 / 1000.0)
    }

    /// Cobertura pública (0..=1): lo que atiende el estado.
    pub fn public_coverage(&self, kind: ServiceKind, population: u32) -> f32 {
        let st = self.get(kind);
        let demand = Self::demand(kind, population).max(1e-6);
        (st.public_units / demand).min(1.0)
    }

    /// Cobertura privada (0..=1): lo que atiende el sector privado.
    pub fn private_coverage(&self, kind: ServiceKind, population: u32) -> f32 {
        let st = self.get(kind);
        let demand = Self::demand(kind, population).max(1e-6);
        (st.private_units / demand).min(1.0)
    }

    /// Gasto mensual total en servicios (solo lo público).
    pub fn monthly_upkeep(&self) -> f64 {
        self.services
            .iter()
            .zip(ServiceKind::ALL)
            .map(|(st, kind)| (st.public_units * kind.spec().upkeep_per_unit) as f64)
            .sum()
    }

    /// Avanza un mes todos los servicios:
    /// - el sector privado crece hacia la demanda insatisfecha según el clima,
    /// - se recalcula la cobertura,
    /// - devuelve el gasto mensual público total (lo añade la economía).
    pub fn tick(&mut self, population: u32, climate: f32) -> f64 {
        let mut upkeep = 0.0_f64;
        for (i, kind) in ServiceKind::ALL.iter().enumerate() {
            let spec = kind.spec();
            let demand = Self::demand(*kind, population);
            let st = &mut self.services[i];

            let uncovered = (demand - st.public_units - st.private_units).max(0.0);
            // Crecimiento privado mensual modesto y acotado a la demanda.
            st.private_units += uncovered * spec.private_drive * climate * 0.05;
            st.private_units = st.private_units.min(demand);

            upkeep += (st.public_units * spec.upkeep_per_unit) as f64;
        }
        self.recompute_coverage(population);
        upkeep
    }

    /// Recalcula `coverage` desde la capacidad actual sin avanzar el mes.
    /// La UI lo usa para reflejar al instante lo construido/demolido (también en pausa).
    pub fn recompute_coverage(&mut self, population: u32) {
        for (st, kind) in self.services.iter_mut().zip(ServiceKind::ALL) {
            let demand = Self::demand(kind, population).max(1e-6);
            st.coverage = ((st.public_units + st.private_units) / demand).min(1.0);
        }
    }
}
