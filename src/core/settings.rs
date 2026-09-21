//! Configuración de partida/usuario: idioma y formato de moneda.
//!
//! `Settings` es serializable para que en el futuro se guarde junto
//! con la partida o en un fichero de preferencias.

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

/// Formato de la moneda del juego: símbolo, separadores y posición.
/// En español el símbolo va detrás (RAE): `14.000.000,00 ₿`;
/// en inglés delante: `₿14,000,000.00`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    pub symbol: String,
    pub thousands_sep: String,
    pub decimal_sep: String,
    /// `true` = símbolo a la derecha de la cifra (español).
    pub symbol_at_end: bool,
}

impl Default for Currency {
    fn default() -> Self {
        Self {
            symbol: "₿".to_owned(),
            thousands_sep: ".".to_owned(),
            decimal_sep: ",".to_owned(),
            symbol_at_end: true,
        }
    }
}

impl Currency {
    /// Símbolos disponibles en el desplegable de Configuración.
    /// Todos están garantizados por DejaVu Sans / Noto Symbols.
    pub const AVAILABLE_SYMBOLS: [&'static str; 6] = ["₿", "§", "€", "$", "¤", "Cr"];

    /// Formato por idioma: español usa punto de millares y coma decimal
    /// con el símbolo a la derecha; inglés usa coma de millares y punto
    /// decimal con el símbolo a la izquierda. El símbolo es común (`₿`).
    pub fn for_lang(lang: Lang) -> Self {
        match lang {
            Lang::Es => Self {
                symbol: "₿".to_owned(),
                thousands_sep: ".".to_owned(),
                decimal_sep: ",".to_owned(),
                symbol_at_end: true,
            },
            Lang::En => Self {
                symbol: "₿".to_owned(),
                thousands_sep: ",".to_owned(),
                decimal_sep: ".".to_owned(),
                symbol_at_end: false,
            },
        }
    }

    /// Formatea una cantidad con 2 decimales:
    /// español `14.000.000,00 ₿` / inglés `₿14,000,000.00`.
    pub fn format_money(&self, amount: f64) -> String {
        let sign = if amount < 0.0 { "-" } else { "" };
        let s = format!("{:.2}", amount.abs());
        let (int_part, frac_part) = match s.split_once('.') {
            Some((int, frac)) => (int.to_owned(), frac.to_owned()),
            None => (s, String::new()),
        };
        let digits = int_part.len();
        let mut grouped = String::with_capacity(digits + digits / 3);
        for (idx, c) in int_part.chars().enumerate() {
            if idx > 0 && (digits - idx) % 3 == 0 {
                grouped.push_str(&self.thousands_sep);
            }
            grouped.push(c);
        }
        let body = if frac_part.is_empty() {
            grouped
        } else {
            format!("{}{}{}", grouped, self.decimal_sep, frac_part)
        };
        if self.symbol_at_end {
            // Español: símbolo detrás, separado por espacio (RAE)
            format!("{}{} {}", sign, body, self.symbol)
        } else {
            format!("{}{}{}", sign, self.symbol, body)
        }
    }

    /// Formatea un número decimal usando el separador decimal configurado.
    pub fn format_decimal(&self, value: f64, decimals: usize) -> String {
        format!("{:.*}", decimals, value).replace('.', &self.decimal_sep)
    }
}

/// Ajustes del juego.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub currency: Currency,
}

impl Settings {
    /// Cambia el idioma y aplica sus separadores numéricos,
    /// conservando el símbolo de moneda personalizado por el usuario.
    pub fn apply_language(&mut self, lang: Lang) {
        self.lang = lang;
        let symbol = self.currency.symbol.clone();
        self.currency = Currency::for_lang(lang);
        self.currency.symbol = symbol;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_espanol_simbolo_al_final() {
        let c = Currency::for_lang(Lang::Es);
        assert_eq!(c.format_money(14_000_000.0), "14.000.000,00 ₿");
        assert_eq!(c.format_money(-1_234.5), "-1.234,50 ₿");
    }

    #[test]
    fn formato_ingles_simbolo_al_principio() {
        let c = Currency::for_lang(Lang::En);
        assert_eq!(c.format_money(14_000_000.0), "₿14,000,000.00");
    }
}
