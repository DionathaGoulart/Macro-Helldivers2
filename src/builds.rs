//! Builds: sorteio de loadout, regras de balanceamento, sets de armadura e as
//! operações das builds salvas.
//!
//! Tudo aqui é lógica pura — nenhuma chamada de Windows, nenhum estado de tela.
//! O `cargo test` do host cobre exatamente o que roda em produção, e a aba
//! (`ui/build_tab.rs`) só decide quando chamar. O sorteio recebe o gerador por
//! parâmetro para os testes poderem rodá-lo centenas de vezes.
//!
//! Porte de `legacy/src/renderer/components/BuildTab.jsx` (~114–281).

use std::collections::HashMap;

use rand::seq::{IndexedRandom, SliceRandom};
use rand::{Rng, RngExt};

use crate::data::EQUIP_SLOT_COUNT;
use crate::data::{self, EquipSlot, Equipment, GameData, Item, StatsMap, StratMeta, Stratagem};
use crate::loadouts::Loadout;
use crate::meta_stats::{ItemStat, Stats};
use crate::settings::{Settings, SLOT_COUNT};
use crate::shared::Slots;
use crate::util;

/// Uma build: quatro estratagemas e um item por categoria de equipamento.
///
/// Guarda ids, não os itens: assim ela sobrevive a uma atualização dos JSONs e
/// vira build salva sem conversão nenhuma.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Build {
    pub stratagems: Slots,
    equip: [Option<String>; EQUIP_SLOT_COUNT],
}

impl Build {
    pub fn equip(&self, slot: EquipSlot) -> Option<&str> {
        self.equip[slot.index()].as_deref()
    }

    pub fn set_equip(&mut self, slot: EquipSlot, id: Option<String>) {
        self.equip[slot.index()] = id;
    }

    /// A v1 só salvava e só aplicava build com pelo menos um estratagema.
    pub fn has_stratagem(&self) -> bool {
        self.stratagems.iter().any(Option::is_some)
    }

    pub fn is_empty(&self) -> bool {
        !self.has_stratagem() && self.equip.iter().all(Option::is_none)
    }

    /// Item de uma categoria, resolvido contra o `equipment.json` atual.
    pub fn item<'a>(&self, equipment: &'a Equipment, slot: EquipSlot) -> Option<Item<'a>> {
        equipment.find(slot, self.equip(slot)?)
    }

    /// Capacete ou capa que fecham o set da armadura equipada — o que ganha a
    /// etiqueta "SET" na tela (mesma regra do legado: capacete de nome idêntico,
    /// capa da mesma warbond).
    pub fn is_set_piece(&self, equipment: &Equipment, slot: EquipSlot) -> bool {
        let Some(item) = self.item(equipment, slot) else {
            return false;
        };
        let Some(armor) = self.item(equipment, EquipSlot::Armor) else {
            return false;
        };
        match slot {
            EquipSlot::Helmet => item.nome() == armor.nome(),
            EquipSlot::Cape => match (item.warbond(), armor.warbond()) {
                (Some(cape), Some(armor)) => !armor.is_empty() && cape == armor,
                _ => false,
            },
            _ => false,
        }
    }
}

/// As três opções de sorteio da aba, lidas dos settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    pub match_set: bool,
    pub balanced: bool,
    pub max_one_sentry: bool,
}

impl Rules {
    pub fn from_settings(settings: &Settings) -> Rules {
        Rules {
            match_set: settings.build_match_set,
            balanced: settings.build_balanced,
            max_one_sentry: settings.build_max_one_sentry,
        }
    }
}

/// Itens que o próximo sorteio deve preservar (os cadeados da tela).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Locks {
    stratagems: [bool; SLOT_COUNT],
    equip: [bool; EQUIP_SLOT_COUNT],
}

impl Locks {
    pub fn stratagem(&self, index: usize) -> bool {
        self.stratagems.get(index).copied().unwrap_or(false)
    }

    pub fn toggle_stratagem(&mut self, index: usize) {
        if let Some(lock) = self.stratagems.get_mut(index) {
            *lock = !*lock;
        }
    }

    pub fn equip(&self, slot: EquipSlot) -> bool {
        self.equip[slot.index()]
    }

    pub fn toggle_equip(&mut self, slot: EquipSlot) {
        self.equip[slot.index()] = !self.equip[slot.index()];
    }
}

/// Sorteia uma build inteira, preservando o que está travado.
///
/// Porte de `generateFullBuild` (~168–212): a build balanceada coloca primeiro
/// uma arma de apoio e um item de mochila (um apoio que já vem com mochila conta
/// pelos dois), e só então os slots restantes saem de uma pool embaralhada.
pub fn generate<R: Rng + ?Sized>(
    prev: Option<&Build>,
    locks: &Locks,
    rules: Rules,
    data: &GameData,
    equipment: &Equipment,
    meta: &StratMeta,
    rng: &mut R,
) -> Build {
    let mut strats = Slots::default();
    for (index, slot) in strats.iter_mut().enumerate() {
        if locks.stratagem(index) {
            *slot = prev.and_then(|build| build.stratagems[index]);
        }
    }

    let mut pool: Vec<u32> = data.all().iter().map(|strat| strat.id).collect();
    pool.shuffle(rng);

    if rules.balanced {
        if !strats.iter().flatten().any(|id| meta.is_support(*id)) {
            place_first(&mut strats, &pool, rules, data, meta, |id| {
                meta.is_support(id)
            });
        }
        if !strats.iter().flatten().any(|id| meta.is_backpack(*id)) {
            place_first(&mut strats, &pool, rules, data, meta, |id| {
                meta.is_backpack(id)
            });
        }
    }
    for index in 0..SLOT_COUNT {
        if strats[index].is_some() {
            continue;
        }
        if let Some(id) = pool
            .iter()
            .copied()
            .find(|id| can_add(*id, &strats, rules, data, meta))
        {
            strats[index] = Some(id);
        }
    }

    let mut build = Build {
        stratagems: strats,
        ..Build::default()
    };
    for slot in EquipSlot::ALL {
        let id = match kept_equip(prev, locks, slot) {
            Some(id) => Some(id),
            None => random_item(equipment, slot, rng).map(|item| item.id().to_string()),
        };
        build.set_equip(slot, id);
    }
    apply_set_matching(&mut build, locks, rules, equipment, rng);
    build
}

/// Preenche o primeiro slot vazio com o primeiro item da pool que atende ao
/// predicado (`placeInEmptySlot` do legado).
fn place_first(
    strats: &mut Slots,
    pool: &[u32],
    rules: Rules,
    data: &GameData,
    meta: &StratMeta,
    predicate: impl Fn(u32) -> bool,
) {
    let Some(index) = strats.iter().position(Option::is_none) else {
        return;
    };
    if let Some(id) = pool
        .iter()
        .copied()
        .find(|id| predicate(*id) && can_add(*id, strats, rules, data, meta))
    {
        strats[index] = Some(id);
    }
}

/// O estratagema cabe na build como ela está? (`canAdd` do legado.)
fn can_add(id: u32, strats: &Slots, rules: Rules, data: &GameData, meta: &StratMeta) -> bool {
    if strats.contains(&Some(id)) {
        return false;
    }
    let Some(strat) = data.by_id(id) else {
        return false;
    };

    // A lista dos equipados não tem buracos, e nenhum deles é o slot em edição:
    // aqui todo conflito conta (o legado passava `activeSlot = -1`).
    let equipped: Vec<Option<&Stratagem>> =
        strats.iter().flatten().map(|id| data.by_id(*id)).collect();
    if data::has_exclusive_conflict(strat, &equipped, usize::MAX) {
        return false;
    }

    let any = |predicate: &dyn Fn(u32) -> bool| strats.iter().flatten().any(|id| predicate(*id));
    if rules.max_one_sentry && meta.is_sentry(id) && any(&|id| meta.is_sentry(id)) {
        return false;
    }
    if rules.balanced && meta.is_support(id) && any(&|id| meta.is_support(id)) {
        return false;
    }
    if rules.balanced && meta.is_backpack(id) && any(&|id| meta.is_backpack(id)) {
        return false;
    }
    true
}

/// Item que o cadeado manda preservar, se houver um na build anterior.
fn kept_equip(prev: Option<&Build>, locks: &Locks, slot: EquipSlot) -> Option<String> {
    locks
        .equip(slot)
        .then(|| prev.and_then(|build| build.equip(slot)))
        .flatten()
        .map(str::to_string)
}

fn random_item<'a, R: Rng + ?Sized>(
    equipment: &'a Equipment,
    slot: EquipSlot,
    rng: &mut R,
) -> Option<Item<'a>> {
    let count = equipment.count(slot);
    if count == 0 {
        return None;
    }
    equipment.at(slot, rng.random_range(..count))
}

/// Fecha o set da armadura sorteada: capacete de nome idêntico e capa da mesma
/// warbond. Set sem capa correspondente mantém a aleatória (~156–166).
fn apply_set_matching<R: Rng + ?Sized>(
    build: &mut Build,
    locks: &Locks,
    rules: Rules,
    equipment: &Equipment,
    rng: &mut R,
) {
    if !rules.match_set {
        return;
    }
    let Some(armor_id) = build.equip(EquipSlot::Armor).map(str::to_string) else {
        return;
    };
    let Some(armor) = equipment.armor_by_id(&armor_id) else {
        return;
    };

    if !locks.equip(EquipSlot::Helmet) {
        if let Some(helmet) = equipment
            .helmet
            .iter()
            .find(|helmet| helmet.nome == armor.nome)
        {
            build.set_equip(EquipSlot::Helmet, Some(helmet.id.clone()));
        }
    }
    if !locks.equip(EquipSlot::Cape) && !armor.warbond.is_empty() {
        let capes: Vec<&str> = equipment
            .cape
            .iter()
            .filter(|cape| cape.warbond == armor.warbond)
            .map(|cape| cape.id.as_str())
            .collect();
        if let Some(cape) = capes.choose(rng) {
            build.set_equip(EquipSlot::Cape, Some((*cape).to_string()));
        }
    }
}

// --- Build meta ---

/// Quantos estratagemas do topo entram no sorteio meta — e são os mesmos que a
/// tela lista (`TOP_STRATS` do legado).
pub const META_TOP_STRATS: usize = 10;
/// Armas exibidas (e sorteadas) por categoria.
pub const META_TOP_WEAPONS: usize = 3;
/// Passivas de armadura exibidas, e as únicas que a armadura sorteada pode ter.
pub const META_TOP_PASSIVES: usize = 5;
/// Quando as regras ativas exigem algo que não está no topo, o sorteio cai nos
/// mais usados da lista inteira que atendam.
const META_FALLBACK: usize = 3;

/// Um item das estatísticas: o que ele referencia aqui e os números do site.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick<T> {
    /// Id do estratagema, id do equipamento ou nome da passiva.
    pub item: T,
    pub stat: ItemStat,
}

/// As listas da sub-aba Meta, resolvidas contra os nossos JSONs e ordenadas por
/// pick rate (`metaLists` do legado, ~318–335).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetaLists {
    pub stratagems: Vec<Pick<u32>>,
    /// Armas por categoria, indexadas por [`EquipSlot::index`] — primária,
    /// secundária e granada são justamente as três primeiras.
    weapons: [Vec<Pick<String>>; 3],
    /// Passivas pelo nome com que a armadura as referencia.
    pub passives: Vec<Pick<String>>,
    /// Partidas analisadas, o número do crédito ao site.
    pub games: u64,
}

impl MetaLists {
    /// Armas de uma categoria; categoria que não é de arma responde vazio.
    pub fn weapons(&self, slot: EquipSlot) -> &[Pick<String>] {
        match slot.is_weapon() {
            true => &self.weapons[slot.index()],
            false => &[],
        }
    }

    /// Sem nada resolvido não há o que mostrar nem o que sortear — é o caso de
    /// uma resposta vazia ou de um `statsMap.json` defasado.
    pub fn is_empty(&self) -> bool {
        self.stratagems.is_empty()
            && self.passives.is_empty()
            && self.weapons.iter().all(Vec::is_empty)
    }
}

/// Transforma as três respostas cruas nas listas da tela.
pub fn meta_lists(
    stats: &Stats,
    data: &GameData,
    equipment: &Equipment,
    map: &StatsMap,
) -> MetaLists {
    let mut stratagems: Vec<Pick<u32>> = stats
        .strategem
        .items
        .iter()
        .filter_map(|(slug, stat)| {
            // Slug desconhecido, ou apontando para um id que saiu do jogo, some
            // da lista em vez de virar linha vazia.
            let id = *map.strategem.get(slug)?;
            data.by_id(id)?;
            Some(Pick {
                item: id,
                stat: *stat,
            })
        })
        .collect();
    sort_picks(&mut stratagems);

    let mut weapons: [Vec<Pick<String>>; 3] = Default::default();
    for (slug, stat) in &stats.weapons.items {
        let Some(reference) = map.weapons.get(slug) else {
            continue;
        };
        let Some(slot) = EquipSlot::from_key(&reference.cat).filter(|slot| slot.is_weapon()) else {
            continue;
        };
        if equipment.find(slot, &reference.id).is_none() {
            continue;
        }
        weapons[slot.index()].push(Pick {
            item: reference.id.clone(),
            stat: *stat,
        });
    }
    for list in &mut weapons {
        sort_picks(list);
    }

    let mut passives: Vec<Pick<String>> = stats
        .armor
        .items
        .iter()
        .filter_map(|(key, stat)| {
            let name = map.armor.get(key)?;
            // Validada como as outras listas: um `statsMap.json` desatualizado
            // (passiva renomeada) produziria um top sem armadura possível, e a
            // regra de armadura meta falharia em silêncio na geração.
            equipment.passive(name)?;
            Some(Pick {
                item: name.clone(),
                stat: *stat,
            })
        })
        .collect();
    sort_picks(&mut passives);

    MetaLists {
        stratagems,
        weapons,
        passives,
        games: stats.strategem.total.games,
    }
}

/// Maior pick rate primeiro. O desempate pelo próprio item é acréscimo nosso: as
/// respostas chegam num mapa, cuja ordem de iteração muda a cada execução, e sem
/// ele dois empatados trocariam de lugar na tela sem motivo.
fn sort_picks<T: Ord>(picks: &mut [Pick<T>]) {
    picks.sort_by(|a, b| {
        b.stat
            .loadouts_percentage
            .total_cmp(&a.stat.loadouts_percentage)
            .then_with(|| a.item.cmp(&b.item))
    });
}

/// Sorteio ponderado pelo pick rate (`weightedFrom` do legado, ~338–347).
fn weighted<'a, T, R: Rng + ?Sized>(list: &[&'a Pick<T>], rng: &mut R) -> Option<&'a Pick<T>> {
    let total: f64 = list.iter().map(|pick| pick.stat.loadouts_percentage).sum();
    // Lista sem peso nenhum (site zerado, todos empatados em 0) devolve o
    // primeiro, que é o que a v1 fazia.
    if total <= 0.0 {
        return list.first().copied();
    }
    let mut roll = rng.random_range(0.0..total);
    for pick in list {
        roll -= pick.stat.loadouts_percentage;
        if roll <= 0.0 {
            return Some(pick);
        }
    }
    list.last().copied()
}

/// Sorteia uma build a partir das estatísticas.
///
/// Porte de `generateMetaBuild` (~349–424): mesmas regras da build aleatória,
/// mas a pool é o topo exibido na tela, e o peso de cada item é o pick rate.
#[allow(clippy::too_many_arguments)]
pub fn generate_meta<R: Rng + ?Sized>(
    prev: Option<&Build>,
    locks: &Locks,
    rules: Rules,
    lists: &MetaLists,
    data: &GameData,
    equipment: &Equipment,
    meta: &StratMeta,
    rng: &mut R,
) -> Build {
    let mut strats = Slots::default();
    for (index, slot) in strats.iter_mut().enumerate() {
        if locks.stratagem(index) {
            *slot = prev.and_then(|build| build.stratagems[index]);
        }
    }

    if rules.balanced {
        if !strats.iter().flatten().any(|id| meta.is_support(*id)) {
            place_meta(
                &mut strats,
                lists,
                rules,
                data,
                meta,
                Some(&|id| meta.is_support(id)),
                rng,
            );
        }
        if !strats.iter().flatten().any(|id| meta.is_backpack(*id)) {
            place_meta(
                &mut strats,
                lists,
                rules,
                data,
                meta,
                Some(&|id| meta.is_backpack(id)),
                rng,
            );
        }
    }
    for index in 0..SLOT_COUNT {
        if strats[index].is_some() {
            continue;
        }
        if let Some(id) = pick_meta(&strats, lists, rules, data, meta, None, rng) {
            strats[index] = Some(id);
        }
    }

    let mut build = Build {
        stratagems: strats,
        ..Build::default()
    };

    // Armas: só o topo exibido de cada categoria, com peso maior para o nº 1.
    // Categoria sem nenhuma arma resolvida cai no sorteio comum.
    for slot in [EquipSlot::Primary, EquipSlot::Secondary, EquipSlot::Grenade] {
        let id = match kept_equip(prev, locks, slot) {
            Some(id) => Some(id),
            None => {
                let top: Vec<&Pick<String>> =
                    lists.weapons(slot).iter().take(META_TOP_WEAPONS).collect();
                match weighted(&top, rng) {
                    Some(pick) => Some(pick.item.clone()),
                    None => random_item(equipment, slot, rng).map(|item| item.id().to_string()),
                }
            }
        };
        build.set_equip(slot, id);
    }

    // Armadura: só as que carregam uma das passivas do topo.
    let armor = match kept_equip(prev, locks, EquipSlot::Armor) {
        Some(id) => Some(id),
        None => {
            let top: Vec<&Pick<String>> = lists.passives.iter().take(META_TOP_PASSIVES).collect();
            let candidates: Vec<&str> = match weighted(&top, rng) {
                Some(passive) => equipment
                    .armor
                    .iter()
                    .filter(|armor| armor.passive == passive.item)
                    .map(|armor| armor.id.as_str())
                    .collect(),
                None => Vec::new(),
            };
            match candidates.choose(rng) {
                Some(id) => Some((*id).to_string()),
                None => {
                    random_item(equipment, EquipSlot::Armor, rng).map(|item| item.id().to_string())
                }
            }
        }
    };
    build.set_equip(EquipSlot::Armor, armor);

    // O resto não tem estatística: sai no sorteio comum, respeitando o cadeado.
    for slot in [EquipSlot::Helmet, EquipSlot::Cape, EquipSlot::Booster] {
        let id = match kept_equip(prev, locks, slot) {
            Some(id) => Some(id),
            None => random_item(equipment, slot, rng).map(|item| item.id().to_string()),
        };
        build.set_equip(slot, id);
    }

    apply_set_matching(&mut build, locks, rules, equipment, rng);
    build
}

/// Preenche o primeiro slot vazio com um estratagema do topo que atenda ao
/// predicado (`placeInEmptySlot` da build meta).
fn place_meta<R: Rng + ?Sized>(
    strats: &mut Slots,
    lists: &MetaLists,
    rules: Rules,
    data: &GameData,
    meta: &StratMeta,
    predicate: Option<&dyn Fn(u32) -> bool>,
    rng: &mut R,
) {
    let Some(index) = strats.iter().position(Option::is_none) else {
        return;
    };
    if let Some(id) = pick_meta(strats, lists, rules, data, meta, predicate, rng) {
        strats[index] = Some(id);
    }
}

/// Escolhe um estratagema entre os mais usados que cabem na build.
///
/// Só o topo mostrado na tela entra. Se as regras ativas exigirem algo que não
/// está lá — balanceado precisa de mochila e o top 10 não tem —, a escolha cai
/// nos [`META_FALLBACK`] mais usados da lista inteira que atendam (~367–380).
fn pick_meta<R: Rng + ?Sized>(
    strats: &Slots,
    lists: &MetaLists,
    rules: Rules,
    data: &GameData,
    meta: &StratMeta,
    predicate: Option<&dyn Fn(u32) -> bool>,
    rng: &mut R,
) -> Option<u32> {
    let allowed = |pick: &&Pick<u32>| {
        predicate.is_none_or(|check| check(pick.item))
            && can_add(pick.item, strats, rules, data, meta)
    };

    let top: Vec<&Pick<u32>> = lists
        .stratagems
        .iter()
        .take(META_TOP_STRATS)
        .filter(allowed)
        .collect();
    let pool = match top.is_empty() {
        false => top,
        true => lists
            .stratagems
            .iter()
            .filter(allowed)
            .take(META_FALLBACK)
            .collect(),
    };
    weighted(&pool, rng).map(|pick| pick.item)
}

// --- Build personalizada ---

/// O card da grade não responde: o estratagema já está noutro slot da build ou
/// conflita com um exclusivo já equipado (`isCustomCardDisabled` do legado).
pub fn custom_disabled(build: &Build, slot: usize, strat: &Stratagem, data: &GameData) -> bool {
    if build.stratagems.get(slot) == Some(&Some(strat.id)) {
        return false;
    }
    if build.stratagems.contains(&Some(strat.id)) {
        return true;
    }
    let equipped = data.resolve(&build.stratagems);
    data::has_exclusive_conflict(strat, &equipped, slot)
}

/// Equipa (ou desequipa) o estratagema no slot em edição. `false` quando a
/// regra recusa a jogada — o slot em edição avança de qualquer jeito, como na
/// v1, onde o avanço acontecia fora do `setState`.
pub fn custom_assign(build: &mut Build, slot: usize, strat: &Stratagem, data: &GameData) -> bool {
    // Recusa como os demais caminhos de erro, em vez de estourar no índice.
    if slot >= build.stratagems.len() {
        return false;
    }
    if build.stratagems.get(slot) == Some(&Some(strat.id)) {
        build.stratagems[slot] = None;
        return true;
    }
    if build
        .stratagems
        .iter()
        .enumerate()
        .any(|(index, equipped)| index != slot && *equipped == Some(strat.id))
    {
        return false;
    }
    let equipped = data.resolve(&build.stratagems);
    if data::has_exclusive_conflict(strat, &equipped, slot) {
        return false;
    }
    build.stratagems[slot] = Some(strat.id);
    true
}

/// Lista da grade personalizada: tudo o que a busca deixa passar, na ordem
/// Offensive → Supply → Defensive e, dentro da categoria, por nome.
pub fn custom_list(data: &GameData, search: &str) -> Vec<u32> {
    const TAG_ORDER: [&str; 3] = ["Offensive", "Supply", "Defensive"];
    let rank = |strat: &Stratagem| {
        strat
            .primary_tag()
            .and_then(|tag| TAG_ORDER.iter().position(|known| *known == tag))
            .unwrap_or(99)
    };

    let needle = data::normalize_text(search.trim());
    let mut list: Vec<&Stratagem> = data
        .all()
        .iter()
        .filter(|strat| needle.is_empty() || data::normalize_text(&strat.nome).contains(&needle))
        .collect();
    list.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.nome.cmp(&b.nome)));
    list.into_iter().map(|strat| strat.id).collect()
}

// --- Builds salvas ---

/// Nome sugerido quando o campo fica em branco: o primeiro `Build {n}` livre.
///
/// A v1 usava `Build {contagem+1}`, que depois de uma exclusão podia colidir
/// com uma build existente — e a colisão a sobrescreveria em silêncio.
pub fn default_name(loadouts: &[Loadout]) -> String {
    (1..)
        .map(|n| format!("Build {n}"))
        .find(|name| {
            !loadouts
                .iter()
                .any(|loadout| loadout.name.eq_ignore_ascii_case(name))
        })
        .expect("sempre há um número livre")
}

/// Salva a build exibida. Um nome já usado (sem diferenciar maiúsculas)
/// sobrescreve a build existente em vez de criar outra — `handleSaveBuild`.
pub fn save(loadouts: &mut Vec<Loadout>, name: &str, build: &Build) -> bool {
    if !build.has_stratagem() {
        return false;
    }
    let name = match name.trim() {
        "" => default_name(loadouts),
        typed => typed.to_string(),
    };
    let slot_ids = build.stratagems.to_vec();
    // O mapa entra mesmo vazio: a v1 gravava `equip: {}` numa build sem
    // equipamento, e é esse campo que decide se aplicar substitui a tela toda.
    let equip: HashMap<String, String> = EquipSlot::ALL
        .into_iter()
        .filter_map(|slot| Some((slot.key().to_string(), build.equip(slot)?.to_string())))
        .collect();

    match loadouts
        .iter_mut()
        .find(|loadout| loadout.name.to_lowercase() == name.to_lowercase())
    {
        Some(existing) => {
            existing.slot_ids = slot_ids;
            existing.equip = Some(equip);
        }
        None => loadouts.push(Loadout {
            id: next_id(loadouts),
            name,
            slot_ids,
            equip: Some(equip),
        }),
    }
    true
}

/// Id no formato da v1 (`Date.now()` em texto). Duas builds salvas no mesmo
/// milissegundo receberiam o mesmo id — o relógio só anda de 1 em 1ms —, então o
/// valor é empurrado até ser inédito.
fn next_id(loadouts: &[Loadout]) -> String {
    let mut id = util::epoch_millis();
    while loadouts.iter().any(|loadout| loadout.id == id.to_string()) {
        id += 1;
    }
    id.to_string()
}

/// O que aplicar uma build salva produz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// Vai para os slots de macro (já sem conflitos de exclusividade).
    pub slots: Slots,
    /// Volta para a tela como build exibida.
    pub build: Build,
}

/// Aplica uma build salva (`handleApplyLoadout`, ~130–148).
///
/// Os três casos da v1: build com equipamento substitui a tela inteira; build
/// antiga (salva antes de o equipamento existir) só troca os estratagemas da
/// build exibida; e sem nada na tela ela vira a build exibida.
pub fn apply(
    loadout: &Loadout,
    current: Option<&Build>,
    data: &GameData,
    equipment: Option<&Equipment>,
) -> Applied {
    let slots = crate::loadouts::sanitize(&loadout.slot_ids, data);

    let mut build = match (&loadout.equip, current) {
        (None, Some(current)) => current.clone(),
        _ => Build::default(),
    };
    build.stratagems = slots;

    if let Some(equip) = &loadout.equip {
        for slot in EquipSlot::ALL {
            // Item que saiu do jogo vira slot vazio, como o `equipById` da v1.
            let id = equip
                .get(slot.key())
                .filter(|id| equipment.is_none_or(|equipment| equipment.find(slot, id).is_some()));
            build.set_equip(slot, id.cloned());
        }
    }

    Applied { slots, build }
}

/// Índice da build salva que bate com os slots atuais — o chip em destaque.
///
/// A comparação usa a lista saneada, que é o que `apply` põe nos slots: sem
/// isso, uma build com id morto, lista curta (v1) ou conflito herdado nunca
/// acenderia — justamente logo depois de ser aplicada.
pub fn active_loadout(loadouts: &[Loadout], slots: Slots, data: &GameData) -> Option<usize> {
    loadouts
        .iter()
        .position(|loadout| crate::loadouts::sanitize(&loadout.slot_ids, data) == slots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::StratKind;

    /// Quantas builds cada teste estatístico sorteia. Alto o bastante para uma
    /// regra quebrada aparecer, baixo o bastante para o teste ser instantâneo.
    const ROLLS: usize = 200;

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    fn equipment() -> &'static Equipment {
        data::equipment().expect("equipment.json do repositório")
    }

    fn meta(data: &GameData) -> StratMeta {
        StratMeta::build(data, equipment())
    }

    fn rules(balanced: bool, max_one_sentry: bool) -> Rules {
        Rules {
            match_set: true,
            balanced,
            max_one_sentry,
        }
    }

    fn roll(rules: Rules, data: &GameData, meta: &StratMeta) -> Build {
        generate(
            None,
            &Locks::default(),
            rules,
            data,
            equipment(),
            meta,
            &mut rand::rng(),
        )
    }

    #[test]
    fn a_build_fills_every_slot_without_repeating() {
        let data = data();
        let meta = meta(&data);
        for _ in 0..ROLLS {
            let build = roll(rules(false, false), &data, &meta);
            assert!(build.stratagems.iter().all(Option::is_some), "slot vazio");
            for index in 1..SLOT_COUNT {
                assert!(
                    !build.stratagems[..index].contains(&build.stratagems[index]),
                    "estratagema repetido: {:?}",
                    build.stratagems
                );
            }
            for slot in EquipSlot::ALL {
                assert!(
                    build.item(equipment(), slot).is_some(),
                    "{} sem item",
                    slot.key()
                );
            }
        }
    }

    #[test]
    fn two_mechs_never_share_a_build() {
        let data = data();
        let meta = meta(&data);
        for tag in data::EXCLUSIVE_TAGS {
            for _ in 0..ROLLS {
                let build = roll(rules(false, false), &data, &meta);
                let count = build
                    .stratagems
                    .iter()
                    .flatten()
                    .filter(|id| data.by_id(**id).is_some_and(|strat| strat.has_tag(tag)))
                    .count();
                assert!(count <= 1, "{tag} apareceu {count} vezes");
            }
        }
    }

    #[test]
    fn a_balanced_build_always_carries_a_support_weapon_and_a_backpack() {
        let data = data();
        let meta = meta(&data);
        for _ in 0..ROLLS {
            let build = roll(rules(true, false), &data, &meta);
            let ids: Vec<u32> = build.stratagems.iter().flatten().copied().collect();
            assert!(
                ids.iter().any(|id| meta.is_support(*id)),
                "build balanceada sem arma de apoio"
            );
            assert!(
                ids.iter().any(|id| meta.is_backpack(*id)),
                "build balanceada sem item de mochila"
            );
            // E nunca dois do mesmo tipo — a regra também é de exclusão.
            assert_eq!(ids.iter().filter(|id| meta.is_support(**id)).count(), 1);
            assert_eq!(ids.iter().filter(|id| meta.is_backpack(**id)).count(), 1);
        }
    }

    #[test]
    fn the_sentry_cap_holds() {
        let data = data();
        let meta = meta(&data);
        let mut with_sentry = 0;
        for _ in 0..ROLLS {
            let build = roll(rules(false, true), &data, &meta);
            let sentries = build
                .stratagems
                .iter()
                .flatten()
                .filter(|id| meta.is_sentry(**id))
                .count();
            assert!(sentries <= 1, "{sentries} sentinelas na mesma build");
            with_sentry += usize::from(sentries == 1);
        }
        assert!(
            with_sentry > 0,
            "sentinela nenhuma saiu em {ROLLS} sorteios"
        );
    }

    #[test]
    fn locked_items_survive_the_next_roll() {
        let data = data();
        let meta = meta(&data);
        let first = roll(rules(false, false), &data, &meta);

        let mut locks = Locks::default();
        locks.toggle_stratagem(1);
        locks.toggle_equip(EquipSlot::Primary);
        assert!(locks.stratagem(1) && locks.equip(EquipSlot::Primary));
        assert!(!locks.stratagem(0) && !locks.equip(EquipSlot::Armor));

        for _ in 0..ROLLS {
            let next = generate(
                Some(&first),
                &locks,
                rules(false, false),
                &data,
                equipment(),
                &meta,
                &mut rand::rng(),
            );
            assert_eq!(next.stratagems[1], first.stratagems[1]);
            assert_eq!(
                next.equip(EquipSlot::Primary),
                first.equip(EquipSlot::Primary)
            );
        }
    }

    #[test]
    fn a_locked_slot_is_not_duplicated_by_the_roll() {
        let data = data();
        let meta = meta(&data);
        let first = roll(rules(false, false), &data, &meta);
        let mut locks = Locks::default();
        locks.toggle_stratagem(0);

        for _ in 0..ROLLS {
            let next = generate(
                Some(&first),
                &locks,
                rules(false, false),
                &data,
                equipment(),
                &meta,
                &mut rand::rng(),
            );
            let kept = next.stratagems[0];
            assert!(!next.stratagems[1..].contains(&kept));
        }
    }

    #[test]
    fn set_matching_pairs_the_helmet_and_the_cape_with_the_armor() {
        let data = data();
        let meta = meta(&data);
        let equipment = equipment();
        let mut matched_helmet = 0;

        for _ in 0..ROLLS {
            let build = roll(rules(false, false), &data, &meta);
            let armor = build.item(equipment, EquipSlot::Armor).unwrap();
            let helmet = build.item(equipment, EquipSlot::Helmet).unwrap();
            let cape = build.item(equipment, EquipSlot::Cape).unwrap();

            if helmet.nome() == armor.nome() {
                matched_helmet += 1;
                assert!(build.is_set_piece(equipment, EquipSlot::Helmet));
            }
            // A capa só é trocada quando existe uma da warbond da armadura.
            let armor_warbond = armor.warbond().unwrap_or_default();
            if !armor_warbond.is_empty()
                && equipment
                    .cape
                    .iter()
                    .any(|item| item.warbond == armor_warbond)
            {
                assert_eq!(cape.warbond(), Some(armor_warbond));
                assert!(build.is_set_piece(equipment, EquipSlot::Cape));
            }
        }
        assert!(
            matched_helmet > ROLLS / 2,
            "o capacete do set quase sempre existe: {matched_helmet}/{ROLLS}"
        );
    }

    #[test]
    fn set_matching_can_be_turned_off_and_never_beats_a_lock() {
        let data = data();
        let meta = meta(&data);
        let equipment = equipment();

        // Sem a regra, o capacete quase nunca casa com a armadura.
        let matched = (0..ROLLS)
            .filter(|_| {
                let build = roll(
                    Rules {
                        match_set: false,
                        ..rules(false, false)
                    },
                    &data,
                    &meta,
                );
                let armor = build.item(equipment, EquipSlot::Armor).unwrap();
                let helmet = build.item(equipment, EquipSlot::Helmet).unwrap();
                helmet.nome() == armor.nome()
            })
            .count();
        assert!(matched < ROLLS / 4, "sets demais sem a regra: {matched}");

        // Com o capacete travado, o set não o substitui.
        let first = roll(rules(false, false), &data, &meta);
        let mut locks = Locks::default();
        locks.toggle_equip(EquipSlot::Helmet);
        for _ in 0..ROLLS {
            let next = generate(
                Some(&first),
                &locks,
                rules(false, false),
                &data,
                equipment,
                &meta,
                &mut rand::rng(),
            );
            assert_eq!(
                next.equip(EquipSlot::Helmet),
                first.equip(EquipSlot::Helmet)
            );
        }
    }

    #[test]
    fn a_build_with_nothing_classified_still_fills_four_slots() {
        // Sem `equipment.json` os predicados ficam todos falsos; a build
        // balanceada não pode travar por causa disso.
        let data = data();
        let meta = StratMeta::default();
        assert_eq!(meta.kind(0), StratKind::default());

        let build = roll(rules(true, true), &data, &meta);
        assert!(build.stratagems.iter().all(Option::is_some));
    }

    // --- Meta ---

    fn stat(percentage: f64) -> ItemStat {
        ItemStat {
            loadouts_percentage: percentage,
            ..ItemStat::default()
        }
    }

    fn pick<T>(item: T, percentage: f64) -> Pick<T> {
        Pick {
            item,
            stat: stat(percentage),
        }
    }

    fn section(items: &[(&str, f64)], games: u64) -> crate::meta_stats::Section {
        crate::meta_stats::Section {
            items: items
                .iter()
                .map(|(slug, percentage)| ((*slug).to_string(), stat(*percentage)))
                .collect(),
            total: crate::meta_stats::Total { games },
        }
    }

    /// Listas com os estratagemas dados, em pick rate decrescente, e nada mais.
    fn lists_of(ids: &[u32]) -> MetaLists {
        MetaLists {
            stratagems: ids
                .iter()
                .enumerate()
                .map(|(rank, id)| pick(*id, 50.0 - rank as f64))
                .collect(),
            ..MetaLists::default()
        }
    }

    fn roll_meta(lists: &MetaLists, rules: Rules, data: &GameData, meta: &StratMeta) -> Build {
        generate_meta(
            None,
            &Locks::default(),
            rules,
            lists,
            data,
            equipment(),
            meta,
            &mut rand::rng(),
        )
    }

    #[test]
    fn the_lists_resolve_the_slugs_and_come_out_by_pick_rate() {
        let data = data();
        let equipment = equipment();
        let map = data::stats_map().expect("statsMap.json do repositório");

        // Slugs reais do mapa, escolhidos em ordem estável (o mapa é um hash).
        let mut strat_slugs: Vec<&String> = map.strategem.keys().collect();
        strat_slugs.sort();
        let mut weapon_slugs: Vec<&String> = map.weapons.keys().collect();
        weapon_slugs.sort();
        let mut armor_slugs: Vec<&String> = map.armor.keys().collect();
        armor_slugs.sort();

        let stats = Stats {
            // Fora de ordem de propósito: quem ordena é o `meta_lists`.
            strategem: section(
                &[
                    (strat_slugs[0], 5.0),
                    (strat_slugs[1], 40.0),
                    ("slug-que-nao-existe", 99.0),
                ],
                4_009,
            ),
            weapons: section(&[(weapon_slugs[0], 12.0), (weapon_slugs[1], 30.0)], 0),
            armor: section(&[(armor_slugs[0], 7.0), (armor_slugs[1], 18.0)], 0),
        };

        let lists = meta_lists(&stats, &data, equipment, map);
        assert!(!lists.is_empty());
        assert_eq!(lists.games, 4_009);

        // Slug desconhecido não vira linha na tela.
        assert_eq!(lists.stratagems.len(), 2);
        assert_eq!(lists.stratagems[0].item, map.strategem[strat_slugs[1]]);
        assert_eq!(lists.stratagems[0].stat.loadouts_percentage, 40.0);
        assert!(lists
            .stratagems
            .iter()
            .all(|pick| data.by_id(pick.item).is_some()));

        // Cada arma cai na categoria que o mapa indica.
        for slug in [weapon_slugs[0], weapon_slugs[1]] {
            let reference = &map.weapons[slug];
            let slot = EquipSlot::from_key(&reference.cat).expect("categoria conhecida");
            assert!(lists
                .weapons(slot)
                .iter()
                .any(|pick| pick.item == reference.id));
        }
        // Categoria que não é de arma nunca tem lista.
        assert!(lists.weapons(EquipSlot::Armor).is_empty());

        assert_eq!(lists.passives.len(), 2);
        assert_eq!(lists.passives[0].item, map.armor[armor_slugs[1]]);
        assert!(MetaLists::default().is_empty());
    }

    #[test]
    fn a_weighted_pick_follows_the_pick_rate() {
        let popular = pick(1u32, 90.0);
        let rare = pick(2u32, 10.0);
        let list = [&popular, &rare];

        let mut rng = rand::rng();
        let hits = (0..ROLLS)
            .filter(|_| weighted(&list, &mut rng).unwrap().item == 1)
            .count();
        // Com 90/10 o esperado é 180 em 200; a folga cobre a variação do sorteio
        // sem deixar passar uma lista virada ao contrário.
        assert!(hits > ROLLS * 3 / 4, "o mais usado saiu só {hits} vezes");
        assert!(hits < ROLLS, "o menos usado nunca saiu");

        // Lista sem peso nenhum devolve o primeiro, como a v1.
        let zeroed = [&pick(7u32, 0.0), &pick(8u32, 0.0)];
        assert_eq!(weighted(&zeroed, &mut rng).unwrap().item, 7);
        assert!(weighted::<u32, _>(&[], &mut rng).is_none());
    }

    #[test]
    fn a_meta_build_only_takes_stratagems_from_the_top() {
        let data = data();
        let meta = meta(&data);
        let top: Vec<u32> = data.all().iter().take(15).map(|strat| strat.id).collect();
        let lists = lists_of(&top);

        for _ in 0..ROLLS {
            let build = roll_meta(&lists, rules(false, false), &data, &meta);
            assert!(build.stratagems.iter().all(Option::is_some));
            for id in build.stratagems.iter().flatten() {
                assert!(
                    top[..META_TOP_STRATS].contains(id),
                    "{id} está fora do top exibido"
                );
            }
            for index in 1..SLOT_COUNT {
                assert!(!build.stratagems[..index].contains(&build.stratagems[index]));
            }
        }
    }

    #[test]
    fn the_backpack_fallback_reaches_outside_the_top() {
        let data = data();
        let meta = meta(&data);

        // Topo sem apoio nem mochila; o resto da lista tem os dois, lá no fim.
        let plain: Vec<u32> = data
            .all()
            .iter()
            .filter(|strat| !meta.is_support(strat.id) && !meta.is_backpack(strat.id))
            .take(META_TOP_STRATS)
            .map(|strat| strat.id)
            .collect();
        let support = data
            .all()
            .iter()
            .find(|strat| meta.is_support(strat.id) && !meta.is_backpack(strat.id))
            .expect("uma arma de apoio sem mochila")
            .id;
        let backpack = data
            .all()
            .iter()
            .find(|strat| meta.is_backpack(strat.id) && !meta.is_support(strat.id))
            .expect("um item de mochila que não é apoio")
            .id;

        let mut ids = plain.clone();
        ids.extend([support, backpack]);
        let lists = lists_of(&ids);

        for _ in 0..ROLLS {
            let build = roll_meta(&lists, rules(true, false), &data, &meta);
            let equipped: Vec<u32> = build.stratagems.iter().flatten().copied().collect();
            assert!(
                equipped.iter().any(|id| meta.is_support(*id)),
                "build balanceada sem arma de apoio"
            );
            assert!(
                equipped.iter().any(|id| meta.is_backpack(*id)),
                "build balanceada sem item de mochila"
            );
        }
    }

    #[test]
    fn the_meta_gear_comes_from_the_top_of_each_list() {
        let data = data();
        let meta = meta(&data);
        let equipment = equipment();

        let weapon_ids = |slot: EquipSlot| -> Vec<String> {
            (0..4)
                .filter_map(|index| equipment.at(slot, index))
                .map(|item| item.id().to_string())
                .collect()
        };
        let mut weapons: [Vec<Pick<String>>; 3] = Default::default();
        for slot in [EquipSlot::Primary, EquipSlot::Secondary, EquipSlot::Grenade] {
            weapons[slot.index()] = weapon_ids(slot)
                .into_iter()
                .enumerate()
                .map(|(rank, id)| pick(id, 40.0 - rank as f64))
                .collect();
        }

        // Seis passivas reais: só as cinco primeiras podem sair na armadura.
        let mut names: Vec<String> = Vec::new();
        for armor in &equipment.armor {
            if !names.contains(&armor.passive) {
                names.push(armor.passive.clone());
            }
        }
        let passives: Vec<Pick<String>> = names
            .iter()
            .take(META_TOP_PASSIVES + 1)
            .enumerate()
            .map(|(rank, name)| pick(name.clone(), 30.0 - rank as f64))
            .collect();

        let lists = MetaLists {
            stratagems: lists_of(
                &data
                    .all()
                    .iter()
                    .take(12)
                    .map(|strat| strat.id)
                    .collect::<Vec<u32>>(),
            )
            .stratagems,
            weapons,
            passives,
            games: 1,
        };

        for _ in 0..ROLLS {
            let build = roll_meta(&lists, rules(false, false), &data, &meta);
            for slot in [EquipSlot::Primary, EquipSlot::Secondary, EquipSlot::Grenade] {
                let chosen = build.equip(slot).expect("arma sorteada");
                let top: Vec<&str> = lists
                    .weapons(slot)
                    .iter()
                    .take(META_TOP_WEAPONS)
                    .map(|pick| pick.item.as_str())
                    .collect();
                assert!(
                    top.contains(&chosen),
                    "{chosen} fora do top de {}",
                    slot.key()
                );
            }

            let armor = build
                .item(equipment, EquipSlot::Armor)
                .expect("armadura sorteada");
            let passive = &equipment.armor_by_id(armor.id()).unwrap().passive;
            assert!(
                names[..META_TOP_PASSIVES].contains(passive),
                "{passive} não está entre as passivas do topo"
            );
            // Capacete, capa e booster não têm estatística: só não podem faltar.
            for slot in [EquipSlot::Helmet, EquipSlot::Cape, EquipSlot::Booster] {
                assert!(
                    build.item(equipment, slot).is_some(),
                    "{} vazio",
                    slot.key()
                );
            }
        }
    }

    #[test]
    fn a_meta_build_keeps_what_is_locked() {
        let data = data();
        let meta = meta(&data);
        let ids: Vec<u32> = data.all().iter().take(12).map(|strat| strat.id).collect();
        let lists = lists_of(&ids);
        let first = roll_meta(&lists, rules(false, false), &data, &meta);

        let mut locks = Locks::default();
        locks.toggle_stratagem(2);
        locks.toggle_equip(EquipSlot::Booster);

        for _ in 0..ROLLS {
            let next = generate_meta(
                Some(&first),
                &locks,
                rules(false, false),
                &lists,
                &data,
                equipment(),
                &meta,
                &mut rand::rng(),
            );
            assert_eq!(next.stratagems[2], first.stratagems[2]);
            assert_eq!(
                next.equip(EquipSlot::Booster),
                first.equip(EquipSlot::Booster)
            );
            // E o travado não sai de novo noutro slot.
            let kept = next.stratagems[2];
            assert_eq!(
                next.stratagems.iter().filter(|slot| **slot == kept).count(),
                1
            );
        }
    }

    #[test]
    fn without_lists_no_stratagem_is_picked_but_the_gear_still_rolls() {
        // Resposta vazia, ou `statsMap.json` defasado: a tela nem oferece o
        // botão, mas o sorteio não pode entrar em pânico se chamado.
        let data = data();
        let meta = meta(&data);
        let build = roll_meta(&MetaLists::default(), rules(true, true), &data, &meta);

        assert!(build.stratagems.iter().all(Option::is_none));
        for slot in EquipSlot::ALL {
            assert!(build.item(equipment(), slot).is_some(), "{}", slot.key());
        }
    }

    // --- Personalizada ---

    fn strat(data: &GameData, index: usize) -> &Stratagem {
        &data.all()[index]
    }

    #[test]
    fn the_custom_grid_equips_removes_and_refuses_duplicates() {
        let data = data();
        let mut build = Build::default();
        let first = strat(&data, 0);

        assert!(custom_assign(&mut build, 0, first, &data));
        assert_eq!(build.stratagems[0], Some(first.id));

        // Clicar de novo no mesmo slot remove.
        assert!(custom_assign(&mut build, 0, first, &data));
        assert_eq!(build.stratagems[0], None);

        // Equipado noutro slot: recusado, e o card nem responde ao mouse.
        custom_assign(&mut build, 0, first, &data);
        assert!(!custom_assign(&mut build, 2, first, &data));
        assert!(custom_disabled(&build, 2, first, &data));
        assert!(
            !custom_disabled(&build, 0, first, &data),
            "no próprio slot ele continua clicável, para remover"
        );
    }

    #[test]
    fn the_custom_grid_respects_exclusive_tags() {
        let data = data();
        let mechas: Vec<&Stratagem> = data
            .all()
            .iter()
            .filter(|strat| strat.has_tag("Mecha"))
            .collect();
        let mut build = Build::default();
        custom_assign(&mut build, 0, mechas[0], &data);

        assert!(!custom_assign(&mut build, 1, mechas[1], &data));
        assert!(custom_disabled(&build, 1, mechas[1], &data));
        // Trocar um exo pelo outro no mesmo slot é permitido.
        assert!(custom_assign(&mut build, 0, mechas[1], &data));
        assert_eq!(build.stratagems[0], Some(mechas[1].id));
    }

    #[test]
    fn the_custom_list_is_ordered_by_category_and_filtered_by_the_search() {
        let data = data();
        let all = custom_list(&data, "");
        assert_eq!(all.len(), data.all().len());

        let tag_of = |id: u32| {
            data.by_id(id)
                .and_then(|strat| strat.primary_tag())
                .unwrap_or_default()
                .to_string()
        };
        let tags: Vec<String> = all.iter().map(|id| tag_of(*id)).collect();
        let first_supply = tags.iter().position(|tag| tag == "Supply").unwrap();
        let last_offensive = tags.iter().rposition(|tag| tag == "Offensive").unwrap();
        assert!(last_offensive < first_supply, "ofensivos vêm primeiro");

        // A busca ignora caixa e acento, como na aba de macros.
        let filtered = custom_list(&data, "  ORBITAL ");
        assert!(!filtered.is_empty() && filtered.len() < all.len());
        assert!(filtered
            .iter()
            .all(|id| data::normalize_text(&data.by_id(*id).unwrap().nome).contains("orbital")));
        assert!(custom_list(&data, "zzzz").is_empty());
    }

    // --- Builds salvas ---

    fn saved_build(data: &GameData) -> Build {
        let mut build = Build {
            stratagems: [Some(strat(data, 0).id), None, Some(strat(data, 3).id), None],
            ..Build::default()
        };
        build.set_equip(EquipSlot::Primary, Some("primary-ar-2-coyote".into()));
        build
    }

    #[test]
    fn saving_names_overwrites_by_name_and_ignores_an_empty_build() {
        let data = data();
        let mut loadouts = Vec::new();
        let build = saved_build(&data);

        assert!(!save(&mut loadouts, "", &Build::default()), "build vazia");
        assert!(loadouts.is_empty());

        assert!(save(&mut loadouts, "  ", &build));
        assert_eq!(loadouts[0].name, "Build 1", "sem nome, o padrão da v1");
        assert_eq!(loadouts[0].slot_ids, build.stratagems.to_vec());
        assert_eq!(
            loadouts[0].equip.as_ref().unwrap().get("primary"),
            Some(&"primary-ar-2-coyote".to_string())
        );

        // Mesmo nome com outra caixa sobrescreve em vez de duplicar.
        let mut other = build.clone();
        other.stratagems[1] = Some(strat(&data, 5).id);
        save(&mut loadouts, "BUILD 1", &other);
        assert_eq!(loadouts.len(), 1);
        assert_eq!(loadouts[0].slot_ids, other.stratagems.to_vec());
        assert_eq!(loadouts[0].name, "Build 1", "o nome original fica");

        save(&mut loadouts, "Bug Sweep", &build);
        assert_eq!(loadouts.len(), 2);
        assert_ne!(loadouts[0].id, loadouts[1].id);
    }

    #[test]
    fn a_build_without_gear_still_saves_the_equip_field() {
        // A v1 gravava `equip: {}`, e é isso que faz o aplicar limpar a tela.
        let data = data();
        let mut loadouts = Vec::new();
        let mut build = Build::default();
        build.stratagems[0] = Some(strat(&data, 0).id);

        save(&mut loadouts, "Só estratagemas", &build);
        assert_eq!(loadouts[0].equip.as_ref().map(HashMap::len), Some(0));
    }

    #[test]
    fn applying_a_saved_build_fills_the_slots_and_the_screen() {
        let data = data();
        let mut loadouts = Vec::new();
        let build = saved_build(&data);
        save(&mut loadouts, "Bug Sweep", &build);

        let applied = apply(&loadouts[0], None, &data, Some(equipment()));
        assert_eq!(applied.slots, build.stratagems);
        assert_eq!(applied.build, build);
    }

    #[test]
    fn an_old_build_without_gear_keeps_what_is_on_screen() {
        let data = data();
        let current = saved_build(&data);
        let old = Loadout {
            id: "1".into(),
            name: "Antiga".into(),
            slot_ids: vec![Some(strat(&data, 7).id), None, None, None],
            equip: None,
        };

        // Com build na tela, só os estratagemas mudam.
        let applied = apply(&old, Some(&current), &data, Some(equipment()));
        assert_eq!(applied.build.stratagems[0], Some(strat(&data, 7).id));
        assert_eq!(
            applied.build.equip(EquipSlot::Primary),
            current.equip(EquipSlot::Primary)
        );

        // Sem nada na tela, ela nasce só com os estratagemas.
        let applied = apply(&old, None, &data, Some(equipment()));
        assert_eq!(applied.build.equip(EquipSlot::Primary), None);
        assert_eq!(applied.build.stratagems[0], Some(strat(&data, 7).id));
    }

    #[test]
    fn applying_cleans_conflicts_and_drops_gear_that_left_the_game() {
        let data = data();
        let mechas: Vec<u32> = data
            .all()
            .iter()
            .filter(|strat| strat.has_tag("Mecha"))
            .map(|strat| strat.id)
            .collect();
        let loadout = Loadout {
            id: "1".into(),
            name: "Exos".into(),
            slot_ids: vec![Some(mechas[0]), Some(mechas[1]), Some(9_999), None],
            equip: Some(HashMap::from([
                ("primary".to_string(), "primary-nao-existe".to_string()),
                ("armor".to_string(), "armor-a-35-recon".to_string()),
            ])),
        };

        let applied = apply(&loadout, None, &data, Some(equipment()));
        assert_eq!(applied.slots[0], Some(mechas[0]));
        assert_eq!(applied.slots[1], None, "o segundo exo cai");
        assert_eq!(applied.slots[2], None, "id que sumiu do jogo cai");
        assert_eq!(applied.build.equip(EquipSlot::Primary), None);
        assert_eq!(
            applied.build.equip(EquipSlot::Armor),
            Some("armor-a-35-recon")
        );
    }

    #[test]
    fn the_chip_of_the_current_slots_is_the_active_one() {
        let data = data();
        let mut loadouts = Vec::new();
        let build = saved_build(&data);
        save(&mut loadouts, "Bug Sweep", &build);
        save(&mut loadouts, "Outra", &{
            let mut other = build.clone();
            other.stratagems[1] = Some(strat(&data, 9).id);
            other
        });

        assert_eq!(active_loadout(&loadouts, build.stratagems, &data), Some(0));
        assert_eq!(active_loadout(&loadouts, Slots::default(), &data), None);
        assert_eq!(active_loadout(&[], build.stratagems, &data), None);
    }

    #[test]
    fn a_loadout_that_needed_sanitizing_still_lights_up_after_apply() {
        let data = data();
        // Build salva com um id que sumiu do jogo: `apply` o descarta, e o chip
        // precisa comparar contra a mesma lista saneada.
        let loadout = Loadout {
            id: "1".into(),
            name: "Velha".into(),
            slot_ids: vec![Some(data.all()[0].id), Some(9_999), None, None],
            equip: None,
        };
        let applied = apply(&loadout, None, &data, None);
        assert_eq!(
            active_loadout(std::slice::from_ref(&loadout), applied.slots, &data),
            Some(0)
        );
    }

    #[test]
    fn a_blank_name_never_overwrites_a_surviving_build() {
        let data = data();
        let build = saved_build(&data);
        let mut loadouts = Vec::new();
        save(&mut loadouts, "Build 1", &build);
        save(&mut loadouts, "Build 2", &build);
        // "Build 1" foi excluída; o próximo nome em branco era "Build 2" na v1
        // — e sobrescreveria a sobrevivente.
        loadouts.remove(0);

        assert_eq!(default_name(&loadouts), "Build 1");
        save(&mut loadouts, "", &build);
        assert_eq!(loadouts.len(), 2, "a build nova não engole a existente");
    }

    #[test]
    fn assigning_to_an_out_of_range_slot_is_refused_not_a_panic() {
        let data = data();
        let mut build = Build::default();
        let strat = &data.all()[0];
        assert!(!custom_assign(&mut build, 4, strat, &data));
        assert_eq!(build.stratagems, Slots::default());
    }
}
