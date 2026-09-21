//! Calendario del juego: mes/año y avance del tiempo simulado.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Calendar {
    pub month: u32, // 1..=12
    pub year: u32,
}

impl Default for Calendar {
    fn default() -> Self {
        Self { month: 1, year: 2026 }
    }
}

impl Calendar {
    pub fn advance_month(&mut self) {
        self.month += 1;
        if self.month > 12 {
            self.month = 1;
            self.year += 1;
        }
    }
}
