//! Equipamento novo sem release: armas, armaduras, capacetes, capas, boosters e
//! passivas que a API de dados conhece e o `equipment.json` embarcado ainda não.
//!
//! Segue as regras do [`crate::data_sync`] (é o mesmo worker): uma consulta por
//! sessão, nunca com o jogo em foco, e o resultado vale a partir do próximo
//! boot. O JSON embarcado manda em tudo o que ele já tem; da API só entra o que
//! falta, na ordem alfabética da categoria, com o ícone baixado para a pasta de
//! configuração.
//!
//! O `equipment.json` embarcado sai da mesma API (`scripts/sync-equipment.mjs`),
//! com as mesmas regras de conversão: o script espelha [`from_api`] e
//! [`name_key`], e um item que o app baixou aqui entra no release seguinte com o
//! mesmo id e a mesma ficha.
//!
//! Nada daqui vira `SendInput`: o pior que um dado ruim faz é uma linha
//! estranha no dropdown. Mesmo assim tudo é validado, na chegada e ao ler o
//! cache (que fica numa pasta gravável pelo usuário), e o nome do ícone vira
//! caminho no disco.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::data::{self, Armor, Cosmetic, Described, Equipment, Weapon};
use crate::data_sync;
use crate::util;

/// Cache em `config_dir` com os itens novos da última sincronização.
pub const CACHE_FILE: &str = "equipment-remote.json";

/// Versão do formato do cache: um app mais novo ignora o arquivo de um mais
/// velho em vez de interpretá-lo errado.
const CACHE_VERSION: u32 = 1;

/// Subpasta de `remote-icons` com os ícones de equipamento. Separada da dos
/// estratagemas: a faxina de lá apaga todo arquivo que não é estratagema.
const ICON_DIR: &str = "equipment";

/// Prefixo do `imagem` dos itens baixados, relativo a `assets/icons/`.
const ICON_PREFIX: &str = "remote/equipment/";

/// A maior lista (armas) tem ~550 KB crus e ~50 KB com gzip.
const MAX_LIST_BYTES: u64 = 8 * 1024 * 1024;

/// Teto de itens novos por categoria numa sincronização. Um patch traz dois ou
/// três; dezenas de uma vez é a API quebrada (nomes num formato novo que não
/// casam com nada), e aí a categoria inteira fica de fora.
const MAX_NEW_PER_CATEGORY: usize = 12;

/// Limites de texto: nome e ficha curtos, descrição de booster e passiva maior.
const MAX_NAME_CHARS: usize = 80;
const MAX_TEXT_CHARS: usize = 400;

/// Coleções da API, na ordem em que são baixadas. Passivas primeiro: a
/// armadura aponta para a dela pelo id.
const COLLECTIONS: [&str; 6] = [
    "passives", "weapons", "armors", "helmets", "capes", "boosters",
];

/// Categoria do item, com a chave do `equipment.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Primary,
    Secondary,
    Grenade,
    Armor,
    Helmet,
    Cape,
    Booster,
    Passive,
}

impl Category {
    const ALL: [Category; 8] = [
        Category::Primary,
        Category::Secondary,
        Category::Grenade,
        Category::Armor,
        Category::Helmet,
        Category::Cape,
        Category::Booster,
        Category::Passive,
    ];

    /// Prefixo do id no `equipment.json` (`primary-ar-23-liberator`).
    fn key(self) -> &'static str {
        match self {
            Category::Primary => "primary",
            Category::Secondary => "secondary",
            Category::Grenade => "grenade",
            Category::Armor => "armor",
            Category::Helmet => "helmet",
            Category::Cape => "cape",
            Category::Booster => "booster",
            Category::Passive => "passives",
        }
    }

    /// Coleção da API, que também é a pasta das imagens dela.
    fn collection(self) -> &'static str {
        match self {
            Category::Primary | Category::Secondary | Category::Grenade => "weapons",
            Category::Armor => "armors",
            Category::Helmet => "helmets",
            Category::Cape => "capes",
            Category::Booster => "boosters",
            Category::Passive => "passives",
        }
    }

    /// Nome e id de cada item embarcado da categoria.
    fn bundled(self, equipment: &Equipment) -> Vec<(&str, &str)> {
        fn pairs<T: Named>(items: &[T]) -> Vec<(&str, &str)> {
            items.iter().map(Named::names).collect()
        }
        match self {
            Category::Primary => pairs(&equipment.primary),
            Category::Secondary => pairs(&equipment.secondary),
            Category::Grenade => pairs(&equipment.grenade),
            Category::Armor => pairs(&equipment.armor),
            Category::Helmet => pairs(&equipment.helmet),
            Category::Cape => pairs(&equipment.cape),
            Category::Booster => pairs(&equipment.booster),
            Category::Passive => pairs(&equipment.passives),
        }
    }
}

/// Nome e id de um item embarcado, seja qual for o tipo dele.
trait Named {
    fn names(&self) -> (&str, &str);
}

impl Named for Weapon {
    fn names(&self) -> (&str, &str) {
        (&self.nome, &self.id)
    }
}

impl Named for Armor {
    fn names(&self) -> (&str, &str) {
        (&self.nome, &self.id)
    }
}

impl Named for Cosmetic {
    fn names(&self) -> (&str, &str) {
        (&self.nome, &self.id)
    }
}

impl Named for Described {
    fn names(&self) -> (&str, &str) {
        (&self.nome, &self.id)
    }
}

/// Um item novo da API, já no formato do app. É também o formato do cache.
///
/// Os campos da ficha dependem da categoria e ficam vazios nas outras: tipo,
/// dano, capacidade e cadência nas armas; peso, ARM/VEL/STA e passiva na
/// armadura; warbond na armadura, no capacete e na capa; descrição no booster
/// e na passiva.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteItem {
    pub category: Category,
    /// Id da API. Só serve para achar a passiva da armadura.
    pub slug: String,
    pub nome: String,
    /// Outros nomes pelos quais a wiki conhece o item.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Caminho da imagem na API (`/images/v1/weapons/<slug>.<hash>.webp`).
    pub image: String,
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub dano: String,
    #[serde(default)]
    pub capacidade: Option<String>,
    #[serde(default)]
    pub cadencia: Option<String>,
    #[serde(default)]
    pub peso: String,
    #[serde(default)]
    pub armor: i32,
    #[serde(default)]
    pub speed: i32,
    #[serde(default)]
    pub stamina: i32,
    #[serde(default)]
    pub passive: String,
    #[serde(default)]
    pub warbond: String,
    #[serde(default)]
    pub descricao: String,
}

impl RemoteItem {
    /// Id no formato do `equipment.json`: é o que o `sync-equipment.mjs` grava
    /// quando o item entra num release, então build salva antes dele continua
    /// valendo depois.
    pub fn id(&self) -> String {
        format!("{}-{}", self.category.key(), slug(&self.nome))
    }

    /// Nome do arquivo na API. Já vem com hash de conteúdo.
    fn api_file(&self) -> &str {
        self.image.rsplit('/').next().unwrap_or_default()
    }

    /// Nome do arquivo em `remote-icons/equipment/`. Leva a categoria na frente:
    /// capacete e armadura de mesmo nome têm arquivos com nomes parecidos na API.
    fn icon_file(&self) -> String {
        format!("{}-{}", self.category.key(), self.api_file())
    }

    /// Caminho do ícone relativo a `assets/icons/`, como o resto do app usa.
    fn imagem(&self) -> String {
        format!("{ICON_PREFIX}{}", self.icon_file())
    }

    /// Confere tudo o que o app lê. O cache passa por aqui de novo na leitura.
    fn is_valid(&self) -> bool {
        let text_ok = |text: &str, max: usize| {
            text.chars().count() <= max && !text.chars().any(char::is_control)
        };
        let optional_ok =
            |text: &Option<String>| text.as_deref().is_none_or(|t| text_ok(t, MAX_TEXT_CHARS));
        let stat_ok = |value: i32| (0..=1_000).contains(&value);
        let prefix = format!("/images/v1/{}/", self.category.collection());

        let common = data_sync::valid_slug(&self.slug)
            && !self.nome.trim().is_empty()
            && text_ok(&self.nome, MAX_NAME_CHARS)
            && !slug(&self.nome).is_empty()
            && self.aliases.len() <= 8
            && self
                .aliases
                .iter()
                .all(|alias| text_ok(alias, MAX_NAME_CHARS))
            && self.image.starts_with(&prefix)
            && self.image[prefix.len()..] == *self.api_file()
            && data_sync::valid_icon_file(self.api_file())
            && [
                &self.tipo,
                &self.dano,
                &self.peso,
                &self.passive,
                &self.warbond,
                &self.descricao,
            ]
            .into_iter()
            .all(|text| text_ok(text, MAX_TEXT_CHARS))
            && optional_ok(&self.capacidade)
            && optional_ok(&self.cadencia)
            && [self.armor, self.speed, self.stamina]
                .into_iter()
                .all(stat_ok);
        let specific = match self.category {
            Category::Armor => {
                matches!(self.peso.as_str(), "Light" | "Medium" | "Heavy")
                    && !self.passive.trim().is_empty()
            }
            _ => true,
        };
        common && specific
    }

    /// O item no formato em que ele entra no `Equipment`.
    fn insert_into(&self, equipment: &mut Equipment) {
        let (id, nome, imagem) = (self.id(), self.nome.trim().to_string(), self.imagem());
        let weapon = || Weapon {
            nome: nome.clone(),
            tipo: self.tipo.clone(),
            dano: self.dano.clone(),
            capacidade: self.capacidade.clone(),
            cadencia: self.cadencia.clone(),
            id: id.clone(),
            imagem: imagem.clone(),
        };
        let cosmetic = || Cosmetic {
            nome: nome.clone(),
            warbond: self.warbond.clone(),
            id: id.clone(),
            imagem: imagem.clone(),
        };
        let described = || Described {
            nome: nome.clone(),
            descricao: self.descricao.clone(),
            id: id.clone(),
            imagem: imagem.clone(),
        };
        match self.category {
            Category::Primary => insert_sorted(&mut equipment.primary, weapon(), |w| &w.nome),
            Category::Secondary => insert_sorted(&mut equipment.secondary, weapon(), |w| &w.nome),
            Category::Grenade => insert_sorted(&mut equipment.grenade, weapon(), |w| &w.nome),
            Category::Armor => insert_sorted(
                &mut equipment.armor,
                Armor {
                    nome: nome.clone(),
                    peso: self.peso.clone(),
                    armor: self.armor,
                    speed: self.speed,
                    stamina: self.stamina,
                    passive: self.passive.clone(),
                    warbond: self.warbond.clone(),
                    id: id.clone(),
                    imagem: imagem.clone(),
                },
                |a| &a.nome,
            ),
            Category::Helmet => insert_sorted(&mut equipment.helmet, cosmetic(), |c| &c.nome),
            Category::Cape => insert_sorted(&mut equipment.cape, cosmetic(), |c| &c.nome),
            Category::Booster => insert_sorted(&mut equipment.booster, described(), |b| &b.nome),
            Category::Passive => insert_sorted(&mut equipment.passives, described(), |p| &p.nome),
        }
    }
}

/// As listas do `equipment.json` estão em ordem alfabética (é o que o
/// `scripts/sync-equipment.mjs` grava); o item novo entra no lugar dele, e não
/// no fim.
fn insert_sorted<T>(list: &mut Vec<T>, item: T, name: impl Fn(&T) -> &String) {
    let at = list.partition_point(|other| name(other) < name(&item));
    list.insert(at, item);
}

/// Id do item a partir do nome, como o `slug` do `scripts/sync-equipment.mjs`:
/// minúsculas sem acento, e o resto vira `-`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in data::normalize_text(name).chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Chave de comparação de nomes: sem caixa, acento, pontuação nem o
/// desambiguador de página da wiki. É o que faz `CPH-26 Commandant (Armor)`
/// casar com `CPH-26 Commandant`, e a warbond `Castellan’s Creed` (a API grava
/// assim nas capas) casar com `Castellan's Creed`. Igual ao `nameKey` do
/// `scripts/sync-equipment.mjs`.
fn name_key(name: &str) -> String {
    let name = name.trim();
    let name = match name.rfind(" (") {
        Some(at) if name.ends_with(')') => &name[..at],
        _ => name,
    };
    data::normalize_text(name)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// A categoria embarcada já tem o item: pelo nome, por um apelido ou pelo id.
fn is_known(bundled: &[(&str, &str)], item: &RemoteItem) -> bool {
    let id = item.id();
    let keys: Vec<String> = std::iter::once(&item.nome)
        .chain(&item.aliases)
        .map(|name| name_key(name))
        .collect();
    bundled
        .iter()
        .any(|(nome, other)| *other == id || keys.contains(&name_key(nome)))
}

// --- Leitura da API ---

#[derive(Debug, Deserialize)]
struct Envelope {
    data: Vec<serde_json::Value>,
}

/// Um item de qualquer coleção, só com o que o app lê. Tudo opcional: um campo
/// que falte derruba o item, não a lista.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiItem {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    upcoming: bool,
    #[serde(default)]
    image: Option<ApiImage>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    subcategory: Option<String>,
    #[serde(default)]
    stats_raw: HashMap<String, serde_json::Value>,
    #[serde(default)]
    weight: Option<String>,
    #[serde(default)]
    armor_rating: Option<f64>,
    #[serde(default)]
    speed: Option<f64>,
    #[serde(default)]
    stamina_regen: Option<f64>,
    #[serde(default)]
    passive_id: Option<String>,
    #[serde(default)]
    source: Option<ApiSource>,
    #[serde(default)]
    effect: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ApiSource {
    #[serde(default)]
    label: Option<String>,
}

impl ApiItem {
    /// Texto de uma linha da ficha crua da wiki (`Standard Damage`, `Capacity`).
    fn stat(&self, key: &str) -> Option<String> {
        match self.stats_raw.get(key)? {
            serde_json::Value::String(text) => Some(text.trim().to_string()),
            serde_json::Value::Number(number) => Some(number.to_string()),
            _ => None,
        }
        .filter(|text| !text.is_empty())
    }

    /// Warbond de onde o item sai, sem a página (`Halo: ODST P2` → `Halo: ODST`),
    /// como o scrape grava.
    fn warbond(&self) -> String {
        let label = self
            .source
            .as_ref()
            .and_then(|source| source.label.as_deref())
            .unwrap_or_default()
            .trim();
        match label.rsplit_once(" P") {
            Some((head, page)) if !page.is_empty() && page.bytes().all(|b| b.is_ascii_digit()) => {
                head.trim_end().to_string()
            }
            _ => label.to_string(),
        }
    }
}

fn number(value: Option<f64>) -> Option<i32> {
    value
        .filter(|value| value.is_finite() && (0.0..=1_000.0).contains(value))
        .map(|value| value.round() as i32)
}

/// Primeira letra maiúscula: `medium` → `Medium`, `standard` → `Standard`.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// O item da API no formato do app, ou `None` (item futuro, de outra
/// categoria ou com dado estranho). `passives` leva o id da passiva ao nome,
/// que é como a armadura a referencia.
fn from_api(
    collection: &str,
    item: ApiItem,
    passives: &HashMap<String, String>,
) -> Option<RemoteItem> {
    if item.upcoming {
        return None;
    }
    let category = match collection {
        "weapons" => match item.category.as_deref()? {
            "primary" => Category::Primary,
            "secondary" => Category::Secondary,
            "throwable" => Category::Grenade,
            // `civilian`: pá, facão e o que mais o jogo dá fora do loadout.
            _ => return None,
        },
        "armors" => Category::Armor,
        "helmets" => Category::Helmet,
        "capes" => Category::Cape,
        "boosters" => Category::Booster,
        "passives" => Category::Passive,
        _ => return None,
    };

    let mut remote = RemoteItem {
        category,
        slug: item.id.clone(),
        nome: item.name.clone()?.trim().to_string(),
        aliases: item.aliases.clone(),
        image: item.image.as_ref()?.url.clone(),
        tipo: String::new(),
        dano: String::new(),
        capacidade: None,
        cadencia: None,
        peso: String::new(),
        armor: 0,
        speed: 0,
        stamina: 0,
        passive: String::new(),
        warbond: String::new(),
        descricao: String::new(),
    };
    match category {
        Category::Grenade => {
            remote.tipo = capitalized(item.subcategory.as_deref().unwrap_or_default());
            remote.dano = item.stat("Damage").unwrap_or_default();
            remote.capacidade = item.stat("Capacity");
        }
        Category::Primary | Category::Secondary => {
            remote.tipo = item.stat("Weapon Type").unwrap_or_default();
            remote.dano = item.stat("Standard Damage").unwrap_or_default();
            remote.capacidade = item.stat("Capacity");
            remote.cadencia = item.stat("Fire Rate");
        }
        Category::Armor => {
            remote.peso = capitalized(item.weight.as_deref()?);
            remote.armor = number(item.armor_rating)?;
            remote.speed = number(item.speed)?;
            remote.stamina = number(item.stamina_regen)?;
            remote.passive = passives.get(item.passive_id.as_deref()?)?.clone();
            remote.warbond = item.warbond();
        }
        Category::Helmet | Category::Cape => remote.warbond = item.warbond(),
        Category::Booster => {
            remote.descricao = item
                .effect
                .clone()
                .or_else(|| item.description.clone())
                .unwrap_or_default();
        }
        Category::Passive => remote.descricao = item.description.clone().unwrap_or_default(),
    }
    remote.is_valid().then_some(remote)
}

/// Uma coleção da API reduzida aos itens válidos.
pub fn parse_collection(
    collection: &str,
    bytes: &[u8],
    passives: &HashMap<String, String>,
) -> Result<Vec<RemoteItem>> {
    let envelope: Envelope = serde_json::from_slice(bytes)
        .with_context(|| format!("{collection}: resposta da API em formato inesperado"))?;
    let mut list = Vec::new();
    for value in envelope.data {
        match serde_json::from_value::<ApiItem>(value) {
            Ok(item) => {
                let id = item.id.clone();
                let future = item.upcoming;
                match from_api(collection, item, passives) {
                    Some(remote) => list.push(remote),
                    None if !future => {
                        log::debug!("{collection}: item fora do app ou recusado: {id}")
                    }
                    None => {}
                }
            }
            Err(err) => log::warn!("{collection}: item ilegível ({err})"),
        }
    }
    Ok(list)
}

/// Os itens que o JSON embarcado não tem, prontos para entrar nele.
///
/// Armadura com passiva que o app não conhece (nem embarcada, nem nova) fica de
/// fora. Warbond que o JSON já tem mantém a grafia de lá: é por igualdade de
/// texto que a capa casa com a armadura no sorteio de set.
pub fn fresh_items(bundled: &Equipment, api: &[RemoteItem]) -> Vec<RemoteItem> {
    let mut out: Vec<RemoteItem> = Vec::new();
    for category in Category::ALL {
        let known = category.bundled(bundled);
        let mut ids: HashSet<String> = known.iter().map(|(_, id)| id.to_string()).collect();
        let new: Vec<&RemoteItem> = api
            .iter()
            .filter(|item| item.category == category && !is_known(&known, item))
            .filter(|item| ids.insert(item.id()))
            .collect();
        if new.len() > MAX_NEW_PER_CATEGORY {
            log::warn!(
                "a API trouxe {} {} novos de uma vez (teto {MAX_NEW_PER_CATEGORY}); nenhum aplicado",
                new.len(),
                category.key()
            );
            continue;
        }
        out.extend(new.into_iter().cloned());
    }

    let warbonds: HashMap<String, String> = bundled
        .armor
        .iter()
        .map(|armor| &armor.warbond)
        .chain(
            bundled
                .helmet
                .iter()
                .chain(&bundled.cape)
                .map(|item| &item.warbond),
        )
        .filter(|name| !name.is_empty())
        .map(|name| (name_key(name), name.clone()))
        .collect();
    let mut passives: HashMap<String, String> = bundled
        .passives
        .iter()
        .map(|passive| (name_key(&passive.nome), passive.nome.clone()))
        .collect();
    for item in out.iter().filter(|item| item.category == Category::Passive) {
        passives
            .entry(name_key(&item.nome))
            .or_insert_with(|| item.nome.clone());
    }

    out.retain_mut(|item| {
        if let Some(name) = warbonds.get(&name_key(&item.warbond)) {
            item.warbond = name.clone();
        }
        if item.category != Category::Armor {
            return true;
        }
        match passives.get(&name_key(&item.passive)) {
            Some(name) => {
                item.passive = name.clone();
                true
            }
            None => {
                log::warn!(
                    "armadura {} ignorada: passiva desconhecida {}",
                    item.nome,
                    item.passive
                );
                false
            }
        }
    });
    out
}

/// O JSON embarcado com os itens novos da última sincronização. O que um
/// release já trouxe (o item entrou no `equipment.json`) é pulado.
pub fn merge(equipment: &mut Equipment, remote: &[RemoteItem]) {
    for item in fresh_items(equipment, remote) {
        item.insert_into(equipment);
    }
}

// --- Cache ---

#[derive(Debug, Serialize, Deserialize)]
struct Cache {
    version: u32,
    items: Vec<RemoteItem>,
}

/// Itens novos da última sincronização, ou nenhum (sem arquivo, formato velho,
/// dado adulterado).
pub fn load_cache() -> Vec<RemoteItem> {
    read_cache(&util::config_path(CACHE_FILE))
}

fn read_cache(path: &Path) -> Vec<RemoteItem> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let cache: Cache = match serde_json::from_slice(&bytes) {
        Ok(cache) => cache,
        Err(err) => {
            log::warn!("{} ilegível ({err}); ignorado", path.display());
            return Vec::new();
        }
    };
    if cache.version != CACHE_VERSION {
        return Vec::new();
    }
    cache
        .items
        .into_iter()
        .filter(RemoteItem::is_valid)
        .collect()
}

fn write_cache(path: &Path, items: &[RemoteItem]) -> Result<()> {
    let cache = Cache {
        version: CACHE_VERSION,
        items: items.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&cache).context("falha ao serializar o cache")?;
    util::write_atomic(path, &json)
}

// --- Worker ---

/// Baixa as coleções, garante os ícones dos itens novos e grava o cache.
/// Devolve quantos itens a API tem que o JSON embarcado não tem. Chamada pelo
/// worker do [`data_sync`], que já cuida de sessão e foco.
pub fn sync(agent: &ureq::Agent) -> Result<usize> {
    let bundled = data::bundled_equipment().context("equipment.json embarcado indisponível")?;

    let mut passives: HashMap<String, String> = HashMap::new();
    let mut api = Vec::new();
    for collection in COLLECTIONS {
        let bytes = data_sync::get_bytes(agent, &format!("/v1/{collection}.json"), MAX_LIST_BYTES)?;
        let items = parse_collection(collection, &bytes, &passives)?;
        if collection == "passives" {
            passives = items
                .iter()
                .map(|item| (item.slug.clone(), item.nome.clone()))
                .collect();
        }
        api.extend(items);
    }
    let fresh = fresh_items(&bundled, &api);

    let dir = util::config_dir()
        .join(util::DOWNLOADED_ICONS_DIR)
        .join(ICON_DIR);
    for item in &fresh {
        let path = dir.join(item.icon_file());
        if path.exists() {
            continue;
        }
        // Sem ícone o item ainda funciona (o card sai com o marcador vazio).
        if let Err(err) = data_sync::download_image(agent, &item.image, &path) {
            log::warn!("ícone de {} não baixou: {err:#}", item.nome);
        }
    }

    write_cache(&util::config_path(CACHE_FILE), &fresh)?;
    prune_icons(&dir, &fresh);
    Ok(fresh.len())
}

/// Apaga os ícones que nenhum item novo usa mais: os que entraram no JSON
/// embarcado num release e os de uma versão anterior da arte.
fn prune_icons(dir: &Path, fresh: &[RemoteItem]) {
    let keep: HashSet<String> = fresh.iter().map(RemoteItem::icon_file).collect();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !keep.contains(entry.file_name().to_string_lossy().as_ref()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::EquipSlot;

    fn bundled() -> Equipment {
        data::bundled_equipment().expect("equipment.json do repositório")
    }

    fn envelope(items: &[serde_json::Value]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({ "meta": {}, "data": items })).unwrap()
    }

    fn image(collection: &str, slug: &str) -> serde_json::Value {
        serde_json::json!({ "url": format!("/images/v1/{collection}/{slug}.0123abcd.webp") })
    }

    fn weapon(slug: &str, name: &str, category: &str) -> serde_json::Value {
        serde_json::json!({
            "id": slug,
            "name": name,
            "category": category,
            "subcategory": "assault_rifle",
            "upcoming": false,
            "image": image("weapons", slug),
            "statsRaw": {
                "Weapon Type": "Assault Rifles",
                "Standard Damage": "70 Ballistic",
                "Capacity": "45",
                "Fire Rate": "700 rpm",
            },
        })
    }

    fn passive(slug: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": slug,
            "name": name,
            "description": format!("{name} faz algo."),
            "image": image("passives", slug),
        })
    }

    fn armor(slug: &str, name: &str, passive_id: &str, label: &str) -> serde_json::Value {
        serde_json::json!({
            "id": slug,
            "name": name,
            "weight": "heavy",
            "armorRating": 150,
            "speed": 450,
            "staminaRegen": 50,
            "passiveId": passive_id,
            "source": { "type": "warbond", "label": label },
            "image": image("armors", slug),
        })
    }

    fn cosmetic(collection: &str, slug: &str, name: &str, label: &str) -> serde_json::Value {
        serde_json::json!({
            "id": slug,
            "name": name,
            "source": { "type": "warbond", "label": label },
            "image": image(collection, slug),
        })
    }

    /// As seis coleções como a API responde, com itens novos, conhecidos e
    /// casos de borda.
    fn api() -> Vec<RemoteItem> {
        let passives = parse_collection(
            "passives",
            &envelope(&[
                passive("passiva-teste", "Passiva Teste"),
                passive("siege-ready", "Siege-Ready"),
            ]),
            &HashMap::new(),
        )
        .unwrap();
        let by_id: HashMap<String, String> = passives
            .iter()
            .map(|item| (item.slug.clone(), item.nome.clone()))
            .collect();

        let mut future = weapon("ar-98-futuro", "AR-98 Futuro", "primary");
        future["upcoming"] = serde_json::json!(true);
        let mut all = passives;
        all.extend(
            parse_collection(
                "weapons",
                &envelope(&[
                    weapon("ar-99-teste", "AR-99 Teste", "primary"),
                    weapon("ar-23-liberator", "AR-23 Liberator", "primary"),
                    weapon("cqc-72-entrenchment-tool", "Trench Shovel", "civilian"),
                    future,
                    serde_json::json!({
                        "id": "g-99-teste",
                        "name": "G-99 Teste",
                        "category": "throwable",
                        "subcategory": "special",
                        "image": image("weapons", "g-99-teste"),
                        "statsRaw": { "Fuse Time": "Impact" },
                    }),
                ]),
                &by_id,
            )
            .unwrap(),
        );
        all.extend(
            parse_collection(
                "armors",
                &envelope(&[
                    armor(
                        "bfm-999-teste",
                        "BFM-999 Teste",
                        "passiva-teste",
                        "Battle-Hardened P2",
                    ),
                    armor(
                        "tg-99-teste",
                        "TG-99 Teste",
                        "siege-ready",
                        "Castellan\u{2019}s Creed P1",
                    ),
                    // A wiki desambigua a página com "(Armor)"; o JSON embarcado não.
                    armor(
                        "cph-26-commandant",
                        "CPH-26 Commandant (Armor)",
                        "siege-ready",
                        "Masters of Ceremony",
                    ),
                    armor("sem-passiva", "SP-1 Sem Passiva", "nao-existe", "x"),
                ]),
                &by_id,
            )
            .unwrap(),
        );
        all.extend(
            parse_collection(
                "capes",
                &envelope(&[cosmetic(
                    "capes",
                    "capa-teste",
                    "Capa Teste",
                    "Castellan\u{2019}s Creed P3",
                )]),
                &by_id,
            )
            .unwrap(),
        );
        all
    }

    fn find<'a>(items: &'a [RemoteItem], name: &str) -> Option<&'a RemoteItem> {
        items.iter().find(|item| item.nome == name)
    }

    #[test]
    fn only_what_the_bundled_json_lacks_is_new() {
        let fresh = fresh_items(&bundled(), &api());
        let names: Vec<&str> = fresh.iter().map(|item| item.nome.as_str()).collect();

        assert!(names.contains(&"AR-99 Teste"));
        assert!(names.contains(&"G-99 Teste"));
        assert!(names.contains(&"BFM-999 Teste"));
        assert!(names.contains(&"Passiva Teste"));
        assert!(names.contains(&"Capa Teste"));
        // Já embarcados, inclusive com o desambiguador da wiki no nome.
        assert!(!names.contains(&"AR-23 Liberator"));
        assert!(!names.contains(&"CPH-26 Commandant (Armor)"));
        assert!(!names.contains(&"Siege-Ready"));
        // Fora do loadout, futuro e armadura com passiva que ninguém conhece.
        assert!(!names.contains(&"Trench Shovel"));
        assert!(!names.contains(&"AR-98 Futuro"));
        assert!(!names.contains(&"SP-1 Sem Passiva"));
    }

    #[test]
    fn a_new_item_carries_the_same_sheet_the_scrape_would_write() {
        let fresh = fresh_items(&bundled(), &api());

        let rifle = find(&fresh, "AR-99 Teste").unwrap();
        assert_eq!(rifle.id(), "primary-ar-99-teste");
        assert_eq!(rifle.tipo, "Assault Rifles");
        assert_eq!(rifle.dano, "70 Ballistic");
        assert_eq!(rifle.capacidade.as_deref(), Some("45"));
        assert_eq!(rifle.cadencia.as_deref(), Some("700 rpm"));

        let grenade = find(&fresh, "G-99 Teste").unwrap();
        assert_eq!(grenade.id(), "grenade-g-99-teste");
        assert_eq!(grenade.tipo, "Special");
        assert_eq!(grenade.dano, "");

        let tanker = find(&fresh, "BFM-999 Teste").unwrap();
        assert_eq!(tanker.peso, "Heavy");
        assert_eq!((tanker.armor, tanker.speed, tanker.stamina), (150, 450, 50));
        assert_eq!(tanker.passive, "Passiva Teste");
        // Sem a página da warbond, como o script grava.
        assert_eq!(tanker.warbond, "Battle-Hardened");
        assert_eq!(
            tanker.imagem(),
            "remote/equipment/armor-bfm-999-teste.0123abcd.webp"
        );

        let passive = find(&fresh, "Passiva Teste").unwrap();
        assert_eq!(passive.id(), "passives-passiva-teste");
        assert_eq!(passive.descricao, "Passiva Teste faz algo.");
    }

    #[test]
    fn a_known_warbond_keeps_the_bundled_spelling() {
        // A API grava a Castellan's Creed com o apóstrofo curvo nas capas e
        // reto nas armaduras. A capa só casa com a armadura com o mesmo texto,
        // então vale a grafia do JSON embarcado.
        let bundled = bundled();
        let spelling = bundled
            .armor
            .iter()
            .find(|armor| armor.warbond.starts_with("Castellan"))
            .map(|armor| armor.warbond.clone())
            .expect("armadura da Castellan's Creed");
        let fresh = fresh_items(&bundled, &api());
        assert_eq!(find(&fresh, "TG-99 Teste").unwrap().warbond, spelling);
        assert_eq!(find(&fresh, "Capa Teste").unwrap().warbond, spelling);
        // E a passiva conhecida fica com a grafia de lá.
        assert_eq!(find(&fresh, "TG-99 Teste").unwrap().passive, "Siege-Ready");
    }

    #[test]
    fn merging_puts_each_item_in_its_alphabetical_place() {
        let mut equipment = bundled();
        let before = equipment.primary.len();
        merge(&mut equipment, &api());

        assert_eq!(equipment.primary.len(), before + 1);
        let names: Vec<&str> = equipment.primary.iter().map(|w| w.nome.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);

        let rifle = equipment
            .find(EquipSlot::Primary, "primary-ar-99-teste")
            .expect("arma nova no equipamento");
        assert_eq!(
            rifle.imagem(),
            "remote/equipment/primary-ar-99-teste.0123abcd.webp"
        );
        // A armadura nova resolve a passiva nova.
        let tanker = equipment
            .armor
            .iter()
            .find(|armor| armor.nome == "BFM-999 Teste")
            .unwrap();
        assert!(equipment.passive(&tanker.passive).is_some());

        // O que um release trouxe depois não entra duas vezes.
        let again = equipment.clone();
        merge(&mut equipment, &api());
        assert_eq!(equipment, again);
    }

    #[test]
    fn downloaded_icons_resolve_to_their_own_folder() {
        let item = find(&fresh_items(&bundled(), &api()), "AR-99 Teste")
            .unwrap()
            .clone();
        let path = util::resource_path(&format!("icons/{}", item.imagem()));
        assert!(path.starts_with(util::config_dir()));
        assert!(path.ends_with("remote-icons/equipment/primary-ar-99-teste.0123abcd.webp"));
    }

    #[test]
    fn api_items_are_validated() {
        let base = find(&api(), "AR-99 Teste").unwrap().clone();
        assert!(base.is_valid());
        let broken = |f: fn(&mut RemoteItem)| {
            let mut item = base.clone();
            f(&mut item);
            item.is_valid()
        };
        assert!(!broken(|i| i.slug = "../x".into()));
        assert!(!broken(|i| i.nome = "  ".into()));
        assert!(!broken(|i| i.nome = "x".repeat(200)));
        assert!(!broken(|i| i.nome = "a\u{0}b".into()));
        assert!(!broken(
            |i| i.image = "/images/v1/armors/x.0123abcd.webp".into()
        ));
        assert!(!broken(
            |i| i.image = "/images/v1/weapons/../../x.webp".into()
        ));
        assert!(!broken(|i| i.image = "/images/v1/weapons/x.png".into()));
        assert!(!broken(|i| i.dano = "x".repeat(1_000)));
        assert!(!broken(|i| i.armor = -1));
        assert!(!broken(|i| {
            i.category = Category::Armor;
            i.image = "/images/v1/armors/x.0123abcd.webp".into();
            i.peso = "Enorme".into();
            i.passive = "Siege-Ready".into();
        }));
    }

    #[test]
    fn a_flood_of_new_items_is_refused_whole() {
        let items: Vec<serde_json::Value> = (0..MAX_NEW_PER_CATEGORY + 1)
            .map(|n| weapon(&format!("zz-{n}"), &format!("ZZ-{n} Quebrada"), "secondary"))
            .collect();
        let api = parse_collection("weapons", &envelope(&items), &HashMap::new()).unwrap();
        assert_eq!(api.len(), MAX_NEW_PER_CATEGORY + 1);
        assert!(fresh_items(&bundled(), &api).is_empty());
        // No limite, todos entram.
        assert_eq!(
            fresh_items(&bundled(), &api[..MAX_NEW_PER_CATEGORY]).len(),
            MAX_NEW_PER_CATEGORY
        );
    }

    #[test]
    fn a_broken_item_does_not_sink_the_collection() {
        let items = [
            weapon("ar-99-teste", "AR-99 Teste", "primary"),
            serde_json::json!({ "id": 7 }),
            serde_json::json!({ "id": "sem-nome", "category": "primary" }),
        ];
        let list = parse_collection("weapons", &envelope(&items), &HashMap::new()).unwrap();
        assert_eq!(list.len(), 1);
        assert!(parse_collection("weapons", b"{}", &HashMap::new()).is_err());
    }

    #[test]
    fn the_cache_round_trips_and_rejects_tampering() {
        let dir = std::env::temp_dir().join(format!("mh2-equip-cache-{}", std::process::id()));
        let path = dir.join(CACHE_FILE);
        let fresh = fresh_items(&bundled(), &api());
        write_cache(&path, &fresh).unwrap();
        assert_eq!(read_cache(&path), fresh);

        // Item adulterado sai; os outros ficam.
        let mut tampered = fresh.clone();
        tampered[0].image = "/images/v1/passives/../../../x.webp".into();
        write_cache(&path, &tampered).unwrap();
        assert_eq!(read_cache(&path), fresh[1..].to_vec());

        // Versão de outro formato é ignorada inteira.
        let json = serde_json::json!({ "version": CACHE_VERSION + 1, "items": fresh });
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        assert!(read_cache(&path).is_empty());
        assert!(read_cache(&dir.join("nao-existe.json")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slugs_and_name_keys_follow_the_scrape() {
        assert_eq!(slug("AR-23 Liberator"), "ar-23-liberator");
        assert_eq!(slug("A/G-16 Gatling"), "a-g-16-gatling");
        assert_eq!(
            slug("Cloak of Posterity's Gratitude"),
            "cloak-of-posterity-s-gratitude"
        );
        assert_eq!(
            name_key("CPH-26 Commandant (Armor)"),
            name_key("CPH-26 Commandant")
        );
        assert_eq!(
            name_key("Castellan\u{2019}s Creed"),
            name_key("Castellan's Creed")
        );
        assert_ne!(name_key("B-01 Tactical"), name_key("B-08 Light Gunner"));
    }

    #[test]
    #[ignore = "depende de rede"]
    fn the_live_api_serves_every_collection() {
        let agent = util::http_agent(Some(std::time::Duration::from_secs(30)));
        let bundled = bundled();
        let mut passives = HashMap::new();
        let mut api = Vec::new();
        for collection in COLLECTIONS {
            let bytes =
                data_sync::get_bytes(&agent, &format!("/v1/{collection}.json"), MAX_LIST_BYTES)
                    .expect("coleção da API");
            let items = parse_collection(collection, &bytes, &passives).unwrap();
            assert!(!items.is_empty(), "{collection} vazia");
            if collection == "passives" {
                passives = items
                    .iter()
                    .map(|item| (item.slug.clone(), item.nome.clone()))
                    .collect();
            }
            api.extend(items);
        }
        // O JSON embarcado sai da mesma API: fora o que saiu depois do último
        // `npm run scrape`, nada é novo. A regra de nomes não pode transformar
        // o arsenal inteiro em "novo".
        let fresh = fresh_items(&bundled, &api);
        assert!(fresh.len() < 20, "{} itens novos", fresh.len());

        // O ícone de cada item novo baixa e decodifica: arma vem em outro
        // tamanho que o dos ícones de 256px.
        let dir = std::env::temp_dir().join(format!("mh2-equip-icons-{}", std::process::id()));
        for item in &fresh {
            eprintln!("novo: {:?} {}", item.category, item.nome);
            let path = dir.join(item.icon_file());
            data_sync::download_image(&agent, &item.image, &path).expect("ícone da API");
            let decoded = crate::gfx::images::decode(&path).unwrap();
            assert!(decoded.width.max(decoded.height) <= crate::gfx::images::MAX_EDGE_PX);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
