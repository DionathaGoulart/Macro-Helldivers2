//! Estatísticas de pick rate da comunidade (backend do helldive.live).
//!
//! É uma API não documentada de site de fã: pode mudar sem aviso, então as
//! structs são tolerantes (campo ausente vira o padrão) e uma falha degrada com
//! o aviso genérico da v1 em vez de derrubar a aba.
//!
//! Porte de `legacy/src/main/index.js` (~680–698) e do cache de 6h que o
//! renderer guardava no `localStorage` (`BuildTab.jsx` ~285–311). Aqui o cache é
//! um arquivo em `config_dir` (R9), o que também o faz sobreviver ao boot.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::shared::{Shared, UiEvent};
use crate::util;

const API: &str = "https://utm7j5pjvi.us-east-1.awsapprunner.com";

/// Patch "Exo Experts" — atualizar quando o site adicionar patch novo.
const PATCH_ID: u32 = 12;

/// O mesmo teto da v1 (`AbortSignal.timeout(10000)`), aplicado à requisição
/// inteira: conexão, TLS e corpo.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Arquivo de cache em `config_dir` (R9).
pub const CACHE_FILE: &str = "meta-cache.json";

/// Seis horas, como na v1: o site recalcula os números algumas vezes por dia.
pub const CACHE_TTL_MS: u64 = 6 * 3_600 * 1_000;

/// Os três endpoints que compõem uma consulta, no valor que o parâmetro `type`
/// espera.
const KINDS: [&str; 3] = ["strategem", "weapons", "armor"];

/// Facção consultada.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Faction {
    #[default]
    Terminid,
    Automaton,
    Illuminate,
}

impl Faction {
    pub const ALL: [Faction; 3] = [Faction::Terminid, Faction::Automaton, Faction::Illuminate];

    /// Valor do parâmetro `faction` — também a primeira metade da chave de cache.
    pub fn slug(self) -> &'static str {
        match self {
            Faction::Terminid => "terminid",
            Faction::Automaton => "automaton",
            Faction::Illuminate => "illuminate",
        }
    }
}

/// Dificuldades oferecidas pela tela: `0` é "todas", e o resto são os níveis
/// que o site separa.
pub const DIFFICULTIES: [u8; 5] = [0, 7, 8, 9, 10];

/// Números de um item numa consulta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemStat {
    /// Em quantos por cento dos loadouts ele aparece.
    #[serde(default)]
    pub loadouts_percentage: f64,
    /// Variação em relação ao período anterior; ausente em item novo.
    #[serde(default)]
    pub change: Option<f64>,
    #[serde(default, rename = "isNew")]
    pub is_new: Option<bool>,
}

impl ItemStat {
    pub fn change(self) -> f64 {
        self.change.unwrap_or_default()
    }

    pub fn is_new(self) -> bool {
        self.is_new.unwrap_or_default()
    }
}

/// Totais de uma consulta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Total {
    #[serde(default)]
    pub games: u64,
}

/// Resposta de um dos três endpoints: os itens por slug e os totais.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Section {
    #[serde(default)]
    pub items: HashMap<String, ItemStat>,
    #[serde(default)]
    pub total: Total,
}

/// As três respostas de uma combinação facção × dificuldade.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    #[serde(default)]
    pub strategem: Section,
    #[serde(default)]
    pub weapons: Section,
    #[serde(default)]
    pub armor: Section,
}

/// Resultado de uma consulta, entregue à janela por `UiEvent::MetaStats`.
#[derive(Debug, Clone, PartialEq)]
pub struct MetaResult {
    /// Combinação consultada; uma resposta que não é da combinação em tela é
    /// descartada (o usuário pode ter trocado de facção no meio).
    pub key: String,
    /// `None` quando a consulta falhou — a tela mostra o aviso genérico da v1.
    pub stats: Option<Arc<Stats>>,
}

/// Chave de cache no formato da v1: `"terminid|9"`.
pub fn cache_key(faction: Faction, difficulty: u8) -> String {
    format!("{}|{}", faction.slug(), difficulty)
}

/// Pede as estatísticas de uma combinação.
///
/// Cache fresco responde na hora, sem thread nem rede — é o que a v1 fazia com o
/// `localStorage`, e é o que evita a tela piscar "consultando" a cada troca de
/// aba. Sem cache, um worker efêmero consulta os três endpoints e o resultado
/// volta por `UiEvent::MetaStats`.
pub fn request(shared: &Arc<Shared>, faction: Faction, difficulty: u8) -> Option<Arc<Stats>> {
    let key = cache_key(faction, difficulty);
    if let Some(stats) = cached(&key) {
        return Some(Arc::new(stats));
    }

    let worker = Arc::clone(shared);
    let requested = key.clone();
    let spawned = std::thread::Builder::new()
        .name("meta".to_string())
        .spawn(move || {
            let stats = match fetch(faction, difficulty) {
                Ok(stats) => {
                    store(&requested, &stats);
                    Some(Arc::new(stats))
                }
                Err(err) => {
                    log::warn!("estatísticas da comunidade indisponíveis: {err:#}");
                    None
                }
            };
            worker.send_ui(UiEvent::MetaStats(MetaResult {
                key: requested,
                stats,
            }));
        });

    if let Err(err) = spawned {
        // Sem worker não há quem responda, e a tela ficaria carregando para
        // sempre: o aviso de erro sai daqui mesmo.
        log::warn!("worker de estatísticas não subiu: {err}");
        shared.send_ui(UiEvent::MetaStats(MetaResult { key, stats: None }));
    }
    None
}

/// Consulta os três endpoints em sequência. Qualquer um deles falhando derruba a
/// consulta inteira, como o `for` da v1.
pub fn fetch(faction: Faction, difficulty: u8) -> Result<Stats> {
    let agent = util::http_agent(Some(TIMEOUT));

    let mut sections = Vec::with_capacity(KINDS.len());
    for kind in KINDS {
        let url = url(faction, difficulty, kind);
        let section = agent
            .get(&url)
            .call()
            .with_context(|| format!("GET {url}"))?
            .body_mut()
            .read_json::<Section>()
            .with_context(|| format!("resposta inesperada de {kind}"))?;
        sections.push(section);
    }

    let mut sections = sections.into_iter();
    Ok(Stats {
        strategem: sections.next().unwrap_or_default(),
        weapons: sections.next().unwrap_or_default(),
        armor: sections.next().unwrap_or_default(),
    })
}

fn url(faction: Faction, difficulty: u8, kind: &str) -> String {
    format!(
        "{API}/items_stats?faction={}&patch_id={PATCH_ID}&difficulty={difficulty}\
         &mission=All&modifier=ALL&type={kind}",
        faction.slug()
    )
}

/// Uma entrada do arquivo de cache, no formato do R9.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Entry {
    /// Milissegundos desde a época, como o `Date.now()` da v1.
    at: u64,
    data: Stats,
}

type Cache = HashMap<String, Entry>;

/// Estatísticas ainda dentro da validade; `None` quando não há entrada ou ela
/// venceu.
fn cached(key: &str) -> Option<Stats> {
    let mut cache = read_cache();
    let entry = cache.remove(key)?;
    fresh(&entry, util::epoch_millis()).then_some(entry.data)
}

fn fresh(entry: &Entry, now: u64) -> bool {
    now.saturating_sub(entry.at) < CACHE_TTL_MS
}

fn read_cache() -> Cache {
    let path = util::config_path(CACHE_FILE);
    let Ok(bytes) = std::fs::read(&path) else {
        return Cache::new();
    };
    match serde_json::from_slice(&bytes) {
        Ok(cache) => cache,
        Err(err) => {
            // Cache corrompido não é problema: a consulta seguinte o reescreve.
            log::warn!("{} ilegível ({err}); consultando a rede", path.display());
            Cache::new()
        }
    }
}

/// Guarda a consulta, preservando as outras combinações já em cache.
fn store(key: &str, stats: &Stats) {
    let mut cache = read_cache();
    // Entradas vencidas de combinações que ninguém abre há dias só engordariam
    // o arquivo, que é lido inteiro a cada consulta.
    let now = util::epoch_millis();
    cache.retain(|_, entry| fresh(entry, now));
    cache.insert(
        key.to_string(),
        Entry {
            at: now,
            data: stats.clone(),
        },
    );

    let path = util::config_path(CACHE_FILE);
    let write = serde_json::to_vec(&cache)
        .context("falha ao serializar o cache")
        .and_then(|json| util::write_atomic(&path, &json));
    if let Err(err) = write {
        // Perder o cache custa uma consulta a mais na próxima abertura.
        log::warn!("cache de estatísticas não foi salvo: {err:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorte real de uma resposta de `type=strategem`, com os campos que a
    /// tela usa.
    const STRATAGEM_JSON: &[u8] = br#"{
        "total": { "games": 4009, "loadouts": 8716 },
        "items": {
            "barrage_napalm": {
                "loadouts_total": 2668, "loadouts_percentage": 30.6,
                "avg_level": 96, "change": 3.1, "isNew": false
            },
            "exo_lumberer": {
                "loadouts_total": 846, "loadouts_percentage": 9.7,
                "avg_level": 106, "change": 9.7, "isNew": true
            }
        }
    }"#;

    #[test]
    fn a_response_parses_with_the_fields_the_screen_uses() {
        let section: Section = serde_json::from_slice(STRATAGEM_JSON).unwrap();
        assert_eq!(section.total.games, 4009);
        assert_eq!(section.items.len(), 2);

        let napalm = section.items["barrage_napalm"];
        assert_eq!(napalm.loadouts_percentage, 30.6);
        assert_eq!(napalm.change(), 3.1);
        assert!(!napalm.is_new());
        assert!(section.items["exo_lumberer"].is_new());
    }

    #[test]
    fn missing_fields_fall_back_instead_of_failing() {
        // A API é de site de fã: um campo que suma não pode derrubar a aba.
        let section: Section = serde_json::from_slice(br#"{"items":{"x":{}}}"#).unwrap();
        assert_eq!(section.total.games, 0);
        let stat = section.items["x"];
        assert_eq!(stat.loadouts_percentage, 0.0);
        assert_eq!(stat.change(), 0.0);
        assert!(!stat.is_new());

        // E uma resposta inteira que falte também.
        let stats: Stats = serde_json::from_slice(b"{}").unwrap();
        assert!(stats.strategem.items.is_empty());
    }

    #[test]
    fn the_url_carries_every_parameter_the_api_expects() {
        let url = url(Faction::Automaton, 9, "weapons");
        assert!(url.starts_with(&format!("{API}/items_stats?")));
        for part in [
            "faction=automaton",
            "patch_id=12",
            "difficulty=9",
            "mission=All",
            "modifier=ALL",
            "type=weapons",
        ] {
            assert!(url.contains(part), "{url} sem {part}");
        }
        // Sem espaço perdido na quebra de linha do `format!`.
        assert!(!url.contains(' '), "{url}");
        assert_eq!(url.matches('?').count(), 1);
    }

    #[test]
    fn cache_keys_pair_faction_and_difficulty() {
        assert_eq!(cache_key(Faction::Terminid, 0), "terminid|0");
        assert_eq!(cache_key(Faction::Illuminate, 10), "illuminate|10");
        // Cada combinação tem a sua entrada.
        let keys: Vec<String> = Faction::ALL
            .into_iter()
            .flat_map(|faction| {
                DIFFICULTIES
                    .into_iter()
                    .map(move |difficulty| cache_key(faction, difficulty))
            })
            .collect();
        assert_eq!(keys.len(), 15);
        assert_eq!(
            keys.iter().collect::<std::collections::HashSet<_>>().len(),
            15
        );
    }

    #[test]
    fn an_entry_expires_after_six_hours() {
        let entry = Entry {
            at: 1_000_000,
            data: Stats::default(),
        };
        assert!(fresh(&entry, entry.at));
        assert!(fresh(&entry, entry.at + CACHE_TTL_MS - 1));
        assert!(!fresh(&entry, entry.at + CACHE_TTL_MS));
        // Relógio que andou para trás não faz a entrada valer para sempre.
        assert!(fresh(&entry, 0));
    }

    /// Consulta de verdade — fica de fora do `cargo test` padrão porque depende
    /// de rede e do site estar de pé. Rodar com
    /// `cargo test -- --ignored a_live_query` ao mexer no cliente.
    #[test]
    #[ignore = "depende de rede"]
    fn a_live_query_comes_back_populated() {
        let stats = fetch(Faction::Terminid, 0).expect("consulta ao helldive.live");
        assert!(stats.strategem.total.games > 0);
        assert!(!stats.strategem.items.is_empty());
        assert!(!stats.weapons.items.is_empty());
        assert!(!stats.armor.items.is_empty());
        assert!(stats
            .strategem
            .items
            .values()
            .any(|stat| stat.loadouts_percentage > 0.0));
    }

    #[test]
    fn the_cache_file_round_trips() {
        let stats = Stats {
            strategem: serde_json::from_slice(STRATAGEM_JSON).unwrap(),
            ..Stats::default()
        };
        let mut cache = Cache::new();
        cache.insert(
            cache_key(Faction::Terminid, 7),
            Entry {
                at: 1_700_000_000_000,
                data: stats.clone(),
            },
        );

        let json = serde_json::to_vec(&cache).unwrap();
        let read: Cache = serde_json::from_slice(&json).unwrap();
        let entry = &read["terminid|7"];
        assert_eq!(entry.at, 1_700_000_000_000);
        assert_eq!(entry.data, stats);
    }
}
