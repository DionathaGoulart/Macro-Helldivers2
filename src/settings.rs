//! Preferências do usuário: schema, defaults, migração da v1 e persistência.
//!
//! O schema é o da v1 menos `sprintModifier` (o modificador de corrida saiu: o
//! hook de teclado dispara com qualquer modificador seguro). Campos desconhecidos
//! são ignorados na leitura, então um `settings.json` antigo entra sem conversão.

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

use crate::util;

/// Nome do arquivo, igual ao da v1 (a migração depende disso).
pub const SETTINGS_FILE: &str = "settings.json";

/// Perfil de velocidade do macro. Os números de cada perfil vivem no engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    #[default]
    Normal,
    Fast,
    Turbo,
}

impl Speed {
    pub const ALL: [Speed; 3] = [Speed::Normal, Speed::Fast, Speed::Turbo];

    pub fn as_str(self) -> &'static str {
        match self {
            Speed::Normal => "normal",
            Speed::Fast => "fast",
            Speed::Turbo => "turbo",
        }
    }

    /// Valores desconhecidos caem no padrão em vez de invalidar o arquivo inteiro.
    pub fn from_str_or_default(value: &str) -> Speed {
        match value {
            "fast" => Speed::Fast,
            "turbo" => Speed::Turbo,
            _ => Speed::Normal,
        }
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Speed {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Speed::from_str_or_default(&raw))
    }
}

/// Idioma da interface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Pt,
    En,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::Pt, Language::En];

    pub fn as_str(self) -> &'static str {
        match self {
            Language::Pt => "pt",
            Language::En => "en",
        }
    }

    pub fn from_str_or_default(value: &str) -> Language {
        match value {
            "en" => Language::En,
            _ => Language::Pt,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Language::from_str_or_default(&raw))
    }
}

/// Atalhos dos 4 slots de macro.
pub const SLOT_COUNT: usize = 4;
/// Atalhos dos 3 estratagemas de apoio fixo.
pub const SUPPORT_COUNT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    #[serde(deserialize_with = "de_shortcuts")]
    pub shortcuts: [Option<String>; SLOT_COUNT],
    #[serde(deserialize_with = "de_support_shortcuts")]
    pub support_shortcuts: [Option<String>; SUPPORT_COUNT],
    /// Tecla de estratagemas do jogo. Nome canônico de `keys.rs`.
    pub modifier_key: String,
    pub use_arrows: bool,
    pub macro_speed: Speed,
    pub language: Language,
    pub enable_overlay: bool,
    pub always_show_slots: bool,
    pub build_match_set: bool,
    pub build_balanced: bool,
    pub build_max_one_sentry: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shortcuts: [
                Some("F1".to_string()),
                Some("F2".to_string()),
                Some("F3".to_string()),
                Some("F4".to_string()),
            ],
            support_shortcuts: [None, None, None],
            modifier_key: "LeftControl".to_string(),
            use_arrows: false,
            macro_speed: Speed::Normal,
            language: Language::Pt,
            enable_overlay: true,
            always_show_slots: false,
            build_match_set: true,
            build_balanced: false,
            build_max_one_sentry: false,
        }
    }
}

impl Settings {
    /// Lê do disco. Sem arquivo novo, tenta migrar o da v1 e já grava no lugar
    /// certo; sem nada legível, devolve os padrões.
    pub fn load() -> Settings {
        let path = util::config_path(SETTINGS_FILE);
        if path.exists() {
            match Settings::load_from(&path) {
                Ok(settings) => return settings,
                Err(err) => log::warn!("settings.json ilegível ({err:#}); usando padrões"),
            }
        }

        for legacy in util::legacy_config_paths(SETTINGS_FILE) {
            if !legacy.exists() {
                continue;
            }
            match Settings::load_from(&legacy) {
                Ok(settings) => {
                    log::info!("migrando settings da v1: {}", legacy.display());
                    if let Err(err) = settings.save() {
                        log::warn!("falha ao gravar settings migrado: {err:#}");
                    }
                    return settings;
                }
                Err(err) => log::warn!("settings da v1 ilegível ({err:#}): {}", legacy.display()),
            }
        }

        Settings::default()
    }

    pub fn load_from(path: &Path) -> Result<Settings> {
        let bytes =
            std::fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
        Settings::from_json(&bytes)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Settings> {
        serde_json::from_slice(bytes).context("settings.json inválido")
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&util::config_path(SETTINGS_FILE))
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).context("falha ao serializar settings")?;
        util::write_atomic(path, &json)
    }

    /// Atalho do slot `index`, se configurado.
    pub fn shortcut(&self, index: usize) -> Option<&str> {
        self.shortcuts.get(index).and_then(|s| s.as_deref())
    }

    /// Atalho do apoio fixo `index`, se configurado.
    pub fn support_shortcut(&self, index: usize) -> Option<&str> {
        self.support_shortcuts.get(index).and_then(|s| s.as_deref())
    }
}

/// Arquivos podem vir com a lista curta ou longa demais (backup editado à mão,
/// versão antiga); normalizamos em vez de rejeitar o arquivo inteiro.
fn fixed<const N: usize>(mut values: Vec<Option<String>>) -> [Option<String>; N] {
    values.truncate(N);
    values.resize_with(N, || None);
    values
        .try_into()
        .unwrap_or_else(|_| unreachable!("tamanho normalizado acima"))
}

fn de_shortcuts<'de, D>(deserializer: D) -> Result<[Option<String>; SLOT_COUNT], D::Error>
where
    D: Deserializer<'de>,
{
    Ok(fixed(Vec::<Option<String>>::deserialize(deserializer)?))
}

fn de_support_shortcuts<'de, D>(
    deserializer: D,
) -> Result<[Option<String>; SUPPORT_COUNT], D::Error>
where
    D: Deserializer<'de>,
{
    Ok(fixed(Vec::<Option<String>>::deserialize(deserializer)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arquivo real da v1: tem `sprintModifier` e não tem os campos de build.
    const V1_JSON: &[u8] = br#"{
      "shortcuts": ["F1", "F2", "F3", "F4"],
      "supportShortcuts": ["F5", null, null],
      "modifierKey": "LeftAlt",
      "sprintModifier": "Shift",
      "useArrows": true,
      "macroSpeed": "turbo",
      "language": "en",
      "enableOverlay": false,
      "alwaysShowSlots": true
    }"#;

    #[test]
    fn v1_file_loads_and_drops_sprint_modifier() {
        let settings = Settings::from_json(V1_JSON).unwrap();
        assert_eq!(settings.modifier_key, "LeftAlt");
        assert_eq!(settings.macro_speed, Speed::Turbo);
        assert_eq!(settings.language, Language::En);
        assert!(settings.use_arrows);
        assert!(!settings.enable_overlay);
        assert!(settings.always_show_slots);
        assert_eq!(settings.support_shortcut(0), Some("F5"));
        assert_eq!(settings.support_shortcut(1), None);

        // Campos que a v1 não tinha vêm dos padrões.
        assert!(settings.build_match_set);
        assert!(!settings.build_balanced);
        assert!(!settings.build_max_one_sentry);

        // E `sprintModifier` não sobrevive à ida e volta.
        let round = String::from_utf8(serde_json::to_vec(&settings).unwrap()).unwrap();
        assert!(!round.contains("sprintModifier"));
    }

    #[test]
    fn empty_object_yields_defaults() {
        assert_eq!(Settings::from_json(b"{}").unwrap(), Settings::default());
    }

    #[test]
    fn round_trip_preserves_every_field() {
        let settings = Settings {
            shortcuts: [
                Some("Numpad1".into()),
                None,
                Some("Up".into()),
                Some("Equal".into()),
            ],
            support_shortcuts: [None, Some("PageDown".into()), None],
            modifier_key: "Minus".into(),
            macro_speed: Speed::Fast,
            language: Language::En,
            build_balanced: true,
            build_max_one_sentry: true,
            ..Settings::default()
        };

        let json = serde_json::to_vec(&settings).unwrap();
        assert_eq!(Settings::from_json(&json).unwrap(), settings);
    }

    #[test]
    fn unknown_enum_values_fall_back_instead_of_failing() {
        let settings =
            Settings::from_json(br#"{"macroSpeed":"ludicrous","language":"tlh"}"#).unwrap();
        assert_eq!(settings.macro_speed, Speed::Normal);
        assert_eq!(settings.language, Language::Pt);
    }

    #[test]
    fn shortcut_lists_are_normalized_to_fixed_length() {
        let short = Settings::from_json(br#"{"shortcuts":["F9"],"supportShortcuts":[]}"#).unwrap();
        assert_eq!(short.shortcut(0), Some("F9"));
        assert_eq!(short.shortcut(3), None);
        assert_eq!(short.support_shortcuts, [None, None, None]);

        let long =
            Settings::from_json(br#"{"shortcuts":["F1","F2","F3","F4","F5","F6"]}"#).unwrap();
        assert_eq!(long.shortcuts.len(), SLOT_COUNT);
        assert_eq!(long.shortcut(3), Some("F4"));
    }

    #[test]
    fn save_and_load_round_trip_on_disk() {
        let dir = std::env::temp_dir().join(format!("mh2-settings-{}", std::process::id()));
        let path = dir.join(SETTINGS_FILE);
        let settings = Settings {
            language: Language::En,
            ..Settings::default()
        };

        settings.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap(), settings);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
