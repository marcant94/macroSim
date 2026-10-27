//! Estado y lógica macroeconómica: presupuesto, impuestos por tramos, gasto,
//! desempleo y fuga de capital.
//!
//! La fiscalidad tiene dos bases imponibles declaradas aquí (datos, no lógica):
//! la **renta de los hogares** (IRPF por tramos, según la clase social) y el
//! **beneficio de las empresas** (impuesto de sociedades). El tipo efectivo es
//! la recaudación total por habitante sobre la renta bruta: es el número que ve
//! el jugador y el que compara la población para decidir si se queda o se va.

use serde::{Deserialize, Serialize};

use super::demographics::{DemographicsState, SocialClass};

/// Renta bruta mensual media por habitante (base imponible del IRPF).
pub const GROSS_INCOME_PER_CAPITA: f32 = 30.0;
/// Beneficio mensual por habitante (base imponible del impuesto de sociedades).
pub const CORPORATE_PROFIT_PER_CAPITA: f32 = 10.0;

/// Impuestos de la partida inicial. Son también la referencia "neutra": por
/// encima de ellos la población se queja (IRPF) o huye el capital (tramo alto).
pub const NEUTRAL_CORPORATE_TAX: f32 = 20.0;
pub const NEUTRAL_INCOME_TAX_LOW: f32 = 8.0;
pub const NEUTRAL_INCOME_TAX_MID: f32 = 15.0;
pub const NEUTRAL_INCOME_TAX_HIGH: f32 = 25.0;

/// Tramo alto a partir del cual empieza la fuga de capital (%).
pub const HIGH_TAX_COMFORT: f32 = 35.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    pub money: f64,
    pub population: u32,
    pub unemployment_rate: f32, // Porcentaje (ej: 10.5)
    pub corporate_tax: f32,     // Porcentaje (ej: 21.0)
    /// IRPF del tramo bajo (%): castiga el consumo de la clase baja.
    pub income_tax_low: f32,
    /// IRPF del tramo medio (%).
    pub income_tax_mid: f32,
    /// IRPF del tramo alto (%): recauda más, pero expulsa capital.
    pub income_tax_high: f32,
    pub public_spending: f32, // gasto mensual por habitante
    /// Gasto mensual en servicios públicos (lo calcula `ServicesState::tick`).
    /// `default` por compatibilidad con partidas antiguas.
    #[serde(default)]
    pub service_upkeep: f64,
}

impl Default for EconomyState {
    fn default() -> Self {
        Self {
            money: 1_000_000.0,
            population: 50_000,
            unemployment_rate: 8.5,
            corporate_tax: NEUTRAL_CORPORATE_TAX,
            income_tax_low: NEUTRAL_INCOME_TAX_LOW,
            income_tax_mid: NEUTRAL_INCOME_TAX_MID,
            income_tax_high: NEUTRAL_INCOME_TAX_HIGH,
            public_spending: 2.0,
            service_upkeep: 0.0,
        }
    }
}

impl EconomyState {
    /// IRPF que paga una clase social.
    pub fn income_tax_of(&self, class: SocialClass) -> f32 {
        match class {
            SocialClass::Low => self.income_tax_low,
            SocialClass::Mid => self.income_tax_mid,
            SocialClass::High => self.income_tax_high,
        }
    }

    /// Recaudación mensual de IRPF por habitante, ponderando la renta y el tipo
    /// de cada clase. El tramo alto pierde eficacia con la fuga de capital.
    pub fn irpf_per_capita(&self, demographics: &DemographicsState) -> f32 {
        let flight = self.capital_flight();
        let mut total = 0.0;
        for (class, share) in demographics.class_shares() {
            let eficacia = 1.0 - 0.6 * flight * class.flight_sensitivity();
            total += share
                * GROSS_INCOME_PER_CAPITA
                * class.income_weight()
                * eficacia
                * self.income_tax_of(class)
                / 100.0;
        }
        total
    }

    /// Recaudación mensual del impuesto de sociedades por habitante.
    pub fn corporate_revenue_per_capita(&self) -> f32 {
        CORPORATE_PROFIT_PER_CAPITA * self.corporate_tax / 100.0
    }

    /// Ingresos fiscales totales por habitante y mes.
    pub fn revenue_per_capita(&self, demographics: &DemographicsState) -> f32 {
        self.irpf_per_capita(demographics) + self.corporate_revenue_per_capita()
    }

    /// Tipo efectivo total (%): recaudación por habitante sobre la renta bruta.
    /// Es lo que se paga de verdad, no el tipo del cartel.
    pub fn effective_tax_rate(&self, demographics: &DemographicsState) -> f32 {
        (self.revenue_per_capita(demographics) / GROSS_INCOME_PER_CAPITA * 100.0).clamp(0.0, 100.0)
    }

    /// Carga fiscal relativa al nivel neutro: 0 = impuestos de partida,
    /// +1 = se recauda el doble por habitante, -0,5 = la mitad.
    pub fn tax_burden(&self, demographics: &DemographicsState) -> f32 {
        let neutral = Self {
            corporate_tax: NEUTRAL_CORPORATE_TAX,
            income_tax_low: NEUTRAL_INCOME_TAX_LOW,
            income_tax_mid: NEUTRAL_INCOME_TAX_MID,
            income_tax_high: NEUTRAL_INCOME_TAX_HIGH,
            ..self.clone()
        };
        let base = neutral.revenue_per_capita(demographics);
        if base <= 0.0 {
            0.0
        } else {
            self.revenue_per_capita(demographics) / base - 1.0
        }
    }

    /// Fuga de capital (0..=1) provocada por el tramo alto: reduce la
    /// recaudación efectiva, la inversión privada y el peso de la clase alta.
    pub fn capital_flight(&self) -> f32 {
        ((self.income_tax_high - HIGH_TAX_COMFORT) / (100.0 - HIGH_TAX_COMFORT)).clamp(0.0, 1.0)
    }

    /// Ingresos mensuales estimados: IRPF por tramos + impuesto de sociedades.
    pub fn monthly_income(&self, demographics: &DemographicsState) -> f64 {
        self.population as f64 * self.revenue_per_capita(demographics) as f64
    }

    /// Gasto mensual: gasto por habitante x población + mantenimiento de
    /// los servicios públicos (clínicas, comisarías, basuras…).
    pub fn monthly_expenses(&self) -> f64 {
        self.population as f64 * self.public_spending as f64 + self.service_upkeep
    }

    /// Balance mensual (ingresos - gastos). Negativo = déficit.
    pub fn monthly_balance(&self, demographics: &DemographicsState) -> f64 {
        self.monthly_income(demographics) - self.monthly_expenses()
    }

    /// Avanza la economía un mes.
    /// `energy_deficit` indica si la demanda energética superó la capacidad el mes pasado.
    pub fn tick(&mut self, demographics: &DemographicsState, energy_deficit: bool) {
        self.money += self.monthly_balance(demographics);

        // El déficit energético castiga el empleo; con capacidad suficiente,
        // el paro tiende lentamente hacia un mínimo estructural del 3%.
        if energy_deficit {
            self.unemployment_rate += 0.2;
        } else if self.unemployment_rate > 3.0 {
            self.unemployment_rate -= 0.05;
        }
        self.unemployment_rate = self.unemployment_rate.clamp(0.0, 40.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipo_efectivo_inicial_coherente() {
        let economy = EconomyState::default();
        let demo = DemographicsState::default();
        let rate = economy.effective_tax_rate(&demo);
        // IRPF 5,15 + sociedades 2,00 por habitante sobre una renta bruta de 30.
        assert!((rate - 23.84).abs() < 0.1, "tipo efectivo = {rate}");
        assert!(economy.tax_burden(&demo).abs() < 1e-4);
    }

    #[test]
    fn subir_el_tramo_alto_recauda_mas_pero_dispara_la_fuga() {
        let demo = DemographicsState::default();
        let base = EconomyState::default();
        let alto = EconomyState {
            income_tax_high: 55.0,
            ..EconomyState::default()
        };

        assert_eq!(base.capital_flight(), 0.0);
        assert!(alto.capital_flight() > 0.0);
        assert!(
            alto.monthly_income(&demo) > base.monthly_income(&demo),
            "subir el tramo alto debe recaudar más…"
        );

        // …pero no en proporción al tipo: la fuga se come parte del aumento.
        let recaudado = alto.irpf_per_capita(&demo) - base.irpf_per_capita(&demo);
        let sin_fuga = GROSS_INCOME_PER_CAPITA
            * demo.high_share
            * SocialClass::High.income_weight()
            * (alto.income_tax_high - base.income_tax_high)
            / 100.0;
        assert!(
            recaudado < sin_fuga,
            "la fuga de capital debe recortar la recaudación ({recaudado} < {sin_fuga})"
        );
    }

    #[test]
    fn bajar_impuestos_reduce_el_tipo_efectivo() {
        let demo = DemographicsState::default();
        let economy = EconomyState {
            corporate_tax: 0.0,
            income_tax_low: 0.0,
            income_tax_mid: 0.0,
            income_tax_high: 0.0,
            ..EconomyState::default()
        };
        assert_eq!(economy.effective_tax_rate(&demo), 0.0);
        assert!(economy.tax_burden(&demo) < 0.0);
        assert_eq!(economy.monthly_income(&demo), 0.0);
    }

    #[test]
    fn el_balance_inicial_esta_ajustado() {
        // El mantenimiento de servicios se calcula en el tick; aquí se replica
        // el de la partida inicial para medir el balance real de partida.
        let economy = EconomyState {
            service_upkeep: super::super::services::ServicesState::default().monthly_upkeep(),
            ..EconomyState::default()
        };
        let demo = DemographicsState::default();
        let income = economy.monthly_income(&demo);
        let balance = economy.monthly_balance(&demo);
        // Ni colchón infinito ni quiebra instantánea: margen de ±25 %.
        assert!(
            balance.abs() < income * 0.25,
            "balance inicial {balance} frente a ingresos {income}"
        );
    }
}
