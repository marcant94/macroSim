//! Demografía: población por clases sociales y satisfacción de cada una.
//!
//! Clave de diseño: la cobertura **pública** sostiene sobre todo a la clase
//! baja; la **privada** satisface especialmente a las clases media y alta.
//! El descontento de una clase (cobertura que recibe vs. impuestos que paga)
//! es la semilla futura de la delincuencia, las huelgas y la fuga de población.

use serde::{Deserialize, Serialize};

use super::economy::EconomyState;
use super::services::{ServiceKind, ServicesState};

/// Cuánta satisfacción resta la delincuencia con presión social máxima.
pub const CRIME_SATISFACTION_WEIGHT: f32 = 0.30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocialClass {
    Low,
    Mid,
    High,
}

impl SocialClass {
    /// Peso de la cobertura pública para esta clase (el resto es privado).
    /// La baja depende casi del todo del servicio público; la alta prefiere el privado.
    fn public_weight(self) -> f32 {
        match self {
            SocialClass::Low => 0.80,
            SocialClass::Mid => 0.45,
            SocialClass::High => 0.25,
        }
    }

    /// Renta relativa de la clase sobre la renta media (1.0 = clase media).
    pub fn income_weight(self) -> f32 {
        match self {
            SocialClass::Low => 0.5,
            SocialClass::Mid => 1.0,
            SocialClass::High => 2.5,
        }
    }

    /// Cuánto le afecta la fuga de capital: la clase alta es la que se va.
    pub fn flight_sensitivity(self) -> f32 {
        match self {
            SocialClass::Low => 0.0,
            SocialClass::Mid => 0.2,
            SocialClass::High => 1.0,
        }
    }

    /// Cada cuánto se queja la clase por pagar por encima del tipo neutro.
    fn tax_discontent_weight(self) -> f32 {
        // La clase baja es la más sensible a cada punto de IRPF que paga.
        match self {
            SocialClass::Low => 0.004,
            SocialClass::Mid => 0.003,
            SocialClass::High => 0.002,
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemographicsState {
    /// Reparto de la población por clases (las tres suman 1.0).
    pub low_share: f32,
    pub mid_share: f32,
    pub high_share: f32,
}

impl Default for DemographicsState {
    fn default() -> Self {
        Self { low_share: 0.45, mid_share: 0.40, high_share: 0.15 }
    }
}

impl DemographicsState {
    /// Población aproximada de una clase (para informes por clase: futuro).
    #[allow(dead_code)]
    pub fn population_of(&self, class: SocialClass, total: u32) -> u32 {
        let share = match class {
            SocialClass::Low => self.low_share,
            SocialClass::Mid => self.mid_share,
            SocialClass::High => self.high_share,
        };
        (total as f32 * share) as u32
    }

    /// Cuota de cada clase, en el orden de `SocialClass::ALL`.
    pub fn class_shares(&self) -> [(SocialClass, f32); 3] {
        [
            (SocialClass::Low, self.low_share),
            (SocialClass::Mid, self.mid_share),
            (SocialClass::High, self.high_share),
        ]
    }

    /// Cobertura que recibe una clase (0..=1): media ponderada de la pública y
    /// la privada de todos los servicios.
    pub fn services_satisfaction(
        &self,
        class: SocialClass,
        services: &ServicesState,
        population: u32,
    ) -> f32 {
        let w_pub = class.public_weight();
        let w_priv = 1.0 - w_pub;
        let mut acc = 0.0_f32;
        for kind in ServiceKind::ALL {
            let pub_cov = services.public_coverage(kind, population);
            let priv_cov = services.private_coverage(kind, population);
            acc += pub_cov * w_pub + priv_cov * w_priv;
        }
        acc / ServiceKind::ALL.len() as f32
    }

    /// Descontento fiscal de una clase (0..=0.30): pagar por encima de su tipo
    /// neutro resta satisfacción. Bajar impuestos no da "satisfacción extra",
    /// solo evita el castigo (el incentivo para bajar es la migración/inversión).
    pub fn tax_discontent(&self, class: SocialClass, economy: &EconomyState) -> f32 {
        let neutral = match class {
            SocialClass::Low => super::economy::NEUTRAL_INCOME_TAX_LOW,
            SocialClass::Mid => super::economy::NEUTRAL_INCOME_TAX_MID,
            SocialClass::High => super::economy::NEUTRAL_INCOME_TAX_HIGH,
        };
        let exceso = (economy.income_tax_of(class) - neutral).max(0.0);
        (exceso * class.tax_discontent_weight()).min(0.30)
    }

    /// Satisfacción de una clase (0..=1): lo que recibe de los servicios, menos
    /// lo que le duele pagar y lo que sufre la inseguridad. Es la semilla de la
    /// migración y, a la vez, de la propia delincuencia.
    ///
    /// `crime_pressure` (0..=1) lo calcula `core::crime`; se pasa como número
    /// para que este módulo no dependa del de delincuencia.
    pub fn satisfaction(
        &self,
        class: SocialClass,
        services: &ServicesState,
        population: u32,
        economy: &EconomyState,
        crime_pressure: f32,
    ) -> f32 {
        let cobertura = self.services_satisfaction(class, services, population);
        let inseguridad = crime_pressure.clamp(0.0, 1.0) * CRIME_SATISFACTION_WEIGHT;
        (cobertura - self.tax_discontent(class, economy) - inseguridad).clamp(0.0, 1.0)
    }

    /// Satisfacción media de la población (0..=1), ponderada por el peso de cada
    /// clase. Es el indicador que mueve la migración en `core::population`.
    pub fn average_satisfaction(
        &self,
        services: &ServicesState,
        population: u32,
        economy: &EconomyState,
        crime_pressure: f32,
    ) -> f32 {
        self.class_shares()
            .iter()
            .map(|(class, share)| {
                share * self.satisfaction(*class, services, population, economy, crime_pressure)
            })
            .sum()
    }

    /// Avanza un mes: el reparto de clases deriva muy lentamente según
    /// satisfacción y economía (subir de clase con buen gobierno; caer con malo).
    pub fn tick(&mut self, services: &ServicesState, economy: &EconomyState, crime_pressure: f32) {
        let pop = economy.population;
        let s_mid = self.satisfaction(SocialClass::Mid, services, pop, economy, crime_pressure);

        // Deriva mensual muy moderada: la movilidad social no es instantánea.
        let mut d_mid = 0.0_f32;
        let mut d_high = 0.0_f32;
        if s_mid > 0.6 {
            d_mid += 0.0004 * (s_mid - 0.6) * 2.5; // la baja asciende a media
        }
        if s_mid < 0.4 {
            d_mid -= 0.0004 * (0.4 - s_mid) * 2.5; // la media cae a la baja
        }
        if s_mid > 0.75 {
            d_high += 0.0002 * (s_mid - 0.75) * 4.0; // la media asciende a alta
        }
        // El paro empuja hacia abajo a toda la pirámide.
        let job_pressure = (economy.unemployment_rate - 8.0).max(0.0) / 100.0;
        d_high -= job_pressure * 0.001;
        d_mid -= job_pressure * 0.002;
        // Y el tramo alto demasiado alto expulsa a los ricos (fuga de capital).
        d_high -= economy.capital_flight() * 0.0006;

        self.low_share = (self.low_share - d_mid - d_high).clamp(0.05, 0.90);
        self.mid_share = (self.mid_share + d_mid - d_high).clamp(0.05, 0.90);
        self.high_share = (self.high_share + d_high).clamp(0.05, 0.90);
        self.normalize();
    }

    /// Garantiza que las tres cuotas sumen exactamente 1.0.
    fn normalize(&mut self) {
        let total = (self.low_share + self.mid_share + self.high_share).max(1e-6);
        self.low_share /= total;
        self.mid_share /= total;
        self.high_share /= total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POP: u32 = 50_000;

    #[test]
    fn subir_el_irpf_bajo_enfada_sobre_todo_a_la_clase_baja() {
        let demo = DemographicsState::default();
        let services = ServicesState::default();
        let base = EconomyState::default();
        let subido = EconomyState {
            income_tax_low: 40.0,
            ..EconomyState::default()
        };

        let low_base = demo.satisfaction(SocialClass::Low, &services, POP, &base, 0.0);
        let low_sub = demo.satisfaction(SocialClass::Low, &services, POP, &subido, 0.0);
        let high_base = demo.satisfaction(SocialClass::High, &services, POP, &base, 0.0);
        let high_sub = demo.satisfaction(SocialClass::High, &services, POP, &subido, 0.0);

        assert!(low_sub < low_base, "la clase baja debe notar el tramo bajo");
        assert!(
            (high_sub - high_base).abs() < 1e-6,
            "el tramo bajo no debe afectar a la clase alta"
        );
    }

    #[test]
    fn bajar_impuestos_no_da_satisfaccion_extra() {
        let demo = DemographicsState::default();
        let services = ServicesState::default();
        let base = EconomyState::default();
        let bajo = EconomyState {
            income_tax_low: 0.0,
            ..EconomyState::default()
        };
        let igual = demo.satisfaction(SocialClass::Low, &services, POP, &base, 0.0);
        let barato = demo.satisfaction(SocialClass::Low, &services, POP, &bajo, 0.0);
        assert!((igual - barato).abs() < 1e-6);
    }

    #[test]
    fn la_fuga_de_capital_reduce_el_peso_de_la_clase_alta() {
        let services = ServicesState::default();
        let economy = EconomyState {
            income_tax_high: 80.0,
            ..EconomyState::default()
        };
        let mut demo = DemographicsState::default();
        let antes = demo.high_share;
        for _ in 0..12 {
            demo.tick(&services, &economy, 0.0);
        }
        assert!(
            demo.high_share < antes,
            "con fuga de capital la clase alta debe encogerse ({} → {})",
            antes,
            demo.high_share
        );
    }
}
