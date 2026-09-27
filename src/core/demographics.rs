//! Demografía: población por clases sociales y satisfacción de cada una.
//!
//! Clave de diseño: la cobertura **pública** sostiene sobre todo a la clase
//! baja; la **privada** satisface especialmente a las clases media y alta.
//! El descontento de una clase (cobertura que recibe vs. impuestos que paga)
//! es la semilla futura de la delincuencia, las huelgas y la fuga de población.
//
// NOTA: la API todavía no la consume la UI; allow(dead_code) hasta entonces.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::economy::EconomyState;
use super::services::{ServiceKind, ServicesState};

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
    /// Población aproximada de una clase.
    pub fn population_of(&self, class: SocialClass, total: u32) -> u32 {
        let share = match class {
            SocialClass::Low => self.low_share,
            SocialClass::Mid => self.mid_share,
            SocialClass::High => self.high_share,
        };
        (total as f32 * share) as u32
    }

    /// Satisfacción de una clase (0..=1): media ponderada de las coberturas
    /// pública/privada de todos los servicios.
    pub fn satisfaction(&self, class: SocialClass, services: &ServicesState, population: u32) -> f32 {
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

    /// Avanza un mes: el reparto de clases deriva muy lentamente según
    /// satisfacción y economía (subir de clase con buen gobierno; caer con malo).
    pub fn tick(&mut self, services: &ServicesState, economy: &EconomyState) {
        let pop = economy.population;
        let s_mid = self.satisfaction(SocialClass::Mid, services, pop);

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
