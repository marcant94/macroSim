//! Estado y lógica macroeconómica: presupuesto, impuestos, gasto y desempleo.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    pub money: f64,
    pub population: u32,
    pub unemployment_rate: f32, // Porcentaje (ej: 10.5)
    pub corporate_tax: f32,     // Porcentaje (ej: 21.0)
    pub income_tax: f32,
    pub public_spending: f32, // gasto mensual por habitante
}

impl Default for EconomyState {
    fn default() -> Self {
        Self {
            money: 1_000_000.0,
            population: 50_000,
            unemployment_rate: 8.5,
            corporate_tax: 20.0,
            income_tax: 15.0,
            public_spending: 2.0,
        }
    }
}

impl EconomyState {
    /// Ingresos mensuales estimados: impuesto de sociedades + IRPF (modelo simplificado).
    pub fn monthly_income(&self) -> f64 {
        self.population as f64
            * (self.corporate_tax as f64 * 0.5 + self.income_tax as f64 * 0.8)
    }

    /// Gasto mensual: gasto por habitante x población.
    pub fn monthly_expenses(&self) -> f64 {
        self.population as f64 * self.public_spending as f64
    }

    /// Balance mensual (ingresos - gastos). Negativo = déficit.
    pub fn monthly_balance(&self) -> f64 {
        self.monthly_income() - self.monthly_expenses()
    }

    /// Avanza la economía un mes.
    /// `energy_deficit` indica si la demanda energética superó la capacidad el mes pasado.
    pub fn tick(&mut self, energy_deficit: bool) {
        self.money += self.monthly_balance();

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
