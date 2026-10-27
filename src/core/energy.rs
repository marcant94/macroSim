//! Energía: mix de fuentes (como en SimCity / Cities: Skylines).
//!
//! Cada fuente se instala por MW y tiene cuatro costes/beneficios declarados
//! como datos en `EnergySource::spec()`:
//!
//! - **capital**: pago único al construir;
//! - **mantenimiento**: gasto fijo mensual (combustible, salarios, revisión);
//! - **factor de carga**: fracción del tiempo que produce de media (una eólica
//!   instalada de 100 MW entrega ~35 MW medios); el jugador ve dos números, los
//!   MW instalados y los MW efectivos;
//! - **emisiones**: kg de CO₂ por MWh, que se pagan como coste sanitario.
//!
//! De ahí sale la comparación que pide el juego: **MW efectivos por cada 1.000 ₿
//! de mantenimiento** (eficiencia de explotación) frente al **capital por MW**.
//! Los fósiles son baratos de construir y caros de explotar; las renovables al
//! revés, y además comen suelo.

use serde::{Deserialize, Serialize};

use super::economy::EconomyState;

/// Demanda eléctrica por habitante y mes en MW medios (420 MW con 50.000 hab).
pub const MW_PER_CAPITA: f32 = 0.0084;
/// Horas de un mes medio (para pasar MW a MWh).
pub const HOURS_PER_MONTH: f32 = 730.0;
/// Coste sanitario de una tonelada de CO₂ emitida (₿).
pub const EMISSIONS_COST_PER_TONNE: f64 = 0.15;

/// Variación de la producción de una fuente en un mes concreto.
///
/// El `capacity_factor` de la ficha es la **media anual**; esta función dice
/// cuánto rinde ese mes respecto a esa media:
/// - **Térmicas** (carbón, petróleo, gas, nuclear): 1.0. Son gestionables: si
///   hay demanda, producen a plena capacidad.
/// - **Renovables**: estacionalidad (sol en verano, viento en invierno,
///   hidráulica en primavera) más una componente de clima.
///
/// La media sobre los 12 meses del año es ≈ 1.0, así que las comparaciones de
/// la tabla (MW por 1.000 ₿, capital por MW efectivo) siguen siendo válidas.
pub fn monthly_factor(source: EnergySource, month: u32, year: u32) -> f32 {
    use std::f32::consts::TAU;
    // Las gestionables producen lo que se les pide: ni estación ni clima.
    let estacional = match source {
        EnergySource::Coal
        | EnergySource::Oil
        | EnergySource::Gas
        | EnergySource::Nuclear
        | EnergySource::Geothermal => return 1.0,
        // Pico en julio.
        EnergySource::Solar => 1.0 + 0.30 * (((month as f32) - 7.0) / 12.0 * TAU).cos(),
        // Pico en enero.
        EnergySource::Wind => 1.0 + 0.25 * (((month as f32) - 1.0) / 12.0 * TAU).cos(),
        // Pico en abril (lluvias y deshielo).
        EnergySource::Hydro => 1.0 + 0.25 * (((month as f32) - 4.0) / 12.0 * TAU).cos(),
    };
    let clima = 1.0 + 0.15 * ruido_climatico(year, month, source);
    (estacional * clima).clamp(0.30, 1.40)
}

/// Ruido determinista en [-1, 1] a partir de año, mes y fuente (FNV-1a).
/// Determinista a propósito: la partida es reproducible y testeable.
fn ruido_climatico(year: u32, month: u32, source: EnergySource) -> f32 {
    let mut h: u32 = 2_166_136_261;
    for byte in [year, month, source.index() as u32] {
        h ^= byte;
        h = h.wrapping_mul(16_777_619);
    }
    let unitario = (h % 1_000) as f32 / 1_000.0;
    unitario * 2.0 - 1.0
}

/// Fuente de generación. El orden debe coincidir con `ALL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergySource {
    Coal,
    Oil,
    Gas,
    Nuclear,
    Geothermal,
    Wind,
    Solar,
    Hydro,
}

impl EnergySource {
    pub const ALL: [EnergySource; 8] = [
        EnergySource::Coal,
        EnergySource::Oil,
        EnergySource::Gas,
        EnergySource::Nuclear,
        EnergySource::Geothermal,
        EnergySource::Wind,
        EnergySource::Solar,
        EnergySource::Hydro,
    ];

    /// Índice en `EnergyState::sources` (mismo orden que `ALL`).
    pub fn index(self) -> usize {
        match self {
            EnergySource::Coal => 0,
            EnergySource::Oil => 1,
            EnergySource::Gas => 2,
            EnergySource::Nuclear => 3,
            EnergySource::Geothermal => 4,
            EnergySource::Wind => 5,
            EnergySource::Solar => 6,
            EnergySource::Hydro => 7,
        }
    }

    /// Clave i18n del nombre de la fuente.
    pub fn name_key(self) -> &'static str {
        match self {
            EnergySource::Coal => "energy.source.coal",
            EnergySource::Oil => "energy.source.oil",
            EnergySource::Gas => "energy.source.gas",
            EnergySource::Nuclear => "energy.source.nuclear",
            EnergySource::Geothermal => "energy.source.geothermal",
            EnergySource::Wind => "energy.source.wind",
            EnergySource::Solar => "energy.source.solar",
            EnergySource::Hydro => "energy.source.hydro",
        }
    }

    /// Balance de la fuente. Datos, no lógica. Cada fuente tiene un tamaño de
    /// instalación propio (una nuclear no es del mismo tamaño que un parque
    /// solar), así que no se compra "MW sueltos" sino plantas.
    pub fn spec(self) -> SourceSpec {
        match self {
            //                    planta  capital   upkeep  factor  suelo  kg CO2
            //                    MW      /MW       /MW·mes (0..1)  ha/MW  /MWh
            EnergySource::Coal => SourceSpec::new(300.0, 8_000.0, 60.0, 0.85, 0.05, 1_000.0),
            EnergySource::Oil => SourceSpec::new(150.0, 5_500.0, 100.0, 0.90, 0.04, 750.0),
            EnergySource::Gas => SourceSpec::new(250.0, 6_000.0, 90.0, 0.90, 0.03, 450.0),
            EnergySource::Nuclear => SourceSpec::new(500.0, 25_000.0, 120.0, 0.95, 0.05, 12.0),
            EnergySource::Geothermal => SourceSpec::new(120.0, 18_000.0, 90.0, 0.90, 0.10, 15.0),
            EnergySource::Wind => SourceSpec::new(100.0, 12_000.0, 40.0, 0.35, 0.60, 0.0),
            EnergySource::Solar => SourceSpec::new(80.0, 8_500.0, 35.0, 0.25, 0.90, 0.0),
            EnergySource::Hydro => SourceSpec::new(200.0, 20_000.0, 50.0, 0.50, 1.20, 0.0),
        }
    }
}

/// Configuración estática de una fuente de energía.
#[derive(Debug, Clone, Copy)]
pub struct SourceSpec {
    /// Potencia nominal de una instalación de esta fuente (MW).
    pub plant_mw: f32,
    /// Coste de construir un MW (pago único).
    pub capital_cost_per_mw: f32,
    /// Mantenimiento mensual por MW instalado.
    pub upkeep_per_mw: f32,
    /// Producción media como fracción de los MW instalados.
    pub capacity_factor: f32,
    /// Hectáreas que ocupa cada MW instalado.
    pub land_per_mw: f32,
    /// Emisiones en kg de CO₂ por MWh producido.
    pub emissions_kg_per_mwh: f32,
}

impl SourceSpec {
    const fn new(
        plant_mw: f32,
        capital: f32,
        upkeep: f32,
        factor: f32,
        land: f32,
        emissions: f32,
    ) -> Self {
        Self {
            plant_mw,
            capital_cost_per_mw: capital,
            upkeep_per_mw: upkeep,
            capacity_factor: factor,
            land_per_mw: land,
            emissions_kg_per_mwh: emissions,
        }
    }

    /// Coste de construir una instalación completa.
    pub fn capital_per_plant(&self) -> f32 {
        self.plant_mw * self.capital_cost_per_mw
    }

    /// Mantenimiento mensual de una instalación completa.
    pub fn upkeep_per_plant(&self) -> f32 {
        self.plant_mw * self.upkeep_per_mw
    }

    /// MW efectivos que entrega una instalación completa.
    pub fn effective_mw_per_plant(&self) -> f32 {
        self.plant_mw * self.capacity_factor
    }

    /// Hectáreas que ocupa una instalación completa.
    pub fn land_per_plant(&self) -> f32 {
        self.plant_mw * self.land_per_mw
    }

    /// Coste mensual de explotación de cada MW **efectivo**: mantenimiento más
    /// el coste sanitario de sus emisiones. Es lo que hay que comparar: una
    /// central barata de mantener pero sucia sale caro por otro lado.
    pub fn running_cost_per_effective_mw(&self) -> f32 {
        let effective = self.capacity_factor.max(1e-6);
        let upkeep = self.upkeep_per_mw / effective;
        let emissions_per_effective_mw_t =
            HOURS_PER_MONTH * self.emissions_kg_per_mwh / 1_000.0;
        let emissions = emissions_per_effective_mw_t * EMISSIONS_COST_PER_TONNE as f32;
        upkeep + emissions
    }

    /// MW efectivos que salen por cada 1.000 ₿/mes de explotación (mantener +
    /// emisiones). Más alto = más eficiente. Es el número que compara la tabla.
    pub fn effective_mw_per_1000(&self) -> f32 {
        let cost = self.running_cost_per_effective_mw();
        if cost > 0.0 {
            1_000.0 / cost
        } else {
            0.0
        }
    }

    /// Capital necesario para poner en marcha 1 MW efectivo.
    pub fn capital_per_effective_mw(&self) -> f32 {
        if self.capacity_factor > 0.0 {
            self.capital_cost_per_mw / self.capacity_factor
        } else {
            f32::INFINITY
        }
    }
}

/// Instalaciones de una fuente (plantas enteras, no MW sueltos).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceState {
    #[serde(default)]
    pub plants: u32,
}

impl SourceState {
    pub fn new(plants: u32) -> Self {
        Self { plants }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyState {
    pub sources: [SourceState; EnergySource::ALL.len()],
    /// Demanda del mes (sigue a la población y a la actividad).
    pub demand_mw: f32,
    /// Mes y año con los que se calcula la producción (los pone el reloj).
    /// `default` por compatibilidad con partidas guardadas.
    #[serde(default)]
    pub month: u32,
    #[serde(default)]
    pub year: u32,
}

impl Default for EnergyState {
    /// Mix de partida: una central de carbón y otra de gas, baratas de
    /// construir pero sucias. 550 MW nominales / 480 efectivos para 420 de
    /// demanda: la ciudad arranca con un poco de margen.
    fn default() -> Self {
        let mut sources: [SourceState; EnergySource::ALL.len()] = Default::default();
        sources[EnergySource::Coal.index()] = SourceState::new(1);
        sources[EnergySource::Gas.index()] = SourceState::new(1);
        Self {
            sources,
            demand_mw: 420.0,
            month: 1,
            year: 2_026,
        }
    }
}

impl EnergyState {
    pub fn get(&self, source: EnergySource) -> &SourceState {
        &self.sources[source.index()]
    }

    pub fn get_mut(&mut self, source: EnergySource) -> &mut SourceState {
        &mut self.sources[source.index()]
    }

    /// Instalaciones de una fuente.
    pub fn plant_count(&self, source: EnergySource) -> u32 {
        self.get(source).plants
    }

    /// MW nominales de una fuente (instalaciones × tamaño).
    pub fn installed_mw_of(&self, source: EnergySource) -> f32 {
        self.plant_count(source) as f32 * source.spec().plant_mw
    }

    /// MW efectivos de una fuente (instalaciones × tamaño × factor de carga).
    pub fn effective_mw_of(&self, source: EnergySource) -> f32 {
        self.plant_count(source) as f32 * source.spec().effective_mw_per_plant()
    }

    /// MW nominales instalados en total.
    pub fn installed_mw(&self) -> f32 {
        EnergySource::ALL
            .iter()
            .map(|src| self.installed_mw_of(*src))
            .sum()
    }

    /// MW efectivos **de media anual** (lo que se usa para comparar fuentes).
    pub fn effective_capacity_mw(&self) -> f32 {
        EnergySource::ALL
            .iter()
            .map(|src| self.effective_mw_of(*src))
            .sum()
    }

    /// Fija el mes con el que se calcula la producción (lo hace el reloj).
    pub fn set_date(&mut self, month: u32, year: u32) {
        self.month = month.max(1);
        self.year = year.max(1);
    }

    /// Producción real del mes de una fuente (MW). Se calcula al vuelo a partir
    /// de las plantas instaladas y de la estación, así que nunca queda obsoleta.
    pub fn production_mw_of(&self, source: EnergySource) -> f32 {
        let spec = source.spec();
        self.plant_count(source) as f32
            * spec.plant_mw
            * spec.capacity_factor
            * monthly_factor(source, self.month.max(1), self.year.max(1))
    }

    /// Producción real del mes de todo el mix (MW).
    pub fn production_mw(&self) -> f32 {
        EnergySource::ALL
            .iter()
            .map(|src| self.production_mw_of(*src))
            .sum()
    }

    /// ¿La demanda supera lo que se produce **este mes**?
    pub fn has_deficit(&self) -> bool {
        self.demand_mw > self.production_mw()
    }

    /// Demanda cubierta (%). 100 = toda la ciudad tiene luz.
    ///
    /// Ojo: antes esto devolvía demanda/capacidad, que es la **utilización** de
    /// la red, no la cobertura.
    pub fn coverage(&self) -> f32 {
        if self.demand_mw <= 0.0 {
            return 100.0;
        }
        (self.production_mw() / self.demand_mw * 100.0).min(100.0)
    }

    /// Producción mensual en MWh (la real del mes, no la media).
    pub fn monthly_production_mwh(&self) -> f64 {
        self.production_mw() as f64 * HOURS_PER_MONTH as f64
    }

    /// Mantenimiento mensual de todas las centrales.
    pub fn monthly_upkeep(&self) -> f64 {
        EnergySource::ALL
            .iter()
            .map(|src| {
                (self.plant_count(*src) as f32 * src.spec().upkeep_per_plant()) as f64
            })
            .sum()
    }

    /// Mantenimiento mensual de las instalaciones de una fuente.
    pub fn monthly_upkeep_of(&self, source: EnergySource) -> f64 {
        (self.plant_count(source) as f32 * source.spec().upkeep_per_plant()) as f64
    }

    /// Emisiones mensuales en toneladas de CO₂.
    pub fn monthly_emissions_t(&self) -> f64 {
        EnergySource::ALL
            .iter()
            .map(|src| self.monthly_emissions_t_of(*src))
            .sum()
    }

    /// Coste sanitario de las emisiones del mes.
    pub fn emissions_cost(&self) -> f64 {
        self.monthly_emissions_t() * EMISSIONS_COST_PER_TONNE
    }

    /// Coste total del mix por MWh (mantenimiento + emisiones).
    pub fn cost_per_mwh(&self) -> f32 {
        let mwh = self.monthly_production_mwh();
        if mwh <= 0.0 {
            0.0
        } else {
            ((self.monthly_upkeep() + self.emissions_cost()) / mwh) as f32
        }
    }

    /// Emisiones mensuales de una fuente (toneladas de CO₂).
    pub fn monthly_emissions_t_of(&self, source: EnergySource) -> f64 {
        let spec = source.spec();
        let mwh = (self.production_mw_of(source) * HOURS_PER_MONTH) as f64;
        mwh * spec.emissions_kg_per_mwh as f64 / 1_000.0
    }

    /// Créditos que cuesta cada MW producido este mes con una fuente:
    /// mantenimiento más coste de sus emisiones, dividido por los MW reales.
    pub fn cost_per_mw_of(&self, source: EnergySource) -> f64 {
        let mw = self.production_mw_of(source);
        if mw <= 0.0 {
            0.0
        } else {
            (self.monthly_upkeep_of(source)
                + self.monthly_emissions_t_of(source) * EMISSIONS_COST_PER_TONNE)
                / mw as f64
        }
    }

    /// Hectáreas ocupadas por las instalaciones de una fuente.
    pub fn land_of(&self, source: EnergySource) -> f32 {
        self.plant_count(source) as f32 * source.spec().land_per_plant()
    }

    /// Total de instalaciones de todas las fuentes.
    pub fn total_plants(&self) -> u32 {
        EnergySource::ALL.iter().map(|s| self.plant_count(*s)).sum()
    }

    /// Hectáreas ocupadas por las centrales.
    pub fn land_hectareas(&self) -> f32 {
        EnergySource::ALL
            .iter()
            .map(|src| self.land_of(*src))
            .sum()
    }

    /// Avanza la energía un mes: la demanda sigue a la población y a la
    /// actividad económica. Si crece la ciudad, hay que instalar más MW.
    pub fn tick(&mut self, economy: &EconomyState) {
        let actividad = 1.0 + 0.5 * (0.085 - economy.unemployment_rate / 100.0);
        let target = economy.population as f32 * MW_PER_CAPITA * actividad.clamp(0.7, 1.3);
        // La demanda no salta de golpe: se ajusta hacia su objetivo.
        self.demand_mw += (target - self.demand_mw) * 0.25;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ActionError, Simulation};

    #[test]
    fn la_partida_inicial_cubre_la_demanda() {
        let energy = EnergyState::default();
        assert!(!energy.has_deficit());
        assert!(energy.coverage() > 95.0);
    }

    #[test]
    fn la_demanda_sigue_a_la_poblacion() {
        let mut normal = Simulation::default();
        let mut grande = Simulation::default();
        for _ in 0..24 {
            normal.tick_month();
            grande.economy.population = 120_000;
            grande.tick_month();
        }
        assert!(
            grande.energy.demand_mw > normal.energy.demand_mw * 1.5,
            "más población debe exigir más energía ({} vs {})",
            grande.energy.demand_mw,
            normal.energy.demand_mw
        );
    }

    #[test]
    fn construir_capacidad_cobra_capital_y_cubre_el_deficit() {
        let mut sim = Simulation::default();
        // Ciudad al doble de población: la demanda se queda sin cubrir.
        for _ in 0..12 {
            sim.economy.population = 100_000;
            sim.tick_month();
        }
        assert!(sim.energy.has_deficit());

        let money = sim.economy.money;
        let gas = EnergySource::Gas.spec();
        let coste = sim
            .build_energy(EnergySource::Gas)
            .expect("debe poder construir");

        assert!((coste - gas.capital_per_plant() as f64).abs() < 1e-6);
        assert!((sim.economy.money - (money - coste)).abs() < 1e-6);
        assert_eq!(sim.energy.plant_count(EnergySource::Gas), 2);

        // Se sigue construyendo hasta cubrir la demanda.
        while sim.energy.has_deficit() && sim.energy.plant_count(EnergySource::Gas) < 6 {
            sim.build_energy(EnergySource::Gas).expect("más plantas");
        }
        assert!(!sim.energy.has_deficit(), "las plantas de gas deben cubrir el hueco");
    }

    #[test]
    fn construir_energia_sin_dinero_o_sin_suelo_falla() {
        let mut sim = Simulation::default();
        sim.economy.money = 0.0;
        assert_eq!(
            sim.build_energy(EnergySource::Coal),
            Err(ActionError::InsufficientFunds)
        );

        sim.economy.money = 100_000_000.0;
        let usado = sim.land.used(&sim.services, &sim.energy);
        sim.land.total_hectareas = usado + 1.0;
        // Una hidráulica ocupa 240 ha.
        assert_eq!(
            sim.build_energy(EnergySource::Hydro),
            Err(ActionError::InsufficientLand)
        );
    }

    #[test]
    fn demoler_energia_libera_suelo_y_devuelve_parte_del_capital() {
        let mut sim = Simulation::default();
        let antes = sim.energy.installed_mw();
        let suelo = sim.energy.land_hectareas();
        let carbon = EnergySource::Coal.spec();

        let devuelto = sim
            .demolish_energy(EnergySource::Coal)
            .expect("debe poder demoler");

        assert!((devuelto - (carbon.capital_per_plant() * super::super::services::DEMOLISH_REFUND) as f64).abs() < 1e-6);
        assert_eq!(sim.energy.plant_count(EnergySource::Coal), 0);
        assert!((sim.energy.installed_mw() - (antes - carbon.plant_mw)).abs() < 1e-4);
        assert!(sim.energy.land_hectareas() < suelo);
        // Ya no queda nada que demoler de esa fuente.
        assert_eq!(
            sim.demolish_energy(EnergySource::Coal),
            Err(ActionError::NoCapacity)
        );
    }

    #[test]
    fn las_renovables_son_mas_eficientes_de_explotar_y_los_fosiles_de_construir() {
        let carbon = EnergySource::Coal.spec();
        let eolica = EnergySource::Wind.spec();

        assert!(
            eolica.effective_mw_per_1000() > carbon.effective_mw_per_1000(),
            "la eólica debe dar más MW por mantenimiento"
        );
        assert!(
            carbon.capital_per_effective_mw() < eolica.capital_per_effective_mw(),
            "el carbón debe ser más barato de construir por MW efectivo"
        );
        assert!(carbon.emissions_kg_per_mwh > 0.0);
        assert_eq!(eolica.emissions_kg_per_mwh, 0.0);
    }

    #[test]
    fn las_emisiones_encarecen_el_mix_sucio() {
        let sucio = EnergyState::default();
        let mut limpio = EnergyState::default();
        limpio.get_mut(EnergySource::Coal).plants = 0;
        limpio.get_mut(EnergySource::Gas).plants = 0;
        limpio.get_mut(EnergySource::Nuclear).plants = 1;

        assert!(sucio.monthly_emissions_t() > limpio.monthly_emissions_t());
        assert!(sucio.emissions_cost() > limpio.emissions_cost());
        assert!(sucio.cost_per_mwh() > limpio.cost_per_mwh());
    }

    #[test]
    fn los_costes_de_energia_llegan_al_presupuesto() {
        let mut sim = Simulation::default();
        sim.tick_month();
        assert!(sim.economy.energy_upkeep > 0.0);
        assert!(sim.economy.pollution_cost > 0.0);

        let sin_energia = super::super::economy::EconomyState {
            energy_upkeep: 0.0,
            pollution_cost: 0.0,
            ..sim.economy.clone()
        };
        assert!(
            sim.economy.monthly_expenses(&sim.demographics)
                > sin_energia.monthly_expenses(&sim.demographics)
        );
    }

    #[test]
    fn el_mix_inicial_no_quiebra_las_arcas() {
        let mut sim = Simulation::default();
        sim.tick_month();
        let balance = sim.economy.monthly_balance(&sim.demographics);
        let ingresos = sim.economy.monthly_income(&sim.demographics);
        assert!(
            balance > -ingresos * 0.30,
            "el arranque no debe ser un déficit desesperado (balance {balance})"
        );
        assert!(!sim.energy.has_deficit());
    }
}

#[cfg(test)]
mod tests_instalaciones {
    use super::*;
    use crate::core::Simulation;

    #[test]
    fn cada_fuente_tiene_su_tamano_de_planta() {
        let tamanos: Vec<f32> = EnergySource::ALL
            .iter()
            .map(|s| s.spec().plant_mw)
            .collect();
        let unicos: std::collections::HashSet<String> =
            tamanos.iter().map(|t| t.to_string()).collect();
        assert_eq!(
            unicos.len(),
            tamanos.len(),
            "no todas las instalaciones deben medir lo mismo: {tamanos:?}"
        );
    }

    #[test]
    fn una_instalacion_produce_y_ocupa_segun_su_tamano() {
        let eolica = EnergySource::Wind.spec();
        assert_eq!(eolica.plant_mw, 100.0);
        assert!((eolica.effective_mw_per_plant() - 35.0).abs() < 1e-6);
        assert!((eolica.land_per_plant() - 60.0).abs() < 1e-3);
        assert!((eolica.capital_per_plant() - 1_200_000.0).abs() < 1e-3);
        assert!((eolica.upkeep_per_plant() - 4_000.0).abs() < 1e-3);
    }

    #[test]
    fn construir_suma_una_instalacion_y_su_capacidad() {
        let mut sim = Simulation::default();
        sim.economy.money = 5_000_000.0; // una eólica cuesta 1,2 M
        let antes = sim.energy.installed_mw();
        sim.build_energy(EnergySource::Wind).expect("construir");

        assert_eq!(sim.energy.plant_count(EnergySource::Wind), 1);
        assert!((sim.energy.installed_mw() - antes - 100.0).abs() < 1e-4);
        // 100 MW nominales rinden 35 efectivos.
        assert!((sim.energy.effective_mw_of(EnergySource::Wind) - 35.0).abs() < 1e-4);
    }

    #[test]
    fn la_partida_inicial_usa_poco_suelo() {
        let sim = Simulation::default();
        let porcentaje = sim.land.usage_percent(&sim.services, &sim.energy);
        assert!(
            porcentaje < 25.0,
            "arrancar con {porcentaje}% de suelo usado deja poco margen de juego"
        );
        assert!(sim.land.free(&sim.services, &sim.energy) > 3_000.0);
    }
}

#[cfg(test)]
mod tests_petroleo {
    use super::*;
    use crate::core::Simulation;

    #[test]
    fn el_petroleo_es_el_mas_barato_de_construir_y_el_mas_caro_de_explotar() {
        let petroleo = EnergySource::Oil.spec();
        let gas = EnergySource::Gas.spec();
        let carbon = EnergySource::Coal.spec();

        // Más barato por MW efectivo que gas y carbón…
        assert!(petroleo.capital_per_effective_mw() < gas.capital_per_effective_mw());
        assert!(petroleo.capital_per_effective_mw() < carbon.capital_per_effective_mw());
        // …y peor de explotar que ambos, con emisiones intermedias.
        assert!(petroleo.effective_mw_per_1000() < gas.effective_mw_per_1000());
        assert!(petroleo.effective_mw_per_1000() < carbon.effective_mw_per_1000());
        assert!(petroleo.emissions_kg_per_mwh > gas.emissions_kg_per_mwh);
        assert!(petroleo.emissions_kg_per_mwh < carbon.emissions_kg_per_mwh);
    }

    #[test]
    fn el_petroleo_es_una_fuente_suya_y_se_puede_construir() {
        let mut sim = Simulation::default();
        sim.economy.money = 10_000_000.0;
        assert_eq!(sim.energy.plant_count(EnergySource::Oil), 0);

        sim.build_energy(EnergySource::Oil).expect("construir petróleo");

        assert_eq!(sim.energy.plant_count(EnergySource::Oil), 1);
        assert!((sim.energy.installed_mw_of(EnergySource::Oil) - 150.0).abs() < 1e-4);
        assert!((sim.energy.effective_mw_of(EnergySource::Oil) - 135.0).abs() < 1e-4);
        assert!(sim.energy.monthly_emissions_t_of(EnergySource::Oil) > 0.0);
    }
}

#[cfg(test)]
mod tests_costes {
    use super::*;
    use crate::core::Simulation;

    #[test]
    fn el_coste_por_mw_refleja_mantenimiento_y_emisiones() {
        let mut sim = Simulation::default();
        let sucio = sim.energy.cost_per_mw_of(EnergySource::Coal);
        assert!(sucio > 0.0);

        // Una fuente limpia y barata de mantener cuesta menos por MW.
        sim.economy.money = 50_000_000.0;
        sim.build_energy(EnergySource::Wind).expect("eólica");
        let limpio = sim.energy.cost_per_mw_of(EnergySource::Wind);
        assert!(
            limpio < sucio,
            "eólica {limpio} debería costar menos por MW que carbón {sucio}"
        );
    }

    #[test]
    fn la_geotermica_es_una_base_cara_de_construir_y_barata_de_explotar() {
        let geo = EnergySource::Geothermal.spec();
        let nuclear = EnergySource::Nuclear.spec();

        // Más barata de construir que la nuclear y con emisiones ínfimas…
        assert!(geo.capital_per_effective_mw() < nuclear.capital_per_effective_mw());
        assert!(geo.emissions_kg_per_mwh < 100.0);
        // …y constante todo el año.
        for m in 1..=12 {
            assert_eq!(monthly_factor(EnergySource::Geothermal, m, 2_026), 1.0);
        }
    }
}

#[cfg(test)]
mod tests_variacion {
    use super::*;
    use crate::core::Simulation;

    /// La media de los 12 meses debe coincidir con el factor de la ficha: si no,
    /// las comparaciones de eficiencia de la tabla mentirían.
    #[test]
    fn la_media_anual_respeta_el_factor_de_carga() {
        for source in EnergySource::ALL {
            let media: f32 = (1..=12)
                .map(|m| monthly_factor(source, m, 2_026))
                .sum::<f32>()
                / 12.0;
            assert!(
                (media - 1.0).abs() < 0.06,
                "{source:?}: media anual {media} lejos de 1.0"
            );
        }
    }

    #[test]
    fn el_sol_rinde_mas_en_verano_y_el_viento_en_invierno() {
        let julio = monthly_factor(EnergySource::Solar, 7, 2_026);
        let enero = monthly_factor(EnergySource::Solar, 1, 2_026);
        assert!(julio > enero, "solar: julio {julio} debería superar enero {enero}");

        let enero_v = monthly_factor(EnergySource::Wind, 1, 2_026);
        let julio_v = monthly_factor(EnergySource::Wind, 7, 2_026);
        assert!(
            enero_v > julio_v,
            "eólica: enero {enero_v} debería superar julio {julio_v}"
        );
    }

    #[test]
    fn las_termicas_no_varian_con_la_estacion() {
        for source in [
            EnergySource::Coal,
            EnergySource::Oil,
            EnergySource::Gas,
            EnergySource::Nuclear,
        ] {
            for m in 1..=12 {
                // Producen lo que se les pide, siempre.
                assert_eq!(
                    monthly_factor(source, m, 2_026),
                    1.0,
                    "{source:?} mes {m} no debería variar"
                );
            }
        }
    }

    #[test]
    fn la_produccion_cambia_mes_a_mes() {
        let mut sim = Simulation::default();
        sim.economy.money = 50_000_000.0;
        // Mix 100 % renovable: es donde la variación se nota.
        for kind in [EnergySource::Coal, EnergySource::Gas] {
            sim.demolish_energy(kind).ok();
        }
        for _ in 0..6 {
            sim.build_energy(EnergySource::Solar).expect("solar");
            sim.build_energy(EnergySource::Wind).expect("eólica");
        }

        let mut producciones = Vec::new();
        for _ in 0..12 {
            sim.tick_month();
            producciones.push(sim.energy.production_mw());
        }
        let min = producciones.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = producciones.iter().cloned().fold(0.0_f32, f32::max);
        assert!(
            max > min * 1.05,
            "la producción debería variar entre meses (min {min}, max {max})"
        );
    }

    #[test]
    fn un_mes_malo_puede_abrir_un_deficit_que_la_media_no_ve() {
        let mut sim = Simulation::default();
        sim.economy.money = 50_000_000.0;
        // Solo solar: en invierno rinde mucho menos que su media anual.
        for kind in [EnergySource::Coal, EnergySource::Gas] {
            sim.demolish_energy(kind).ok();
        }
        for _ in 0..18 {
            sim.build_energy(EnergySource::Solar).expect("solar");
        }
        let mut meses_con_deficit = 0;
        for _ in 0..12 {
            sim.tick_month();
            if sim.energy.has_deficit() {
                meses_con_deficit += 1;
            }
        }
        assert!(
            meses_con_deficit > 0,
            "con solo solar, algún mes de invierno debería quedarse corto"
        );
    }
}
