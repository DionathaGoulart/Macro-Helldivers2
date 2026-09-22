//! Dados do jogo: estratagemas, equipamento, mapa de slugs das estatísticas e as
//! regras compartilhadas entre a aba de macros e a de builds.
//!
//! `stratagems.json` é pequeno e vale no boot inteiro. `equipment.json` e
//! `statsMap.json` somam ~130 KB e só interessam à aba de Builds, então ficam
//! atrás de um `OnceLock` carregado na primeira visita.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Id do estratagema na API de dados: o que casa o JSON embarcado com o
    /// que [`crate::data_sync`] baixa. O `id` numérico continua sendo a chave
    /// dos saves.
    #[serde(default)]
    pub slug: String,
    pub nome: String,
    /// Caminho relativo a `assets/icons/` (ex.: `stratagems/Foo_Icon.webp`).
    /// Os baixados da API vivem sob `remote/`, fora da pasta de instalação.
    pub imagem: String,
    #[serde(default)]
    pub tag: Vec<String>,
    pub codex: Vec<Dir>,
    /// Classificação que veio da API, para os estratagemas que o
    /// `stratagemInfo` do `equipment.json` ainda não conhece.
    #[serde(skip)]
    pub kind_hint: Option<StratKind>,
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

/// Estratagema de apoio fixo: não ocupa slot e o codex nunca muda.
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
/// `slots` pode ser mais curto que `active_slot`: a importação de backup checa
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
    /// Lê `assets/data/stratagems.json` (só o que veio no instalador).
    pub fn load() -> Result<GameData> {
        GameData::load_from(&util::asset_path("data/stratagems.json"))
    }

    /// O JSON embarcado mais os estratagemas que a última sincronização com a
    /// API trouxe e ele ainda não tem. É o que o app usa; os testes ficam no
    /// [`GameData::load`], que não depende do que houver em `config_dir`.
    pub fn load_with_updates() -> Result<GameData> {
        let bundled = GameData::load()?.stratagems;
        let remote = crate::data_sync::load_cache();
        Ok(GameData::from_list(crate::data_sync::merge(
            bundled, &remote,
        )))
    }

    pub fn load_from(path: &Path) -> Result<GameData> {
        let bytes =
            std::fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
        GameData::from_json(&bytes)
    }

    pub fn from_json(bytes: &[u8]) -> Result<GameData> {
        let stratagems: Vec<Stratagem> =
            serde_json::from_slice(bytes).context("stratagems.json inválido")?;
        Ok(GameData::from_list(stratagems))
    }

    pub fn from_list(stratagems: Vec<Stratagem>) -> GameData {
        let index = stratagems
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();
        GameData { stratagems, index }
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

/// Categoria de equipamento de uma build, na ordem do `EQUIPMENT_SLOTS` da v1.
/// A ordem importa: é a das chaves de `loadouts.json` e a da grade da tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    Primary,
    Secondary,
    Grenade,
    Armor,
    Helmet,
    Cape,
    Booster,
}

pub const EQUIP_SLOT_COUNT: usize = 7;

impl EquipSlot {
    pub const ALL: [EquipSlot; EQUIP_SLOT_COUNT] = [
        EquipSlot::Primary,
        EquipSlot::Secondary,
        EquipSlot::Grenade,
        EquipSlot::Armor,
        EquipSlot::Helmet,
        EquipSlot::Cape,
        EquipSlot::Booster,
    ];

    /// Posição na ordem canônica: índice dos vetores de build e de locks.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Chave usada em `equipment.json` e no `equip` das builds salvas.
    pub fn key(self) -> &'static str {
        match self {
            EquipSlot::Primary => "primary",
            EquipSlot::Secondary => "secondary",
            EquipSlot::Grenade => "grenade",
            EquipSlot::Armor => "armor",
            EquipSlot::Helmet => "helmet",
            EquipSlot::Cape => "cape",
            EquipSlot::Booster => "booster",
        }
    }

    pub fn from_key(key: &str) -> Option<EquipSlot> {
        EquipSlot::ALL.into_iter().find(|slot| slot.key() == key)
    }

    /// Armas mostram tipo e dano; as demais categorias têm outra ficha.
    pub fn is_weapon(self) -> bool {
        matches!(
            self,
            EquipSlot::Primary | EquipSlot::Secondary | EquipSlot::Grenade
        )
    }
}

/// Um item de equipamento visto pela UI, seja qual for a categoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item<'a> {
    Weapon(&'a Weapon),
    Armor(&'a Armor),
    Cosmetic(&'a Cosmetic),
    Described(&'a Described),
}

impl<'a> Item<'a> {
    pub fn id(self) -> &'a str {
        match self {
            Item::Weapon(item) => &item.id,
            Item::Armor(item) => &item.id,
            Item::Cosmetic(item) => &item.id,
            Item::Described(item) => &item.id,
        }
    }

    pub fn nome(self) -> &'a str {
        match self {
            Item::Weapon(item) => &item.nome,
            Item::Armor(item) => &item.nome,
            Item::Cosmetic(item) => &item.nome,
            Item::Described(item) => &item.nome,
        }
    }

    /// Caminho relativo a `assets/icons/`.
    pub fn imagem(self) -> &'a str {
        match self {
            Item::Weapon(item) => &item.imagem,
            Item::Armor(item) => &item.imagem,
            Item::Cosmetic(item) => &item.imagem,
            Item::Described(item) => &item.imagem,
        }
    }

    /// Warbond do item, onde ela existe. É o que casa a capa com a armadura.
    pub fn warbond(self) -> Option<&'a str> {
        match self {
            Item::Armor(item) => Some(&item.warbond),
            Item::Cosmetic(item) => Some(&item.warbond),
            _ => None,
        }
    }
}

impl Equipment {
    /// Quantos itens a categoria tem.
    pub fn count(&self, slot: EquipSlot) -> usize {
        match slot {
            EquipSlot::Primary => self.primary.len(),
            EquipSlot::Secondary => self.secondary.len(),
            EquipSlot::Grenade => self.grenade.len(),
            EquipSlot::Armor => self.armor.len(),
            EquipSlot::Helmet => self.helmet.len(),
            EquipSlot::Cape => self.cape.len(),
            EquipSlot::Booster => self.booster.len(),
        }
    }

    /// Item pela posição na lista: o que a lista rolável do dropdown e o
    /// sorteio usam, sem montar vetor nenhum.
    pub fn at(&self, slot: EquipSlot, index: usize) -> Option<Item<'_>> {
        match slot {
            EquipSlot::Primary => self.primary.get(index).map(Item::Weapon),
            EquipSlot::Secondary => self.secondary.get(index).map(Item::Weapon),
            EquipSlot::Grenade => self.grenade.get(index).map(Item::Weapon),
            EquipSlot::Armor => self.armor.get(index).map(Item::Armor),
            EquipSlot::Helmet => self.helmet.get(index).map(Item::Cosmetic),
            EquipSlot::Cape => self.cape.get(index).map(Item::Cosmetic),
            EquipSlot::Booster => self.booster.get(index).map(Item::Described),
        }
    }

    /// Item pelo id gravado na build salva; `None` quando ele saiu do jogo.
    pub fn find(&self, slot: EquipSlot, id: &str) -> Option<Item<'_>> {
        (0..self.count(slot))
            .filter_map(|index| self.at(slot, index))
            .find(|item| item.id() == id)
    }

    /// Armadura pelo id. A ficha completa (peso, ARM/VEL/STA, passiva) só
    /// existe nesta categoria.
    pub fn armor_by_id(&self, id: &str) -> Option<&Armor> {
        self.armor.iter().find(|armor| armor.id == id)
    }

    /// Passiva pelo nome, que é como a armadura a referencia.
    pub fn passive(&self, nome: &str) -> Option<&Described> {
        self.passives.iter().find(|passive| passive.nome == nome)
    }
}

/// O que as regras de build precisam saber de um estratagema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StratKind {
    /// Arma de apoio (`type: "Support Weapon"` na wiki).
    pub support: bool,
    /// Ocupa o slot de mochila.
    pub backpack: bool,
    pub sentry: bool,
}

impl StratKind {
    /// Nem apoio, nem mochila, nem sentinela; também o que sai de um nome que
    /// não casou com nada.
    pub const NONE: StratKind = StratKind {
        support: false,
        backpack: false,
        sentry: false,
    };
}

/// Classificação de cada estratagema, casada com o `stratagemInfo` da wiki.
///
/// O casamento é por nome normalizado e reproduz o do legado (~70-101): exato
/// primeiro; substring só a partir de [`MIN_PARTIAL`] caracteres, ficando com o
/// candidato mais longo (o mais específico); e, por último, a designação (o
/// primeiro token do nome, que salva os apelidos que a wiki não registra).
#[derive(Debug, Clone, Default)]
pub struct StratMeta {
    kinds: HashMap<u32, StratKind>,
}

/// Abaixo disto uma substring casaria itens sem parentesco ("EAT" em
/// "Heat"), e o erro contamina as regras de apoio, mochila e sentinela.
const MIN_PARTIAL: usize = 6;

impl StratMeta {
    pub fn build(data: &GameData, equipment: &Equipment) -> StratMeta {
        let infos: Vec<(String, &StratagemInfo)> = equipment
            .stratagem_info
            .iter()
            .map(|info| (match_key(&info.nome), info))
            .collect();

        let mut kinds = HashMap::with_capacity(data.all().len());
        for strat in data.all() {
            let kind = match classify(&strat.nome, &infos) {
                // Estratagema que chegou pela API depois do último release: o
                // `stratagemInfo` ainda não o tem, e a API sabe o tipo dele.
                StratKind::NONE => strat.kind_hint.unwrap_or_default(),
                kind => kind,
            };
            kinds.insert(strat.id, kind);
        }
        StratMeta { kinds }
    }

    pub fn kind(&self, id: u32) -> StratKind {
        self.kinds.get(&id).copied().unwrap_or_default()
    }

    pub fn is_support(&self, id: u32) -> bool {
        self.kind(id).support
    }

    pub fn is_backpack(&self, id: u32) -> bool {
        self.kind(id).backpack
    }

    pub fn is_sentry(&self, id: u32) -> bool {
        self.kind(id).sentry
    }
}

/// Nome reduzido a letras e dígitos minúsculos, como o `norm` do legado.
fn match_key(name: &str) -> String {
    normalize_text(name)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

fn classify(nome: &str, infos: &[(String, &StratagemInfo)]) -> StratKind {
    let key = match_key(nome);

    let mut found = infos.iter().find(|(other, _)| *other == key);
    if found.is_none() {
        let mut best: Option<&(String, &StratagemInfo)> = None;
        for candidate in infos {
            let (other, _) = candidate;
            if other.len().min(key.len()) < MIN_PARTIAL {
                continue;
            }
            if !(key.contains(other.as_str()) || other.contains(key.as_str())) {
                continue;
            }
            if best.is_none_or(|(chosen, _)| other.len() > chosen.len()) {
                best = Some(candidate);
            }
        }
        found = best;
    }
    if found.is_none() {
        // Designação (ex.: "AX/ARC-3"): a wiki às vezes omite apelidos como
        // "Guard Dog", e o prefixo técnico é o que sobra em comum.
        let designation = match_key(nome.split(' ').next().unwrap_or_default());
        if designation.len() >= 4 {
            found = infos.iter().find(|(other, _)| other.contains(&designation));
        }
    }

    let Some((_, info)) = found else {
        return StratKind::default();
    };
    StratKind {
        support: info.tipo.as_deref() == Some("Support Weapon"),
        backpack: info.backpack,
        sentry: info.tipo.as_deref() == Some("Sentry"),
    }
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

/// Espelho do `TOKEN_SYNONYMS` de `scripts/build-stats-map.mjs`: pedaços dos
/// slugs do helldive.live e como eles aparecem nos nomes reais. Vazio casa
/// sempre (prefixos sem informação, como `sup_`).
const SLUG_SYNONYMS: [(&str, &[&str]); 11] = [
    ("sup", &[""]),
    ("n", &[""]),
    ("backpack", &["backpack", "pack"]),
    ("grenade", &["grenade", "g"]),
    ("encampment", &["emplacement", "battlement"]),
    ("gl", &["grenadelauncher", "gl"]),
    ("mg", &["mg", "machinegun"]),
    ("hmg", &["hmg", "heavymachinegun"]),
    ("amr", &["amr", "antimaterielrifle"]),
    ("inc", &["inc", "incendiary"]),
    ("at", &["at", "antitank"]),
];

impl StatsMap {
    /// Estratagema de cada slug de estratagema das estatísticas.
    ///
    /// O mapa embarcado resolve os conhecidos. Slug que ele não tem é de
    /// estratagema lançado depois do release (que o app já tem, via
    /// `data_sync`) e casa aqui pelos pedaços do slug no nome, como o
    /// `build-stats-map.mjs` faz. Com duas travas que o script não precisa,
    /// porque lá alguém confere o resultado: só concorrem estratagemas que o
    /// mapa ainda não cobre, e o par tem que ser único dos dois lados. Na
    /// dúvida o slug fica sem par. Melhor que mostrar o número de outro item.
    pub fn stratagem_ids<'a>(
        &self,
        slugs: impl IntoIterator<Item = &'a str>,
        data: &GameData,
    ) -> HashMap<&'a str, u32> {
        let mapped: HashSet<u32> = self.strategem.values().copied().collect();
        let candidates: Vec<(u32, String)> = data
            .all()
            .iter()
            .filter(|strat| !mapped.contains(&strat.id))
            .map(|strat| (strat.id, match_key(&strat.nome)))
            .collect();
        resolve_slugs(
            slugs,
            |slug| self.strategem.get(slug).copied(),
            &candidates,
            slug_matches,
        )
    }

    /// Arma de cada slug de arma das estatísticas, com a regra do
    /// [`StatsMap::stratagem_ids`]. As que concorrem ao casamento são as que o
    /// mapa não cobre: na prática, as que chegaram pela API depois do release
    /// (`equipment_sync`).
    pub fn weapon_refs<'a>(
        &self,
        slugs: impl IntoIterator<Item = &'a str>,
        equipment: &Equipment,
    ) -> HashMap<&'a str, WeaponRef> {
        let mapped: HashSet<&str> = self.weapons.values().map(|w| w.id.as_str()).collect();
        let candidates: Vec<(WeaponRef, String)> =
            [EquipSlot::Primary, EquipSlot::Secondary, EquipSlot::Grenade]
                .into_iter()
                .flat_map(|slot| {
                    (0..equipment.count(slot)).filter_map(move |index| {
                        let item = equipment.at(slot, index)?;
                        let reference = WeaponRef {
                            cat: slot.key().to_string(),
                            id: item.id().to_string(),
                        };
                        Some((reference, match_key(item.nome())))
                    })
                })
                .filter(|(reference, _)| !mapped.contains(reference.id.as_str()))
                .collect();
        resolve_slugs(
            slugs,
            |slug| self.weapons.get(slug).cloned(),
            &candidates,
            slug_matches,
        )
    }

    /// Passiva de cada chave de armadura das estatísticas (`SIEGE-READY`),
    /// com a mesma regra: só concorrem as passivas que o mapa não cobre.
    pub fn passive_names<'a>(
        &self,
        keys: impl IntoIterator<Item = &'a str>,
        equipment: &Equipment,
    ) -> HashMap<&'a str, String> {
        let mapped: HashSet<&str> = self.armor.values().map(String::as_str).collect();
        let candidates: Vec<(String, String)> = equipment
            .passives
            .iter()
            .filter(|passive| !mapped.contains(passive.nome.as_str()))
            .map(|passive| (passive.nome.clone(), match_key(&passive.nome)))
            .collect();
        resolve_slugs(
            keys,
            |key| self.armor.get(key).cloned(),
            &candidates,
            // As chaves vêm em caixa alta e separadas por hífen.
            |key, name| slug_matches(&key.to_ascii_lowercase().replace('-', "_"), name),
        )
    }
}

/// Resolve cada slug pelo mapa ou, na falta dele, pelo único candidato que
/// casa, desde que nenhum outro slug tenha casado com o mesmo candidato. Na
/// dúvida o slug fica sem par: melhor que mostrar o número de outro item.
fn resolve_slugs<'a, T: Clone + PartialEq>(
    slugs: impl IntoIterator<Item = &'a str>,
    known: impl Fn(&str) -> Option<T>,
    candidates: &[(T, String)],
    matches: impl Fn(&str, &str) -> bool,
) -> HashMap<&'a str, T> {
    let mut resolved = HashMap::new();
    let mut guesses: Vec<(&str, &T)> = Vec::new();
    for slug in slugs {
        if let Some(value) = known(slug) {
            resolved.insert(slug, value);
            continue;
        }
        let mut hits = candidates.iter().filter(|(_, key)| matches(slug, key));
        if let (Some((value, _)), None) = (hits.next(), hits.next()) {
            guesses.push((slug, value));
        }
    }
    for &(slug, value) in &guesses {
        if guesses.iter().filter(|(_, other)| *other == value).count() == 1 {
            resolved.insert(slug, value.clone());
        }
    }
    resolved
}

/// Todo pedaço do slug (`eagle_gas`) aparece no nome reduzido do item.
fn slug_matches(slug: &str, name_key: &str) -> bool {
    slug.split('_').all(|token| {
        let token = token.to_ascii_lowercase();
        match SLUG_SYNONYMS.iter().find(|(known, _)| *known == token) {
            Some((_, variants)) => variants.iter().any(|v| name_key.contains(v)),
            None => name_key.contains(&token),
        }
    })
}

static EQUIPMENT: OnceLock<Option<Equipment>> = OnceLock::new();
static STATS_MAP: OnceLock<Option<StatsMap>> = OnceLock::new();
/// O app pediu os itens novos da API junto com o JSON embarcado.
static EQUIPMENT_UPDATES: AtomicBool = AtomicBool::new(false);

/// Liga, para o resto da sessão, a mistura do equipamento embarcado com os
/// itens novos que a última sincronização com a API trouxe
/// ([`crate::equipment_sync`]). O boot chama antes da primeira visita à aba de
/// builds; os testes não chamam e ficam só com o JSON do repositório, que não
/// depende do que houver em `config_dir`. É o par do
/// [`GameData::load_with_updates`].
pub fn use_equipment_updates() {
    EQUIPMENT_UPDATES.store(true, Ordering::Relaxed);
}

/// Equipamento completo, carregado na primeira chamada. `None` se o asset faltar.
pub fn equipment() -> Option<&'static Equipment> {
    EQUIPMENT
        .get_or_init(|| {
            let mut equipment = bundled_equipment()?;
            if EQUIPMENT_UPDATES.load(Ordering::Relaxed) {
                crate::equipment_sync::merge(&mut equipment, &crate::equipment_sync::load_cache());
            }
            Some(equipment)
        })
        .as_ref()
}

/// Só o `equipment.json` do instalador, lido do disco a cada chamada.
pub fn bundled_equipment() -> Option<Equipment> {
    load_asset("data/equipment.json")
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
        assert_eq!(data.all().len(), 92);
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
    fn every_stratagem_carries_a_unique_api_slug() {
        let data = data();
        let mut seen = std::collections::HashSet::new();
        for strat in data.all() {
            assert!(!strat.slug.is_empty(), "{} sem slug", strat.nome);
            assert!(
                seen.insert(strat.slug.as_str()),
                "slug repetido: {}",
                strat.slug
            );
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

    fn id_named(data: &GameData, nome: &str) -> u32 {
        data.all().iter().find(|s| s.nome == nome).unwrap().id
    }

    #[test]
    fn a_slug_the_map_lacks_matches_a_stratagem_it_does_not_cover() {
        let data = data();
        let mut map = stats_map().unwrap().clone();
        map.strategem.remove("sup_quasar_cannon");

        let ids = map.stratagem_ids(["sup_quasar_cannon", "orbital_laser"], &data);
        assert_eq!(
            ids["sup_quasar_cannon"],
            id_named(&data, "LAS-99 Quasar Cannon")
        );
        assert_eq!(ids["orbital_laser"], map.strategem["orbital_laser"]);
    }

    #[test]
    fn the_stratagems_the_site_does_not_track_yet_would_match() {
        // Eagle Gas e Incinerator FRV não têm par no helldive.live hoje; quando
        // o site os incluir, com slugs no formato dos outros, eles casam.
        let data = data();
        let map = stats_map().unwrap();
        let ids = map.stratagem_ids(["eagle_gas", "frv_incinerator"], &data);
        assert_eq!(ids["eagle_gas"], id_named(&data, "Eagle Gas Airstrike"));
        assert_eq!(
            ids["frv_incinerator"],
            id_named(&data, "M-104 Incinerator FRV")
        );
    }

    #[test]
    fn a_weapon_or_passive_the_map_lacks_matches_its_slug() {
        let mut equipment = bundled_equipment().unwrap();
        equipment.primary.push(Weapon {
            nome: "AR-99 Teste".into(),
            tipo: "Assault Rifles".into(),
            dano: String::new(),
            capacidade: None,
            cadencia: None,
            id: "primary-ar-99-teste".into(),
            imagem: "remote/equipment/x.webp".into(),
        });
        equipment.passives.push(Described {
            nome: "Passiva De Teste".into(),
            descricao: String::new(),
            id: "passives-passiva-de-teste".into(),
            imagem: "remote/equipment/y.webp".into(),
        });
        let map = stats_map().unwrap();

        let refs = map.weapon_refs(["ar_99_teste", "spray_n_pray"], &equipment);
        assert_eq!(refs["ar_99_teste"].id, "primary-ar-99-teste");
        assert_eq!(refs["ar_99_teste"].cat, "primary");
        assert_eq!(refs["spray_n_pray"], map.weapons["spray_n_pray"]);

        let names = map.passive_names(["PASSIVA-DE-TESTE", "SIEGE-READY"], &equipment);
        assert_eq!(names["PASSIVA-DE-TESTE"], "Passiva De Teste");
        assert_eq!(names["SIEGE-READY"], "Siege-Ready");
        // Slug sem par nenhum fica de fora.
        assert!(!map
            .weapon_refs(["nao_existe_mesmo"], &equipment)
            .contains_key("nao_existe_mesmo"));
    }

    #[test]
    fn an_ambiguous_slug_stays_without_a_pair() {
        let data = data();
        let mut map = stats_map().unwrap().clone();
        map.strategem.remove("sup_quasar_cannon");
        map.strategem.remove("sup_laser_cannon");
        // Casa com as duas: nenhuma.
        assert!(map.stratagem_ids(["sup_cannon"], &data).is_empty());

        // Dois slugs no mesmo estratagema: nenhum dos dois.
        let mut map = stats_map().unwrap().clone();
        map.strategem.remove("sup_quasar_cannon");
        assert!(map
            .stratagem_ids(["sup_quasar_cannon", "sup_quasar"], &data)
            .is_empty());

        // Estratagema que o mapa já cobre não concorre.
        let map = stats_map().unwrap();
        assert!(map.stratagem_ids(["sup_autocannon_mk2"], &data).is_empty());
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
    fn equipment_slots_expose_every_category_by_key_and_index() {
        let equipment = equipment().unwrap();
        for (index, slot) in EquipSlot::ALL.iter().enumerate() {
            assert_eq!(slot.index(), index);
            assert_eq!(EquipSlot::from_key(slot.key()), Some(*slot));
            assert!(equipment.count(*slot) > 0, "{} vazio", slot.key());

            let first = equipment.at(*slot, 0).expect("primeiro item");
            assert!(!first.nome().is_empty());
            assert!(first.imagem().starts_with("equipment/"));
            assert_eq!(equipment.find(*slot, first.id()), Some(first));
        }
        assert_eq!(EquipSlot::from_key("sprintModifier"), None);
        assert_eq!(equipment.find(EquipSlot::Armor, "nao-existe"), None);
    }

    #[test]
    fn only_armor_and_cosmetics_carry_a_warbond() {
        let equipment = equipment().unwrap();
        assert!(equipment
            .at(EquipSlot::Armor, 0)
            .unwrap()
            .warbond()
            .is_some());
        assert!(equipment
            .at(EquipSlot::Cape, 0)
            .unwrap()
            .warbond()
            .is_some());
        assert_eq!(equipment.at(EquipSlot::Primary, 0).unwrap().warbond(), None);
        assert_eq!(equipment.at(EquipSlot::Booster, 0).unwrap().warbond(), None);
    }

    #[test]
    fn armor_passives_all_resolve_to_a_description() {
        let equipment = equipment().unwrap();
        for armor in &equipment.armor {
            assert!(
                equipment.passive(&armor.passive).is_some(),
                "{} usa a passiva desconhecida {}",
                armor.nome,
                armor.passive
            );
        }
    }

    #[test]
    fn stratagem_metadata_matches_the_wiki_classification() {
        let data = data();
        let meta = StratMeta::build(&data, equipment().unwrap());

        let by_name = |name: &str| {
            data.all()
                .iter()
                .find(|strat| strat.nome.contains(name))
                .unwrap_or_else(|| panic!("{name} não está no stratagems.json"))
                .id
        };

        // Arma de apoio sem mochila, arma de apoio COM mochila e sentinela.
        let recoilless = by_name("Recoilless Rifle");
        assert!(meta.is_support(recoilless));
        assert!(meta.is_backpack(recoilless), "o canhão vem com a mochila");
        assert!(!meta.is_sentry(recoilless));

        let machine_gun = by_name("MG-43");
        assert!(meta.is_support(machine_gun));
        assert!(!meta.is_backpack(machine_gun));

        let sentry = by_name("Machine Gun Sentry");
        assert!(meta.is_sentry(sentry));
        assert!(!meta.is_support(sentry));

        // Um estratagema orbital não é nada disso.
        let orbital = by_name("Orbital Precision Strike");
        assert_eq!(meta.kind(orbital), StratKind::default());
        // E um id que não existe não entra em pânico.
        assert_eq!(meta.kind(9_999), StratKind::default());
    }

    #[test]
    fn most_stratagems_find_their_wiki_entry() {
        let data = data();
        let meta = StratMeta::build(&data, equipment().unwrap());
        // O casamento é por nome, então alguns nomes de apelido sobram; o que
        // não pode é o conjunto inteiro cair no padrão (regra de balanceado
        // ficaria sem apoio nenhum para escolher).
        let supports = data
            .all()
            .iter()
            .filter(|strat| meta.is_support(strat.id))
            .count();
        let backpacks = data
            .all()
            .iter()
            .filter(|strat| meta.is_backpack(strat.id))
            .count();
        let sentries = data
            .all()
            .iter()
            .filter(|strat| meta.is_sentry(strat.id))
            .count();
        assert!(supports >= 10, "apoios encontrados: {supports}");
        assert!(backpacks >= 5, "mochilas encontradas: {backpacks}");
        assert!(sentries >= 5, "sentinelas encontradas: {sentries}");
    }

    #[test]
    fn the_match_key_strips_everything_but_letters_and_digits() {
        assert_eq!(match_key("A/G-16 Gatling"), "ag16gatling");
        assert_eq!(match_key("Águia Metralhadora"), "aguiametralhadora");
        assert_eq!(match_key("EAT-17"), "eat17");
    }

    #[test]
    fn a_short_name_never_matches_by_substring() {
        let info = StratagemInfo {
            nome: "Heat Sink".to_string(),
            tipo: Some("Support Weapon".to_string()),
            backpack: false,
        };
        let infos = vec![(match_key(&info.nome), &info)];
        // "eat17" tem 5 caracteres: abaixo do piso, então não casa com "heatsink".
        assert_eq!(classify("EAT-17", &infos), StratKind::default());
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
