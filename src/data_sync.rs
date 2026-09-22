//! Estratagemas novos sem release: o app confere a API de dados
//! (helldivers-api.dionatha.com.br, alimentada pela wiki) e acrescenta o que o
//! JSON embarcado ainda não tem.
//!
//! O JSON embarcado continua mandando na ordem curada igual à do jogo, nos nomes
//! e nas tags. Da API entram duas coisas:
//!
//! - estratagema novo, no fim do subgrupo dele ([`group_of`]): perto de onde o
//!   jogo o mostra, não exatamente lá. O próximo release traz a posição certa;
//! - codex trocado num patch ([`update_codexes`]): sem isso o macro digitaria a
//!   sequência velha até o próximo release.
//!
//! O ciclo segue as regras do updater: um check por sessão, nunca com o jogo em
//! foco, rede num worker efêmero. O resultado vale a partir do próximo boot: o
//! `GameData` é um `Arc` imutável dividido por janela, overlay e hooks, e trocar
//! a lista no meio da sessão mudaria os atalhos debaixo do hook.
//!
//! O codex vira `SendInput`, então tudo o que vem de fora é validado: na
//! chegada e de novo ao ler o cache, que fica numa pasta gravável pelo usuário.
//!
//! O mesmo worker traz o equipamento novo ([`crate::equipment_sync`]).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::data::{Dir, StratKind, Stratagem, SUPPORT_STRATS};
use crate::shared::Shared;
use crate::util;

/// Raiz da API de dados.
pub const API_BASE: &str = "https://helldivers-api.dionatha.com.br";

/// Lista de estratagemas: arquivo estático no CDN, sem limite de requisição.
const STRATAGEMS_PATH: &str = "/v1/stratagems.json";

/// De onde saem as imagens; qualquer outro caminho é recusado.
const IMAGES_PATH: &str = "/images/v1/stratagems/";

/// Cache em `config_dir` com a última lista válida.
pub const CACHE_FILE: &str = "stratagems-remote.json";

/// Versão do formato do cache: um app mais novo ignora o arquivo de um mais
/// velho em vez de interpretá-lo errado.
const CACHE_VERSION: u32 = 1;

/// Prefixo do `imagem` dos estratagemas baixados, relativo a `assets/icons/`.
const ICON_PREFIX: &str = "remote/";

/// A lista pesa ~800 KB crus (~70 KB com gzip); o prazo cobre conexão lenta.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Teto do corpo da lista e de cada ícone: defesa contra resposta sem fim.
const MAX_LIST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ICON_BYTES: u64 = 1024 * 1024;

/// Menos que isto e a API está quebrada, não o jogo encolhendo: a lista inteira
/// é recusada. É a mesma trava do `scripts/sync-stratagems.mjs`.
pub const MIN_LOADOUT: usize = 80;

/// O maior codex do jogo tem 8 passos; acima disto é dado corrompido.
const MAX_CODEX: usize = 12;

/// Teto de codex trocados numa sincronização. Um patch mexe em um ou dois;
/// muitos de uma vez é a API quebrada (parser, formato novo), e aí nenhuma troca
/// vale.
const MAX_CODEX_CHANGES: usize = 5;

pub(crate) const USER_AGENT: &str = concat!("macro-helldivers2/", env!("CARGO_PKG_VERSION"));

/// Um estratagema de loadout vindo da API, já validado. É também o formato do
/// cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteStratagem {
    pub slug: String,
    pub nome: String,
    /// `Offensive` | `Supply` | `Defensive` (o mesmo texto da tag embarcada).
    pub permit: String,
    /// `orbital`, `eagle`, `support_weapon`, `backpack`, `vehicle`, `sentry`,
    /// `emplacement`…
    pub kind: String,
    #[serde(default)]
    pub traits: Vec<String>,
    pub codex: Vec<Dir>,
    /// Caminho da imagem na API (`/images/v1/stratagems/<slug>.<hash>.webp`).
    pub image: String,
}

impl RemoteStratagem {
    /// Nome do arquivo do ícone. Vem com hash de conteúdo, então um ícone
    /// trocado na wiki chega com outro nome e nunca reaproveita o velho.
    pub fn icon_file(&self) -> &str {
        self.image.strip_prefix(IMAGES_PATH).unwrap_or_default()
    }

    fn has_trait(&self, name: &str) -> bool {
        self.traits.iter().any(|t| t == name)
    }

    fn slug_has_word(&self, prefix: &str) -> bool {
        self.slug.split('-').any(|word| word.starts_with(prefix))
    }

    /// Confere tudo o que o app usa. O cache passa por aqui de novo na leitura.
    fn is_valid(&self) -> bool {
        valid_slug(&self.slug)
            && !self.nome.trim().is_empty()
            && self.nome.chars().count() <= 80
            && permit_tag(&self.permit.to_ascii_lowercase()) == Some(self.permit.as_str())
            && (1..=MAX_CODEX).contains(&self.codex.len())
            && valid_icon_file(self.icon_file())
            && self.image.starts_with(IMAGES_PATH)
    }

    /// O estratagema como o resto do app o enxerga.
    fn to_stratagem(&self, id: u32) -> Stratagem {
        let mut tag = vec![self.permit.clone()];
        // Só um veículo e só um exo por loadout: é a mesma regra que o sync
        // usa para entrada nova (as antigas têm curadoria manual).
        if self.kind == "vehicle" {
            tag.push(
                if self.slug_has_word("exosuit") {
                    "Mecha"
                } else {
                    "Vehicle"
                }
                .to_string(),
            );
        }
        Stratagem {
            id,
            slug: self.slug.clone(),
            nome: self.nome.trim().to_string(),
            imagem: format!("{ICON_PREFIX}{}", self.icon_file()),
            tag,
            codex: self.codex.clone(),
            kind_hint: Some(StratKind {
                support: self.kind == "support_weapon",
                // Arma com mochila (Autocannon, Recoilless…) também ocupa o slot.
                backpack: self.kind == "backpack" || self.has_trait("backpack"),
                sentry: self.kind == "sentry",
            }),
        }
    }
}

/// Subgrupo do estratagema dentro da seção da cor, como o jogo agrupa.
///
/// A ordem do jogo não está na wiki nem na API; o que dá para derivar é o
/// agrupamento: cor → tipo → (nas armas de apoio) corpo a corpo, comuns,
/// descartáveis e as que vêm com mochila. Dentro do subgrupo a ordem é da
/// Arrowhead e só existe no JSON embarcado.
pub fn group_of(remote: &RemoteStratagem) -> String {
    let family = match remote.kind.as_str() {
        "support_weapon" if remote.has_trait("melee") => "melee",
        "support_weapon" if remote.has_trait("expendable") => "expendable",
        "support_weapon" if remote.has_trait("backpack") => "backpack_weapon",
        "support_weapon" => "weapon",
        "vehicle" if remote.slug_has_word("exosuit") => "exosuit",
        "emplacement" if remote.slug_has_word("mine") => "mines",
        other => other,
    };
    format!("{}/{family}", remote.permit)
}

/// Id numérico de um estratagema que chegou pela API.
///
/// Derivado do slug (FNV-1a de 32 bits com o bit alto ligado), para dar o mesmo
/// número em toda instalação (backup exportado de um PC abre no outro) e para
/// o `scripts/sync-stratagems.mjs` gravar esse mesmo id quando o estratagema
/// entrar no JSON embarcado: slot salvo antes do release continua valendo
/// depois dele. O bit alto separa estes dos ids curados, que são pequenos.
pub fn stable_id(slug: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in slug.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash | 0x8000_0000
}

/// O JSON embarcado com os codex que a API trocou e os estratagemas que ele
/// não tem.
///
/// Cada novo entra no fim da primeira sequência do seu subgrupo. A primeira,
/// porque o jogo tem exceções no meio (a Tesla Tower é sentinela e fica entre
/// as emplacements). Subgrupo que ainda não existe vai para o fim da seção da
/// cor.
pub fn merge(mut bundled: Vec<Stratagem>, remote: &[RemoteStratagem]) -> Vec<Stratagem> {
    let by_slug: HashMap<&str, &RemoteStratagem> =
        remote.iter().map(|r| (r.slug.as_str(), r)).collect();
    update_codexes(&mut bundled, &by_slug);

    let mut groups: Vec<Option<String>> = bundled
        .iter()
        .map(|s| by_slug.get(s.slug.as_str()).map(|r| group_of(r)))
        .collect();
    let mut slugs: HashSet<String> = bundled.iter().map(|s| s.slug.clone()).collect();
    let mut ids: HashSet<u32> = bundled.iter().map(|s| s.id).collect();
    let mut out = bundled;

    for item in remote {
        if !slugs.insert(item.slug.clone()) {
            continue;
        }
        if clashes(&item.codex, &out, None) {
            log::warn!(
                "estratagema {} ignorado: codex em conflito com outro",
                item.slug
            );
            continue;
        }
        let id = stable_id(&item.slug);
        if !ids.insert(id) {
            log::warn!("estratagema {} ignorado: id {id} já está em uso", item.slug);
            continue;
        }
        let group = group_of(item);
        let at = insertion_point(&groups, &out, &group, &item.permit);
        out.insert(at, item.to_stratagem(id));
        groups.insert(at, Some(group));
    }
    out
}

/// Troca o codex dos estratagemas que a API diz ter mudado.
///
/// A trava é a regra que o jogo segue: nenhum codex igual a outro nem começando
/// com outro, senão o menor dispararia no meio do maior. As trocas são
/// conferidas juntas, contra a lista já trocada (dois estratagemas que trocam
/// de codex entre si passam); a que conflita volta ao codex embarcado, e a
/// conferência se repete até nada mais conflitar.
pub fn update_codexes(list: &mut [Stratagem], by_slug: &HashMap<&str, &RemoteStratagem>) {
    let mut changed: Vec<(usize, Vec<Dir>)> = list
        .iter()
        .enumerate()
        .filter_map(|(i, strat)| {
            let remote = by_slug.get(strat.slug.as_str())?;
            (remote.codex != strat.codex).then(|| (i, strat.codex.clone()))
        })
        .collect();
    if changed.is_empty() {
        return;
    }
    if changed.len() > MAX_CODEX_CHANGES {
        log::warn!(
            "a API trocou {} codex de uma vez (teto {MAX_CODEX_CHANGES}); nenhum aplicado",
            changed.len()
        );
        return;
    }

    for (i, _) in &changed {
        list[*i].codex = by_slug[list[*i].slug.as_str()].codex.clone();
    }
    loop {
        let clashing: Vec<usize> = changed
            .iter()
            .map(|(i, _)| *i)
            .filter(|&i| clashes(&list[i].codex, list, Some(i)))
            .collect();
        if clashing.is_empty() {
            break;
        }
        changed.retain(|(i, old)| {
            if !clashing.contains(i) {
                return true;
            }
            log::warn!(
                "codex novo de {} recusado: conflita com outro",
                list[*i].nome
            );
            list[*i].codex = old.clone();
            false
        });
    }
    for (i, old) in &changed {
        log::info!(
            "codex de {} atualizado pela API: {old:?} -> {:?}",
            list[*i].nome,
            list[*i].codex
        );
    }
}

/// O codex é igual a outro da lista (fora o próprio, em `skip`) ou a um dos
/// apoios fixos, ou um deles começa com o outro.
fn clashes(codex: &[Dir], list: &[Stratagem], skip: Option<usize>) -> bool {
    let clash = |other: &[Dir]| other.starts_with(codex) || codex.starts_with(other);
    list.iter()
        .enumerate()
        .any(|(i, strat)| Some(i) != skip && clash(&strat.codex))
        || SUPPORT_STRATS.iter().any(|support| clash(support.codex))
}

fn insertion_point(
    groups: &[Option<String>],
    list: &[Stratagem],
    group: &str,
    permit: &str,
) -> usize {
    let same = |i: usize| groups[i].as_deref() == Some(group);
    if let Some(start) = (0..groups.len()).find(|&i| same(i)) {
        let mut end = start;
        while end + 1 < groups.len() && same(end + 1) {
            end += 1;
        }
        return end + 1;
    }
    list.iter()
        .rposition(|s| s.primary_tag() == Some(permit))
        .map_or(list.len(), |last| last + 1)
}

// --- Validação ---

/// Formato que a API usa para ids. Ele vira parte de caminho no disco (nome do
/// ícone), então nada de `/`, `\`, `.` ou maiúsculas.
pub(crate) fn valid_slug(slug: &str) -> bool {
    (1..=80).contains(&slug.len())
        && !slug.starts_with('-')
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub(crate) fn valid_icon_file(file: &str) -> bool {
    let Some(stem) = file.strip_suffix(".webp") else {
        return false;
    };
    (1..=120).contains(&stem.len())
        && !stem.starts_with('.')
        && !stem.contains("..")
        && stem
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
}

/// `offensive` → `Offensive`; o resto (missão, objetivo) não é de loadout.
fn permit_tag(permit: &str) -> Option<&'static str> {
    match permit {
        "offensive" => Some("Offensive"),
        "supply" => Some("Supply"),
        "defensive" => Some("Defensive"),
        _ => None,
    }
}

fn parse_dir(step: &str) -> Option<Dir> {
    match step {
        "up" => Some(Dir::Up),
        "down" => Some(Dir::Down),
        "left" => Some(Dir::Left),
        "right" => Some(Dir::Right),
        _ => None,
    }
}

/// Um item da API, só com o que o app lê. Tudo opcional: um campo que falte
/// derruba o item, não a lista.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiStratagem {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    permit_type: Option<String>,
    #[serde(default)]
    availability: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    trait_ids: Option<Vec<String>>,
    #[serde(default)]
    code: Option<Vec<String>>,
    #[serde(default)]
    image: Option<ApiImage>,
}

#[derive(Debug, Deserialize)]
struct ApiImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(default)]
    meta: Option<ApiMeta>,
    data: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMeta {
    #[serde(default)]
    data_version: Option<String>,
}

/// Item de loadout válido, ou `None` (fora do loadout ou com dado estranho).
fn from_api(item: ApiStratagem) -> Option<RemoteStratagem> {
    if item.availability.as_deref() != Some("loadout") {
        return None;
    }
    let codex = item
        .code?
        .iter()
        .map(|step| parse_dir(step))
        .collect::<Option<Vec<Dir>>>()?;
    let remote = RemoteStratagem {
        permit: permit_tag(item.permit_type.as_deref()?)?.to_string(),
        slug: item.id,
        nome: item.name?,
        kind: item.kind?,
        traits: item.trait_ids.unwrap_or_default(),
        codex,
        image: item.image?.url,
    };
    remote.is_valid().then_some(remote)
}

/// A resposta da API reduzida aos estratagemas de loadout válidos.
pub fn parse_api(bytes: &[u8]) -> Result<(Vec<RemoteStratagem>, Option<String>)> {
    let envelope: Envelope =
        serde_json::from_slice(bytes).context("resposta da API em formato inesperado")?;
    let mut list = Vec::new();
    for value in envelope.data {
        let id = value
            .get("id")
            .and_then(|id| id.as_str())
            .map(str::to_owned);
        match serde_json::from_value::<ApiStratagem>(value) {
            Ok(item) => {
                let loadout = item.availability.as_deref() == Some("loadout");
                match from_api(item) {
                    Some(remote) => list.push(remote),
                    None if loadout => log::warn!("estratagema recusado: {id:?}"),
                    None => {}
                }
            }
            Err(err) => log::warn!("estratagema ilegível ({id:?}): {err}"),
        }
    }
    if list.len() < MIN_LOADOUT {
        anyhow::bail!(
            "só {} estratagemas de loadout válidos (mínimo {MIN_LOADOUT})",
            list.len()
        );
    }
    let version = envelope.meta.and_then(|meta| meta.data_version);
    Ok((list, version))
}

// --- Cache ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cache {
    version: u32,
    #[serde(default)]
    data_version: Option<String>,
    stratagems: Vec<RemoteStratagem>,
}

/// Última lista válida baixada, ou vazia (sem arquivo, formato velho, dado
/// adulterado). Lista vazia deixa o app só com o JSON embarcado.
pub fn load_cache() -> Vec<RemoteStratagem> {
    read_cache(&util::config_path(CACHE_FILE))
}

fn read_cache(path: &Path) -> Vec<RemoteStratagem> {
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
    let list: Vec<RemoteStratagem> = cache
        .stratagems
        .into_iter()
        .filter(RemoteStratagem::is_valid)
        .collect();
    if list.len() < MIN_LOADOUT {
        log::warn!("{} incompleto; ignorado", path.display());
        return Vec::new();
    }
    list
}

fn write_cache(path: &Path, list: &[RemoteStratagem], data_version: Option<String>) -> Result<()> {
    let cache = Cache {
        version: CACHE_VERSION,
        data_version,
        stratagems: list.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&cache).context("falha ao serializar o cache")?;
    util::write_atomic(path, &json)
}

// --- Worker ---

/// Já houve uma sincronização nesta sessão.
static AUTO_SYNCED: AtomicBool = AtomicBool::new(false);

/// Sincronização da sessão: no máximo uma, e nunca com o jogo em foco.
///
/// Chamada junto com o check do updater: depois da primeira pintura e na
/// primeira perda de foco. Com o jogo na frente a chamada não gasta a vez.
pub fn auto_sync(shared: &Arc<Shared>) {
    if shared.is_game_focused() {
        log::debug!("sincronização de dados adiada: o jogo está em foco");
        return;
    }
    if AUTO_SYNCED.swap(true, Ordering::Relaxed) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("data-sync".to_string())
        .spawn(|| {
            let agent = util::http_agent(Some(TIMEOUT));
            // Independentes: uma lista quebrada não segura a outra.
            match sync(&agent) {
                Ok(0) => log::info!("estratagemas da API conferidos: nada novo"),
                Ok(fresh) => {
                    log::info!("{fresh} estratagema(s) novo(s) da API; entram no próximo boot")
                }
                Err(err) => log::warn!("sincronização de estratagemas falhou: {err:#}"),
            }
            match crate::equipment_sync::sync(&agent) {
                Ok(0) => log::info!("equipamento da API conferido: nada novo"),
                Ok(fresh) => {
                    log::info!(
                        "{fresh} item(ns) de equipamento novo(s) da API; entram no próximo boot"
                    )
                }
                Err(err) => log::warn!("sincronização de equipamento falhou: {err:#}"),
            }
        });
    if let Err(err) = spawned {
        log::warn!("worker de sincronização não subiu: {err}");
    }
}

/// Baixa a lista, garante os ícones dos novos e grava o cache. Devolve quantos
/// estratagemas a API tem que o JSON embarcado não tem.
fn sync(agent: &ureq::Agent) -> Result<usize> {
    let (list, data_version) = fetch_list(agent)?;

    let bundled: HashSet<String> = crate::data::GameData::load()?
        .all()
        .iter()
        .map(|s| s.slug.clone())
        .collect();
    let fresh: Vec<&RemoteStratagem> = list.iter().filter(|r| !bundled.contains(&r.slug)).collect();

    let dir = util::config_dir().join(util::DOWNLOADED_ICONS_DIR);
    for item in &fresh {
        let path = dir.join(item.icon_file());
        if path.exists() {
            continue;
        }
        // Sem ícone o estratagema ainda funciona (o card sai sem imagem), então
        // uma falha aqui não derruba a sincronização.
        if let Err(err) = download_icon(agent, item, &path) {
            log::warn!("ícone de {} não baixou: {err:#}", item.slug);
        }
    }

    write_cache(&util::config_path(CACHE_FILE), &list, data_version)?;
    prune_icons(&dir, &fresh);
    Ok(fresh.len())
}

/// A lista de estratagemas de loadout da API, validada.
fn fetch_list(agent: &ureq::Agent) -> Result<(Vec<RemoteStratagem>, Option<String>)> {
    let bytes = get_bytes(agent, STRATAGEMS_PATH, MAX_LIST_BYTES)?;
    parse_api(&bytes)
}

/// `GET` de um caminho da API, com teto no tamanho do corpo.
pub(crate) fn get_bytes(agent: &ureq::Agent, path: &str, limit: u64) -> Result<Vec<u8>> {
    let url = format!("{API_BASE}{path}");
    agent
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("GET {url}"))?
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .with_context(|| format!("falha ao ler {url}"))
}

fn download_icon(agent: &ureq::Agent, item: &RemoteStratagem, path: &Path) -> Result<()> {
    download_image(agent, &item.image, path)
}

/// Baixa uma imagem da API para `path`, só se ela decodificar.
pub(crate) fn download_image(agent: &ureq::Agent, image: &str, path: &Path) -> Result<()> {
    let bytes = get_bytes(agent, image, MAX_ICON_BYTES)?;

    // Só vira ícone o que decodifica: um arquivo que o decoder recusa ficaria
    // marcado como quebrado no cache de bitmaps e nunca seria baixado de novo.
    let partial = path.with_extension("part");
    util::write_atomic(&partial, &bytes)?;
    let checked = crate::gfx::images::decode(&partial)
        .and_then(|_| std::fs::rename(&partial, path).context("falha ao mover o ícone"));
    if checked.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    checked
}

/// Apaga os ícones que nenhum estratagema novo usa mais: os que entraram no
/// JSON embarcado num release e os de uma versão anterior da arte.
fn prune_icons(dir: &Path, fresh: &[&RemoteStratagem]) {
    let keep: HashSet<&str> = fresh.iter().map(|r| r.icon_file()).collect();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !keep.contains(name.to_string_lossy().as_ref()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    fn remote(slug: &str, permit: &str, kind: &str, traits: &[&str]) -> RemoteStratagem {
        RemoteStratagem {
            slug: slug.to_string(),
            nome: slug.to_string(),
            permit: permit.to_string(),
            kind: kind.to_string(),
            traits: traits.iter().map(|t| t.to_string()).collect(),
            codex: free_codex(slug),
            image: format!("{IMAGES_PATH}{slug}.0123abcd.webp"),
        }
    }

    /// Codex de 8 passos que não conflita com nenhum do jogo, um por slug.
    fn free_codex(slug: &str) -> Vec<Dir> {
        let taken = bundled();
        (stable_id(slug)..)
            .map(|n| {
                (0..8)
                    .map(|k| Dir::ALL[(n >> (2 * k)) as usize & 3])
                    .collect()
            })
            .find(|codex: &Vec<Dir>| !clashes(codex, &taken, None))
            .unwrap()
    }

    fn codex_of<'a>(list: &'a [Stratagem], slug: &str) -> &'a [Dir] {
        &list[position(list, slug)].codex
    }

    fn set_codex(api: &mut [RemoteStratagem], slug: &str, codex: &[Dir]) {
        api.iter_mut().find(|r| r.slug == slug).unwrap().codex = codex.to_vec();
    }

    /// Resposta real da API (17/09/2026), recortada aos campos que o app lê.
    fn fixture() -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/api-stratagems.json");
        std::fs::read(path).expect("fixture da API")
    }

    /// Os estratagemas de loadout da fixture: os 92 do JSON embarcado.
    fn api_snapshot() -> Vec<RemoteStratagem> {
        parse_api(&fixture()).expect("fixture válida").0
    }

    fn bundled() -> Vec<Stratagem> {
        GameData::load().unwrap().all().to_vec()
    }

    fn position(list: &[Stratagem], slug: &str) -> usize {
        list.iter()
            .position(|s| s.slug == slug)
            .unwrap_or_else(|| panic!("{slug} não está na lista"))
    }

    #[test]
    fn stable_id_matches_the_sync_script() {
        // Mesmos números que o `stableId` do scripts/sync-stratagems.mjs dá.
        assert_eq!(stable_id("orbital-precision-strike"), 0xd41c_e89c);
        assert_eq!(stable_id("40-k-meltagun"), 0xbaa3_c645);
        assert_eq!(stable_id(""), 0x811c_9dc5);
    }

    #[test]
    fn the_fixture_covers_every_bundled_stratagem() {
        let api = api_snapshot();
        let slugs: HashSet<&str> = api.iter().map(|r| r.slug.as_str()).collect();
        for strat in bundled() {
            assert!(
                slugs.contains(strat.slug.as_str()),
                "{} fora da API",
                strat.slug
            );
        }
    }

    #[test]
    fn nothing_changes_when_the_api_has_nothing_new() {
        assert_eq!(merge(bundled(), &api_snapshot()), bundled());
    }

    #[test]
    fn names_order_and_tags_stay_bundled() {
        let mut api = api_snapshot();
        for item in &mut api {
            item.nome = "outro nome".into();
            item.kind = "orbital".into();
        }
        assert_eq!(merge(bundled(), &api), bundled());
    }

    #[test]
    fn a_codex_changed_in_a_patch_follows_the_api() {
        let mut api = api_snapshot();
        let new = free_codex("orbital-laser");
        set_codex(&mut api, "orbital-laser", &new);
        let merged = merge(bundled(), &api);

        assert_eq!(codex_of(&merged, "orbital-laser"), new);
        // Só o codex muda: o resto da entrada e o resto da lista ficam.
        let mut expected = bundled();
        let at = position(&expected, "orbital-laser");
        expected[at].codex = new;
        assert_eq!(merged, expected);
    }

    #[test]
    fn a_codex_that_clashes_with_another_is_refused() {
        let old = codex_of(&bundled(), "orbital-laser").to_vec();
        let precision = codex_of(&bundled(), "orbital-precision-strike").to_vec();

        // Igual ao de outro estratagema.
        let mut api = api_snapshot();
        set_codex(&mut api, "orbital-laser", &precision);
        assert_eq!(codex_of(&merge(bundled(), &api), "orbital-laser"), old);

        // Começo do de outro (→ → dispararia antes do → → ↑ da Precision).
        set_codex(&mut api, "orbital-laser", &precision[..2]);
        assert_eq!(codex_of(&merge(bundled(), &api), "orbital-laser"), old);

        // Estendendo o de um apoio fixo (Resupply é ↓ ↓ ↑ →).
        let mut longer = SUPPORT_STRATS[1].codex.to_vec();
        longer.push(Dir::Left);
        set_codex(&mut api, "orbital-laser", &longer);
        assert_eq!(codex_of(&merge(bundled(), &api), "orbital-laser"), old);
    }

    #[test]
    fn two_stratagems_swapping_codexes_both_update() {
        let laser = codex_of(&bundled(), "orbital-laser").to_vec();
        let railcannon = codex_of(&bundled(), "orbital-railcannon-strike").to_vec();
        let mut api = api_snapshot();
        set_codex(&mut api, "orbital-laser", &railcannon);
        set_codex(&mut api, "orbital-railcannon-strike", &laser);

        let merged = merge(bundled(), &api);
        assert_eq!(codex_of(&merged, "orbital-laser"), railcannon);
        assert_eq!(codex_of(&merged, "orbital-railcannon-strike"), laser);
    }

    #[test]
    fn a_refused_change_does_not_block_the_others() {
        let precision = codex_of(&bundled(), "orbital-precision-strike").to_vec();
        let old_laser = codex_of(&bundled(), "orbital-laser").to_vec();
        let new_gatling = free_codex("orbital-gatling-barrage");
        let mut api = api_snapshot();
        set_codex(&mut api, "orbital-laser", &precision);
        set_codex(&mut api, "orbital-gatling-barrage", &new_gatling);

        let merged = merge(bundled(), &api);
        assert_eq!(codex_of(&merged, "orbital-laser"), old_laser);
        assert_eq!(codex_of(&merged, "orbital-gatling-barrage"), new_gatling);
    }

    #[test]
    fn too_many_codex_changes_at_once_are_all_refused() {
        let mut api = api_snapshot();
        for item in api.iter_mut().take(MAX_CODEX_CHANGES + 1) {
            item.codex = free_codex(&item.slug);
        }
        assert_eq!(merge(bundled(), &api), bundled());

        // No teto ainda passa.
        let mut api = api_snapshot();
        for item in api.iter_mut().take(MAX_CODEX_CHANGES) {
            item.codex = free_codex(&item.slug);
        }
        assert_ne!(merge(bundled(), &api), bundled());
    }

    #[test]
    fn a_new_stratagem_with_a_clashing_codex_is_skipped() {
        let mut api = api_snapshot();
        let mut clashing = remote("orbital-copia", "Offensive", "orbital", &["orbital"]);
        clashing.codex = codex_of(&bundled(), "orbital-laser").to_vec();
        api.push(clashing);
        assert_eq!(merge(bundled(), &api), bundled());
    }

    #[test]
    fn a_new_stratagem_lands_at_the_end_of_its_subgroup() {
        let mut api = api_snapshot();
        api.push(remote(
            "eat-99-novo",
            "Supply",
            "support_weapon",
            &["anti-tank", "expendable"],
        ));
        api.push(remote(
            "mls-9-sentinela",
            "Defensive",
            "sentry",
            &["sentry"],
        ));
        api.push(remote(
            "md-99-novas-mines",
            "Defensive",
            "emplacement",
            &["explosive"],
        ));
        api.push(remote("e-99-emplacement", "Defensive", "emplacement", &[]));
        api.push(remote("orbital-novo", "Offensive", "orbital", &["orbital"]));
        let merged = merge(bundled(), &api);
        assert_eq!(merged.len(), bundled().len() + 5);

        // Depois do EAT-17, último descartável, e antes da Autocannon.
        let at = position(&merged, "eat-99-novo");
        assert_eq!(merged[at - 1].slug, "eat-17-expendable-anti-tank");
        assert_eq!(merged[at + 1].slug, "ac-8-autocannon");
        // Sentinela nova fica com as sentinelas, não depois da Tesla Tower.
        let at = position(&merged, "mls-9-sentinela");
        assert_eq!(merged[at - 1].slug, "a-m-12-mortar-sentry");
        // Minas com minas; emplacement com emplacements.
        let at = position(&merged, "md-99-novas-mines");
        assert_eq!(merged[at - 1].slug, "md-i4-incendiary-mines");
        let at = position(&merged, "e-99-emplacement");
        assert_eq!(merged[at - 1].slug, "e-mg-101-hmg-emplacement");
        assert_eq!(merged[at + 1].slug, "a-arc-3-tesla-tower");
        // Orbital novo antes das Eagles.
        let at = position(&merged, "orbital-novo");
        assert_eq!(merged[at + 1].slug, "eagle-500kg-bomb");
    }

    #[test]
    fn two_new_ones_in_the_same_subgroup_keep_the_api_order() {
        let mut api = api_snapshot();
        api.push(remote("cqc-98-a", "Supply", "support_weapon", &["melee"]));
        api.push(remote("cqc-99-b", "Supply", "support_weapon", &["melee"]));
        let merged = merge(bundled(), &api);
        let first = position(&merged, "cqc-98-a");
        assert_eq!(merged[first - 1].slug, "cqc-1-one-true-flag");
        assert_eq!(merged[first + 1].slug, "cqc-99-b");
        assert_eq!(merged[first + 2].slug, "mg-43-machine-gun");
    }

    #[test]
    fn an_unknown_subgroup_goes_to_the_end_of_its_color() {
        let mut api = api_snapshot();
        api.push(remote("coisa-nova", "Offensive", "artillery", &[]));
        let merged = merge(bundled(), &api);
        let at = position(&merged, "coisa-nova");
        assert_eq!(merged[at - 1].slug, "eagle-smoke-strike");
        assert_eq!(merged[at + 1].primary_tag(), Some("Supply"));
    }

    #[test]
    fn a_new_stratagem_gets_tags_icon_and_kind() {
        let mut api = api_snapshot();
        api.push(remote(
            "exo-99-novo-exosuit",
            "Supply",
            "vehicle",
            &["vehicle"],
        ));
        api.push(remote("m-999-frv", "Supply", "vehicle", &["vehicle"]));
        api.push(remote(
            "gr-99-mochila",
            "Supply",
            "support_weapon",
            &["backpack"],
        ));
        let merged = merge(bundled(), &api);

        let exo = &merged[position(&merged, "exo-99-novo-exosuit")];
        assert_eq!(exo.id, stable_id("exo-99-novo-exosuit"));
        assert_eq!(exo.tag, ["Supply", "Mecha"]);
        assert_eq!(exo.imagem, "remote/exo-99-novo-exosuit.0123abcd.webp");
        assert_eq!(
            merged[position(&merged, "exo-99-novo-exosuit") - 1].slug,
            "exo-51-lumberer-exosuit"
        );

        let frv = &merged[position(&merged, "m-999-frv")];
        assert_eq!(frv.tag, ["Supply", "Vehicle"]);

        let weapon = &merged[position(&merged, "gr-99-mochila")];
        let kind = weapon.kind_hint.unwrap();
        assert!(kind.support && kind.backpack && !kind.sentry);
    }

    #[test]
    fn build_rules_classify_a_new_stratagem_by_its_api_kind() {
        let mut api = api_snapshot();
        api.push(remote(
            "gr-99-mochila",
            "Supply",
            "support_weapon",
            &["backpack"],
        ));
        let data = GameData::from_list(merge(bundled(), &api));
        let meta = crate::data::StratMeta::build(&data, crate::data::equipment().unwrap());

        let id = stable_id("gr-99-mochila");
        assert!(meta.is_support(id) && meta.is_backpack(id));
        // Os que o `stratagemInfo` conhece continuam vindo dele.
        let autocannon = data
            .all()
            .iter()
            .find(|s| s.slug == "ac-8-autocannon")
            .unwrap();
        assert!(meta.is_support(autocannon.id) && meta.is_backpack(autocannon.id));
    }

    #[test]
    fn the_merged_list_still_indexes_by_id() {
        let mut api = api_snapshot();
        api.push(remote("orbital-novo", "Offensive", "orbital", &["orbital"]));
        let data = GameData::from_list(merge(bundled(), &api));
        let id = stable_id("orbital-novo");
        assert_eq!(
            data.by_id(id).map(|s| s.slug.as_str()),
            Some("orbital-novo")
        );
        assert_eq!(
            data.by_id(0).map(|s| s.slug.as_str()),
            Some("orbital-precision-strike")
        );
    }

    #[test]
    fn api_items_are_validated() {
        let item = |patch: serde_json::Value| {
            let mut base = serde_json::json!({
                "id": "orbital-teste",
                "name": "Orbital Teste",
                "permitType": "offensive",
                "availability": "loadout",
                "kind": "orbital",
                "traitIds": ["orbital"],
                "code": ["right", "up"],
                "image": { "url": "/images/v1/stratagems/orbital-teste.abc123.webp" }
            });
            for (key, value) in patch.as_object().unwrap() {
                base[key] = value.clone();
            }
            from_api(serde_json::from_value(base).unwrap())
        };

        assert!(item(serde_json::json!({})).is_some());
        assert!(item(serde_json::json!({ "availability": "mission" })).is_none());
        assert!(item(serde_json::json!({ "permitType": "mission" })).is_none());
        assert!(item(serde_json::json!({ "code": [] })).is_none());
        assert!(item(serde_json::json!({ "code": ["up", "jump"] })).is_none());
        let long = vec!["up"; MAX_CODEX + 1];
        assert!(item(serde_json::json!({ "code": long })).is_none());
        assert!(item(serde_json::json!({ "id": "../fora" })).is_none());
        assert!(item(serde_json::json!({ "id": "Maiuscula" })).is_none());
        assert!(item(serde_json::json!({ "name": " " })).is_none());
        assert!(item(serde_json::json!({ "kind": null })).is_none());
        assert!(item(serde_json::json!({
            "image": { "url": "/images/v1/stratagems/../../x.webp" }
        }))
        .is_none());
        assert!(item(serde_json::json!({
            "image": { "url": "https://outro.site/x.webp" }
        }))
        .is_none());
        assert!(item(serde_json::json!({
            "image": { "url": "/images/v1/stratagems/x.png" }
        }))
        .is_none());
    }

    #[test]
    fn a_short_api_list_is_refused_whole() {
        let json = serde_json::json!({
            "data": [{
                "id": "orbital-teste", "name": "Orbital Teste", "permitType": "offensive",
                "availability": "loadout", "kind": "orbital", "code": ["up"],
                "image": { "url": "/images/v1/stratagems/orbital-teste.abc.webp" }
            }]
        });
        let err = parse_api(json.to_string().as_bytes()).unwrap_err();
        assert!(err.to_string().contains("mínimo"), "{err}");
    }

    #[test]
    fn a_broken_item_does_not_sink_the_list() {
        let mut json: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
        json["data"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": 42, "name": ["não", "é", "texto"] }));
        let (list, _) = parse_api(json.to_string().as_bytes()).unwrap();
        assert_eq!(list.len(), api_snapshot().len());
    }

    #[test]
    fn the_cache_round_trips_and_rejects_tampering() {
        let dir = std::env::temp_dir().join(format!("mh2-sync-{}", std::process::id()));
        let path = dir.join(CACHE_FILE);
        let list = api_snapshot();

        write_cache(&path, &list, Some("2026-09-17.x".into())).unwrap();
        assert_eq!(read_cache(&path), list);

        // Um item adulterado sai; o resto fica.
        let mut tampered = list.clone();
        tampered[0].image = "/images/v1/stratagems/../../../evil.webp".into();
        write_cache(&path, &tampered, None).unwrap();
        assert_eq!(read_cache(&path), list[1..]);

        // Formato de outra versão não é lido.
        let json = serde_json::json!({ "version": 99, "stratagems": list });
        std::fs::write(&path, json.to_string()).unwrap();
        assert!(read_cache(&path).is_empty());

        // Lista curta demais é ignorada inteira.
        write_cache(&path, &list[..10], None).unwrap();
        assert!(read_cache(&path).is_empty());

        assert!(read_cache(&dir.join("nao-existe.json")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "depende de rede"]
    fn the_live_api_serves_every_bundled_stratagem_and_its_icons() {
        let agent = util::http_agent(Some(TIMEOUT));
        let (list, version) = fetch_list(&agent).expect("lista da API");
        assert!(version.is_some());
        let slugs: HashSet<&str> = list.iter().map(|r| r.slug.as_str()).collect();
        for strat in bundled() {
            assert!(
                slugs.contains(strat.slug.as_str()),
                "{} sumiu da API",
                strat.slug
            );
        }

        let dir = std::env::temp_dir().join(format!("mh2-icons-{}", std::process::id()));
        let item = &list[0];
        let path = dir.join(item.icon_file());
        download_icon(&agent, item, &path).expect("ícone da API");
        assert!(crate::gfx::images::decode(&path).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn downloaded_icons_resolve_outside_the_install_dir() {
        let path = util::resource_path("icons/remote/orbital-teste.abc.webp");
        assert!(path.starts_with(util::config_dir()));
        assert!(path.ends_with("remote-icons/orbital-teste.abc.webp"));
        assert_eq!(
            util::resource_path("icons/stratagems/x.webp"),
            util::asset_path("icons/stratagems/x.webp")
        );
    }
}
