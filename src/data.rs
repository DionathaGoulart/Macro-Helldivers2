//! Dados do jogo: estratagemas, equipamento, mapa de slugs das estatísticas e as
//! regras compartilhadas entre a aba de macros e a de builds.
//!
//! `stratagems.json` é pequeno e vale no boot inteiro. `equipment.json` e
//! `statsMap.json` somam ~130 KB e só interessam à aba de Builds, então ficam
//! atrás de um `OnceLock` carregado na primeira visita.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::util;

/// Direção de um passo do codex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    /// Índice nas tabelas de scancode de `keys.rs`.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Um estratagema como vem de `assets/data/stratagems.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stratagem {
    pub id: u32,
    pub nome: String,
    /// Caminho relativo a `assets/icons/` (ex.: `stratagems/Foo_Icon.webp`).
    pub imagem: String,
    #[serde(default)]
    pub tag: Vec<String>,
    pub codex: Vec<Dir>,
}

impl Stratagem {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tag.iter().any(|t| t == tag)
    }

    /// Primeira tag, que é como a UI agrupa a grade em seções.
    pub fn primary_tag(&self) -> Option<&str> {
        self.tag.first().map(String::as_str)
    }
}

/// Estratagema de apoio fixo — não ocupa slot e o codex nunca muda.
#[derive(Debug, Clone, Copy)]
pub struct SupportStrat {
    pub nome: &'static str,
    /// Caminho relativo a `assets/icons/`.
    pub imagem: &'static str,
    pub codex: &'static [Dir],
}

/// Reforço, Ressuprimento e Rearme da Águia, na ordem em que a UI os mostra.
pub const SUPPORT_STRATS: [SupportStrat; 3] = [
    SupportStrat {
        nome: "Reinforce",
        imagem: "Reinforce_Stratagem_Icon.webp",
        codex: &[Dir::Up, Dir::Down, Dir::Right, Dir::Left, Dir::Up],
    },
    SupportStrat {
        nome: "Resupply",
        imagem: "Resupply_Stratagem_Icon.webp",
        codex: &[Dir::Down, Dir::Down, Dir::Up, Dir::Right],
    },
    SupportStrat {
        nome: "Eagle Rearm",
        imagem: "Eagle_Rearm_Stratagem_Icon.webp",
        codex: &[Dir::Up, Dir::Up, Dir::Left, Dir::Up, Dir::Right],
    },
];

/// Tags que só permitem um estratagema equipado por vez (exos e veículos).
pub const EXCLUSIVE_TAGS: [&str; 2] = ["Mecha", "Vehicle"];

/// Um estratagema exclusivo conflita se outro slot já carrega a mesma tag.
/// `slots` pode ser mais curto que `active_slot` — a importação de backup checa
/// cada slot contra os anteriores.
pub fn has_exclusive_conflict(
    strat: &Stratagem,
    slots: &[Option<&Stratagem>],
    active_slot: usize,
) -> bool {
    EXCLUSIVE_TAGS.iter().any(|tag| {
        strat.has_tag(tag)
            && slots
                .iter()
                .enumerate()
                .any(|(i, slot)| i != active_slot && slot.is_some_and(|other| other.has_tag(tag)))
    })
}

/// Minúsculas e sem acento: "aguia" acha "Águia", "gatling" acha "A/G-16 Gatling".
pub fn normalize_text(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(deaccent)
        .collect()
}

/// Equivalente prático ao `NFD` + remoção de diacríticos do legado, restrito às
/// letras acentuadas que aparecem em português e nos nomes do jogo.
fn deaccent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ĩ' | 'ī' | 'į' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' => 'u',
        'ç' | 'ć' | 'č' => 'c',
        'ñ' | 'ń' | 'ň' => 'n',
        'ý' | 'ÿ' => 'y',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        'ł' => 'l',
        'đ' => 'd',
        'ğ' => 'g',
        'ř' => 'r',
        'ť' => 't',
        other => other,
    }
}

/// Lista de estratagemas indexada por id.
#[derive(Debug, Clone)]
pub struct GameData {
    stratagems: Vec<Stratagem>,
    index: HashMap<u32, usize>,
}

impl GameData {
    /// Lê `assets/data/stratagems.json`.
    pub fn load() -> Result<GameData> {
        GameData::load_from(&util::asset_path("data/stratagems.json"))
    }

    pub fn load_from(path: &Path) -> Result<GameData> {
        let bytes =
            std::fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
        GameData::from_json(&bytes)
    }

    pub fn from_json(bytes: &[u8]) -> Result<GameData> {
        let stratagems: Vec<Stratagem> =
            serde_json::from_slice(bytes).context("stratagems.json inválido")?;
        let index = stratagems
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();
        Ok(GameData { stratagems, index })
    }

    pub fn all(&self) -> &[Stratagem] {
        &self.stratagems
    }

    pub fn by_id(&self, id: u32) -> Option<&Stratagem> {
        self.index.get(&id).map(|&i| &self.stratagems[i])
    }

    /// Resolve ids de slot em estratagemas; ids que sumiram do JSON viram `None`.
    pub fn resolve(&self, ids: &[Option<u32>]) -> Vec<Option<&Stratagem>> {
        ids.iter()
            .map(|id| id.and_then(|id| self.by_id(id)))
            .collect()
    }
}

/// Arma primária, secundária ou granada.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Weapon {
    pub nome: String,
    pub tipo: String,
    pub dano: String,
    pub capacidade: Option<String>,
    pub cadencia: Option<String>,
    pub id: String,
    pub imagem: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Armor {
    pub nome: String,
    /// `Light` | `Medium` | `Heavy`.
    pub peso: String,
    pub armor: i32,
    pub speed: i32,
    pub stamina: i32,
    pub passive: String,
    pub warbond: String,
    pub id: String,
    pub imagem: String,
}

/// Capacete ou capa: só nome e warbond, que é o que o casamento de set usa.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Cosmetic {
    pub nome: String,
    pub warbond: String,
    pub id: String,
    pub imagem: String,
}

/// Booster ou passiva de armadura.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Described {
    pub nome: String,
    pub descricao: String,
    pub id: String,
    pub imagem: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Warbond {
    pub nome: String,
    pub tipo: String,
    pub data: String,
    pub custo: String,
    pub id: String,
    pub imagem: String,
}

/// Classificação da wiki usada pelas regras de build (arma de apoio, mochila).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StratagemInfo {
    pub nome: String,
    #[serde(rename = "type")]
    pub tipo: Option<String>,
    #[serde(default)]
    pub backpack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Equipment {
    pub primary: Vec<Weapon>,
    pub secondary: Vec<Weapon>,
    pub grenade: Vec<Weapon>,
    pub armor: Vec<Armor>,
    pub helmet: Vec<Cosmetic>,
    pub cape: Vec<Cosmetic>,
    pub booster: Vec<Described>,
    pub passives: Vec<Described>,
    pub warbond: Vec<Warbond>,
    pub stratagem_info: Vec<StratagemInfo>,
}

/// Item de equipamento referenciado por um slug das estatísticas.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WeaponRef {
    /// `primary` | `secondary` | `grenade`.
    pub cat: String,
    pub id: String,
}

/// Ponte entre os slugs do helldive.live e os itens do nosso JSON.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StatsMap {
    /// Slug → id de estratagema.
    pub strategem: HashMap<String, u32>,
    pub weapons: HashMap<String, WeaponRef>,
    /// Slug da passiva → nome em `passives`.
    pub armor: HashMap<String, String>,
}

static EQUIPMENT: OnceLock<Option<Equipment>> = OnceLock::new();
static STATS_MAP: OnceLock<Option<StatsMap>> = OnceLock::new();

/// Equipamento completo, carregado na primeira chamada. `None` se o asset faltar.
pub fn equipment() -> Option<&'static Equipment> {
    EQUIPMENT
        .get_or_init(|| load_asset("data/equipment.json"))
        .as_ref()
}

/// Mapa de slugs das estatísticas, carregado na primeira chamada.
pub fn stats_map() -> Option<&'static StatsMap> {
    STATS_MAP
        .get_or_init(|| load_asset("data/statsMap.json"))
        .as_ref()
}

fn load_asset<T: serde::de::DeserializeOwned>(rel: &str) -> Option<T> {
    let path = util::asset_path(rel);
    match std::fs::read(&path)
        .map_err(anyhow::Error::from)
        .and_then(|bytes| serde_json::from_slice(&bytes).with_context(|| format!("{rel} inválido")))
    {
        Ok(value) => Some(value),
        Err(err) => {
            log::error!("falha ao carregar {}: {err:#}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    #[test]
    fn stratagems_load_with_unique_ids_and_codexes() {
        let data = data();
        assert_eq!(data.all().len(), 91);
        for strat in data.all() {
            assert!(!strat.codex.is_empty(), "{} sem codex", strat.nome);
            assert!(
                strat.imagem.starts_with("stratagems/"),
                "{} com caminho de imagem inesperado: {}",
                strat.nome,
                strat.imagem
            );
            assert_eq!(data.by_id(strat.id).map(|s| &s.nome), Some(&strat.nome));
        }
    }

    #[test]
    fn dir_serializes_uppercase() {
        let codex = vec![Dir::Up, Dir::Down, Dir::Left, Dir::Right];
        let json = serde_json::to_string(&codex).unwrap();
        assert_eq!(json, r#"["UP","DOWN","LEFT","RIGHT"]"#);
        assert_eq!(serde_json::from_str::<Vec<Dir>>(&json).unwrap(), codex);
    }

    #[test]
    fn exclusive_tags_conflict_across_slots() {
        let data = data();
        let mechas: Vec<&Stratagem> = data.all().iter().filter(|s| s.has_tag("Mecha")).collect();
        assert!(mechas.len() >= 2, "o jogo tem mais de um exo");

        // Dois exos não cabem juntos...
        let slots = [Some(mechas[0]), None, None, None];
        assert!(has_exclusive_conflict(mechas[1], &slots, 1));
        // ...mas trocar o próprio slot pelo outro exo é permitido.
        assert!(!has_exclusive_conflict(mechas[1], &slots, 0));
        // E tags diferentes não conflitam entre si.
        let vehicle = data.all().iter().find(|s| s.has_tag("Vehicle")).unwrap();
        assert!(!has_exclusive_conflict(vehicle, &slots, 1));
    }

    #[test]
    fn normalize_strips_case_and_accents() {
        assert_eq!(normalize_text("Águia Metralhadora"), "aguia metralhadora");
        assert_eq!(normalize_text("A/G-16 Gatling"), "a/g-16 gatling");
        assert_eq!(normalize_text("Coração Órfão"), "coracao orfao");
        assert!(normalize_text("Orbital Precision Strike").contains("orbital"));
    }

    #[test]
    fn support_codexes_match_the_game() {
        assert_eq!(
            SUPPORT_STRATS[0].codex,
            [Dir::Up, Dir::Down, Dir::Right, Dir::Left, Dir::Up]
        );
        assert_eq!(
            SUPPORT_STRATS[1].codex,
            [Dir::Down, Dir::Down, Dir::Up, Dir::Right]
        );
        assert_eq!(
            SUPPORT_STRATS[2].codex,
            [Dir::Up, Dir::Up, Dir::Left, Dir::Up, Dir::Right]
        );
    }

    #[test]
    fn lazy_assets_parse() {
        let equipment = equipment().expect("equipment.json do repositório");
        assert!(!equipment.primary.is_empty());
        assert!(!equipment.stratagem_info.is_empty());
        assert!(equipment.armor.iter().all(|a| !a.passive.is_empty()));

        let stats = stats_map().expect("statsMap.json do repositório");
        assert!(!stats.strategem.is_empty());
        assert!(stats.weapons.values().all(|w| !w.cat.is_empty()));
    }

    #[test]
    fn stats_map_stratagem_slugs_point_at_real_ids() {
        let data = data();
        let stats = stats_map().unwrap();
        for (slug, id) in &stats.strategem {
            assert!(data.by_id(*id).is_some(), "slug {slug} aponta pro id {id}");
        }
    }

    #[test]
    fn resolve_drops_ids_that_no_longer_exist() {
        let data = data();
        let resolved = data.resolve(&[Some(0), Some(9_999), None]);
        assert_eq!(resolved[0].map(|s| s.id), Some(0));
        assert!(resolved[1].is_none());
        assert!(resolved[2].is_none());
    }
}
