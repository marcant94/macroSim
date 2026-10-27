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
        // Se redondea antes de decidir el signo: -0,004 no debe salir como "-0".
        let redondeado = (amount.abs() * 100.0).round() / 100.0;
        let sign = if amount < 0.0 && redondeado > 0.0 {
            "-"
        } else {
            ""
        };
        let s = format!("{:.2}", redondeado);
        let (int_part, frac_part) = match s.split_once('.') {
            Some((int, frac)) => (int.to_owned(), frac.to_owned()),
            None => (s, String::new()),
        };
        let grouped = self.group_digits(&int_part);
        // Importes redondos sin céntimos: 18.000 ₿ en vez de 18.000,00 ₿.
        let body = if frac_part.is_empty() || frac_part == "00" {
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

    /// Formatea un entero con separador de miles (`50.000` / `50,000`).
    pub fn format_int(&self, value: f64) -> String {
        let sign = if value < 0.0 { "-" } else { "" };
        let digits = format!("{:.0}", value.abs());
        format!("{}{}", sign, self.group_digits(&digits))
    }

    /// Inserta el separador de miles en una cadena de dígitos.
    fn group_digits(&self, int_part: &str) -> String {
        let digits = int_part.len();
        let mut grouped = String::with_capacity(digits + digits / 3);
        for (idx, c) in int_part.chars().enumerate() {
            if idx > 0 && (digits - idx).is_multiple_of(3) {
                grouped.push_str(&self.thousands_sep);
            }
            grouped.push(c);
        }
        grouped
    }

    /// Formatea un número decimal usando el separador decimal configurado.
    pub fn format_decimal(&self, value: f64, decimals: usize) -> String {
        format!("{:.*}", decimals, value).replace('.', &self.decimal_sep)
    }
}

/// Paleta de colores de la interfaz. Solo son datos: la conversión a
/// `egui::Visuals` vive en `ui::theme` para no meter egui en el núcleo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Azul profundo, tipo consola (la de por defecto).
    #[default]
    Midnight,
    /// Oscuro neutro.
    Dark,
    /// Claro, para pantallas con mucha luz.
    Light,
    /// Papel cálido, descansado para sesiones largas.
    Sepia,
}

impl Theme {
    /// Orden del desplegable: la de por defecto, primero.
    pub const ALL: [Theme; 4] = [Theme::Midnight, Theme::Dark, Theme::Light, Theme::Sepia];

    /// Clave i18n con el nombre de la paleta.
    pub fn name_key(self) -> &'static str {
        match self {
            Theme::Midnight => "theme.midnight",
            Theme::Dark => "theme.dark",
            Theme::Light => "theme.light",
            Theme::Sepia => "theme.sepia",
        }
    }


}

/// Ajustes del juego.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub currency: Currency,
    /// `default` por compatibilidad con ajustes guardados antes de existir.
    #[serde(default)]
    pub theme: Theme,
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
        assert_eq!(c.format_money(14_000_000.0), "14.000.000 ₿");
        assert_eq!(c.format_money(-1_234.5), "-1.234,50 ₿");
        assert_eq!(c.format_money(18_000.0), "18.000 ₿");
        assert_eq!(c.format_money(0.0), "0 ₿");
        assert_eq!(c.format_money(-0.004), "0 ₿");
        assert_eq!(c.format_money(1_234.56), "1.234,56 ₿");
        assert_eq!(c.format_money(0.5), "0,50 ₿");
    }

    #[test]
    fn formato_ingles_simbolo_al_principio() {
        let c = Currency::for_lang(Lang::En);
        assert_eq!(c.format_money(14_000_000.0), "₿14,000,000");
        assert_eq!(c.format_money(1_234.5), "₿1,234.50");
    }
}

#[cfg(test)]
mod tests_formato_entero {
    use super::*;

    #[test]
    fn el_entero_usa_separador_de_miles_del_idioma() {
        let es = Currency::for_lang(Lang::Es);
        assert_eq!(es.format_int(50_000.0), "50.000");
        assert_eq!(es.format_int(245_280.4), "245.280");
        assert_eq!(es.format_int(999.0), "999");
        assert_eq!(es.format_int(1_000_000.0), "1.000.000");
        assert_eq!(es.format_int(-12_345.0), "-12.345");

        let en = Currency::for_lang(Lang::En);
        assert_eq!(en.format_int(50_000.0), "50,000");
        assert_eq!(en.format_int(245_280.4), "245,280");
    }
}
