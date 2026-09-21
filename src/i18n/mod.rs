//! Sistema de localización (i18n).
//!
//! Los textos de cada idioma viven en ficheros JSON separados
//! (`locales/es.json`, `locales/en.json`); el código solo usa claves
//! de traducción, nunca texto literal.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const LOCALES_ES: &str = include_str!("locales/es.json");
pub const LOCALES_EN: &str = include_str!("locales/en.json");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Es,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Es, Lang::En];

    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
        }
    }

    fn json(&self) -> &'static str {
        match self {
            Lang::Es => LOCALES_ES,
            Lang::En => LOCALES_EN,
        }
    }
}

/// Traductor en memoria: diccionario de claves → textos del idioma activo.
#[derive(Debug, Clone)]
pub struct I18n {
    lang: Lang,
    dict: HashMap<String, String>,
}

impl I18n {
    pub fn new(lang: Lang) -> Self {
        let mut i18n = Self { lang, dict: HashMap::new() };
        i18n.reload();
        i18n
    }

    #[allow(dead_code)]
    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
        self.reload();
    }

    fn reload(&mut self) {
        self.dict = match serde_json::from_str(self.lang.json()) {
            Ok(dict) => dict,
            Err(e) => {
                eprintln!("i18n: error al cargar el idioma {:?}: {}", self.lang, e);
                HashMap::new()
            }
        };
    }

    /// Texto sin parámetros. Si falta la clave, devuelve la propia clave
    /// para que sea visible en desarrollo.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.dict.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Texto con parámetros: las traducciones usan marcadores `{0}`, `{1}`, ...
    pub fn tf(&self, key: &str, args: &[&str]) -> String {
        let mut text = self.t(key).to_owned();
        for (i, arg) in args.iter().enumerate() {
            text = text.replace(&format!("{{{}}}", i), arg);
        }
        text
    }
}
