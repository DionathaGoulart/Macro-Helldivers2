//! Aba de builds: opções de sorteio, as sub-abas Meta/Aleatória/Personalizada/
//! Salvas e a build exibida.
//!
//! Como as demais abas, a tela é uma função do estado: entram settings, slots e
//! a build atual, saem nós e áreas clicáveis. As regras de sorteio e de montagem
//! moram em `builds.rs` (lógica pura); aqui ficam só o layout e a tradução de
//! clique em [`Action`].
//!
//! Comportamento portado de `legacy/src/renderer/components/BuildTab.jsx`.
//!
//! A sub-aba Meta não fala com a rede: ela registra o que precisa e a janela
//! dispara a consulta (`meta_stats.rs`), devolvendo a resposta por [`set_meta`].
//! Assim a construção da tela continua sendo função pura do estado.
//!
//! [`set_meta`]: BuildTab::set_meta

use rand::seq::IndexedRandom;
use rand::Rng;

use crate::builds::{self, Build, Locks, MetaLists, Rules, SaveError};
use crate::data::{self, EquipSlot, Equipment, GameData, Item, StratMeta, Stratagem};
use crate::i18n::{self, Tr};
use crate::loadouts::{self, Loadout};
use crate::meta_stats::{self, Faction, ItemStat, MetaResult, DIFFICULTIES};
use crate::settings::{Language, Settings, SLOT_COUNT};
use crate::shared::Slots;
use crate::ui::settings_tab::Change;
use crate::ui::theme::{self, font, motion};
use crate::ui::toolkit::{
    columns, grid_cell, grid_height, id, id_at, Align, Id, ImageStyle, Measure, Rect, TextStyle,
    Ui, Weight,
};
use crate::ui::widgets::{
    self, styles, ButtonVariant, CardHeader, CardMotion, CardState, Glyph, ItemCard, ReelFrame,
    Tone,
};

/// `screen-pad` da coluna de conteúdo.
const PAGE_PADDING: f32 = 24.0;
const PAGE_TOP: f32 = 20.0;
/// Espaço reservado à direita para a barra de rolagem da página.
const SCROLL_GUTTER: f32 = 14.0;
/// Espaço entre os blocos: cabe a sombra dura do de cima.
const SECTION_GAP: f32 = 24.0;

/// Grupo das sub-abas.
const SUBTAB_HEIGHT: f32 = 44.0;
const SUBTAB_PADDING: f32 = 4.0;
const SUBTAB_WIDTH: f32 = 140.0;

/// Linhas de opção.
const TOGGLE_HEIGHT: f32 = 58.0;
const TOGGLE_GAP: f32 = 10.0;
/// Altura de um rótulo de campo.
const LABEL_HEIGHT: f32 = 16.0;
const LABEL_GAP: f32 = 8.0;
/// Botão de sortear (CTA grande).
const GENERATE_HEIGHT: f32 = 52.0;
const GENERATE_WIDTH: f32 = 280.0;

/// Slot em edição da build personalizada.
const CUSTOM_SLOT_HEIGHT: f32 = 124.0;
const CUSTOM_SLOT_IMAGE: f32 = 56.0;
const CUSTOM_SLOT_BAR: f32 = 24.0;
/// Colunas da grade personalizada na janela em tamanho padrão, e o piso quando
/// ela encolhe. Como na aba de macros, a grade ganha colunas numa janela larga
/// em vez de esticar os tiles além do tamanho do ícone.
const CUSTOM_GRID_MIN_COLS: usize = 5;
/// Largura de referência de um tile da grade personalizada.
const CUSTOM_TILE_TARGET: f32 = 150.0;
/// Altura máxima da grade: passando disso ela rola por dentro.
const CUSTOM_GRID_MAX_HEIGHT: f32 = 440.0;
const GRID_GAP: f32 = 12.0;
/// Colunas do equipamento, e o espaço entre as linhas de campos.
const EQUIP_COLS: usize = 4;
const EQUIP_ROW_GAP: f32 = 12.0;
/// Linha de uma build salva: os quatro ícones, o nome e as ações.
const SAVED_ROW_HEIGHT: f32 = 64.0;
const SAVED_ROW_GAP: f32 = 10.0;
const SAVED_ROW_PADDING: f32 = 12.0;
const SAVED_ICON: f32 = 40.0;
const SAVED_ICON_GAP: f32 = 6.0;
/// Estado vazio da lista.
const SAVED_EMPTY_HEIGHT: f32 = 64.0;
/// Linha de estado do card da build atual: cabe o `icon-btn` de descartar.
const STATUS_HEIGHT: f32 = widgets::ICON_BTN_HEIGHT;
/// Folga do texto dos botões do salvar.
const BUTTON_PADDING: f32 = 20.0;
/// × que esvazia um slot em edição, e o de limpar a busca.
const CLEAR_SIZE: f32 = 20.0;
const SEARCH_CLEAR_SIZE: f32 = 22.0;

/// Botões de facção e de dificuldade da sub-aba Meta.
const META_FACTION_HEIGHT: f32 = 38.0;
const META_DIFFICULTY_HEIGHT: f32 = 32.0;
const META_CHOICE_GAP: f32 = 8.0;
const META_BLOCK_GAP: f32 = 18.0;
/// Botão de gerar a build meta.
const META_GENERATE_HEIGHT: f32 = 48.0;
/// Altura do aviso de carregando/erro.
const META_STATUS_HEIGHT: f32 = 56.0;
const META_COLUMN_GAP: f32 = 28.0;
/// Linha de item das listas.
const META_ROW_HEIGHT: f32 = 28.0;
const META_ROW_GAP: f32 = 0.0;
const META_CELL_GAP: f32 = 8.0;
/// Ícone, trilho de uso e as colunas de número.
const META_ICON: f32 = 22.0;
const META_BAR_WIDTH: f32 = 56.0;
const META_BAR_HEIGHT: f32 = 10.0;
const META_NEW_WIDTH: f32 = 38.0;
const META_CHANGE_WIDTH: f32 = 40.0;
const META_PERCENT_WIDTH: f32 = 46.0;

/// Largura mínima dos botões do salvar.
const SAVE_WIDTH: f32 = 120.0;
/// `maxLength={24}` do campo de nome da v1.
const NAME_MAX_CHARS: usize = 24;

/// Confirmação inline de ação destrutiva (§6.8): o botão vira `CONFIRMAR?`
/// por 3s e volta.
const CONFIRM_MS: u32 = 3_000;
/// Toast do salvar, aplicar e excluir: o mesmo tempo do toast do backup.
const NOTICE_MS: u32 = 2_500;
const TOAST_MARGIN: f32 = 20.0;

/// Sorteio animado: o primeiro card gira por `SPIN_MS`, cada card seguinte para
/// `SPIN_STAGGER_MS` depois do anterior, e a chegada dura `LAND_MS`. O último
/// dos 11 cards assenta em pouco mais de um segundo.
const SPIN_MS: u32 = 480;
const SPIN_STAGGER_MS: u32 = 55;
const LAND_MS: u32 = 220;
/// Quadros do rolo, contando o item que sai e o sorteado.
const REEL_FRAMES: usize = 6;
/// Itens sorteados só para passar pelo rolo de cada card. Poucos de propósito:
/// cada ícone novo é um bitmap de 256px decodificado e guardado no cache.
const REEL_SAMPLES: usize = 2;

/// Deslize da página até a build recém-sorteada.
const REVEAL_MS: u32 = 320;
const REVEAL_MARGIN: f32 = 12.0;
/// A build conta como à vista quando o topo dela está pelo menos isto acima da
/// borda de baixo: aí a página não se mexe.
const REVEAL_VISIBLE: f32 = 220.0;

/// Sub-abas da tela: as três da v1 e a lista das builds salvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubTab {
    Meta,
    Random,
    Custom,
    Saved,
}

impl SubTab {
    const ALL: [SubTab; 4] = [SubTab::Meta, SubTab::Random, SubTab::Custom, SubTab::Saved];

    fn label(self, tr: &'static Tr) -> &'static str {
        match self {
            SubTab::Meta => tr.build.sub_meta,
            SubTab::Random => tr.build.sub_random,
            SubTab::Custom => tr.build.sub_custom,
            SubTab::Saved => tr.build.sub_saved,
        }
    }
}

/// O que a janela faz depois de um clique na aba.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Só reconstruir a tela.
    Redraw,
    /// Gravar a preferência e espalhar seus efeitos.
    Setting(Change),
    /// Slots novos: gravar, refazer a tabela de atalhos e avisar o overlay.
    SlotsChanged(Slots),
    /// As builds salvas mudaram: gravar `loadouts.json` e avisar o overlay.
    LoadoutsChanged,
    /// Uma build foi gravada: além de salvar o arquivo, o campo de nome passa a
    /// mostrar o nome com que ela ficou.
    Saved(String),
    /// Uma build salva foi para os slots de macro; o campo de nome mostra qual.
    Applied { slots: Slots, name: String },
    /// Trazer um `EDIT` da aba para a frente.
    FocusEdit(Id),
    /// Trocar o texto de `EDIT`s da aba (esvaziar é trocar por "").
    SetEdits(Vec<(Id, String)>),
}

/// O que a aba precisa saber do resto do app.
pub struct Ctx<'a> {
    pub data: &'a GameData,
    pub settings: &'a Settings,
    pub slots: Slots,
    /// Campo de texto com o foco do teclado, se algum.
    pub focused_edit: Option<Id>,
}

impl Ctx<'_> {
    fn tr(&self) -> &'static Tr {
        i18n::tr(self.settings.language)
    }
}

// --- Ids ---

fn scroll_id() -> Id {
    id("build.scroll")
}

fn sub_tab_id(index: usize) -> Id {
    id_at("build.sub", index)
}

fn option_id(index: usize) -> Id {
    id_at("build.option", index)
}

fn generate_id() -> Id {
    id("build.generate")
}

fn faction_id(index: usize) -> Id {
    id_at("build.meta.faction", index)
}

fn difficulty_id(index: usize) -> Id {
    id_at("build.meta.difficulty", index)
}

fn meta_generate_id() -> Id {
    id("build.meta.generate")
}

fn apply_id() -> Id {
    id("build.apply")
}

/// "Rolar de novo" da barra da build atual.
fn reroll_id() -> Id {
    id("build.reroll")
}

fn import_slots_id() -> Id {
    id("build.custom.import")
}

fn reset_id() -> Id {
    id("build.custom.reset")
}

/// Id do campo de busca da grade personalizada. A janela precisa dele para
/// reconhecer o `EDIT` nativo.
pub fn search_id() -> Id {
    id("build.search")
}

fn clear_search_id() -> Id {
    id("build.search.clear")
}

fn grid_id() -> Id {
    id("build.grid")
}

/// Id do card de um estratagema na grade personalizada. Vem do id do
/// estratagema: o hover não pula de card quando a busca reordena a grade.
fn card_id(stratagem: u32) -> Id {
    id_at("build.card", stratagem as usize)
}

fn custom_slot_id(index: usize) -> Id {
    id_at("build.custom.slot", index)
}

fn custom_clear_id(index: usize) -> Id {
    id_at("build.custom.clear", index)
}

fn strat_lock_id(index: usize) -> Id {
    id_at("build.lock.strat", index)
}

fn equip_lock_id(slot: EquipSlot) -> Id {
    id_at("build.lock.equip", slot.index())
}

fn dropdown_id(slot: EquipSlot) -> Id {
    id_at("build.dropdown", slot.index())
}

/// Linha `index` da lista de `slot`. As categorias ficam em faixas separadas do
/// contador: nenhuma delas chega perto de mil itens.
fn dropdown_row_id(slot: EquipSlot, index: usize) -> Id {
    id_at("build.dropdown.row", slot.index() * 1_000 + index)
}

fn dropdown_scroll_id() -> Id {
    id("build.dropdown.scroll")
}

/// Área que fecha o dropdown ao receber um clique fora da lista.
fn scrim_id() -> Id {
    id("build.dropdown.scrim")
}

fn loadout_apply_id(index: usize) -> Id {
    id_at("build.loadout.apply", index)
}

fn loadout_edit_id(index: usize) -> Id {
    id_at("build.loadout.edit", index)
}

fn loadout_delete_id(index: usize) -> Id {
    id_at("build.loadout.delete", index)
}

/// Pulso do rolo de um card: estratagemas de 0 a 3, equipamento depois.
fn roll_id(card: usize) -> Id {
    id_at("build.roll", card)
}

/// Pulso dos 3s da confirmação inline.
fn confirm_flash_id() -> Id {
    id("build.confirm")
}

/// Pulso do toast.
fn notice_flash_id() -> Id {
    id("build.notice")
}

/// `TENTAR DE NOVO` do banner de erro da sub-aba Meta.
fn meta_retry_id() -> Id {
    id("build.meta.retry")
}

/// Id do campo com o nome da build. A janela precisa dele para reconhecer o
/// `EDIT` nativo.
pub fn name_id() -> Id {
    id("build.name")
}

fn save_id() -> Id {
    id("build.save")
}

fn save_changes_id() -> Id {
    id("build.save.changes")
}

fn save_as_new_id() -> Id {
    id("build.save.new")
}

fn discard_id() -> Id {
    id("build.discard")
}

// --- Estilos ---

fn hint_style() -> TextStyle {
    styles::hint()
}

/// Em que pé está a consulta da sub-aba Meta.
enum MetaState {
    /// Combinação ainda não pedida; a próxima construção registra o pedido.
    Idle,
    Loading,
    Ready(Box<MetaLists>),
    Failed,
}

/// Estado da sub-aba Meta. A facção e a dificuldade são escolha de sessão: a v1
/// também não as guardava.
struct MetaView {
    faction: Faction,
    difficulty: u8,
    state: MetaState,
    /// Consulta que a janela ainda precisa disparar.
    pending: Option<(Faction, u8)>,
}

impl MetaView {
    fn new() -> MetaView {
        MetaView {
            faction: Faction::default(),
            difficulty: DIFFICULTIES[0],
            state: MetaState::Idle,
            pending: None,
        }
    }

    fn key(&self) -> String {
        meta_stats::cache_key(self.faction, self.difficulty)
    }

    /// Troca de facção ou dificuldade: a consulta recomeça do zero.
    fn reset(&mut self) {
        self.state = MetaState::Idle;
        self.pending = None;
    }

    fn lists(&self) -> Option<&MetaLists> {
        match &self.state {
            // Resposta que não casou com nada dos nossos JSONs não tem o que
            // mostrar nem o que sortear: vale como indisponível.
            MetaState::Ready(lists) if !lists.is_empty() => Some(lists),
            _ => None,
        }
    }
}

/// Ação destrutiva esperando o segundo clique (§6.8).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Confirm {
    /// Salvar com o nome de uma build que já existe (id da dona).
    Replace(String),
    /// Excluir a build salva de tal id.
    Delete(String),
}

/// Como a build exibida está em relação às salvas.
#[derive(Debug, Clone, Copy)]
enum SaveState<'a> {
    /// Build nova, sem build salva por trás.
    New,
    /// Veio de uma build salva e continua igual a ela.
    Saved(&'a Loadout),
    /// Veio de uma build salva e mudou: itens ou o nome no campo.
    Changed(&'a Loadout),
}

/// Sorteio em animação.
struct Roll {
    /// Os pulsos ainda não foram acesos: quem os dispara é a próxima construção,
    /// com o relógio da passagem.
    pending: bool,
    /// Quadros do rolo de cada card, na ordem da tela (os 4 estratagemas, depois
    /// o equipamento). Vazio é card parado: travado, ou sem item.
    reels: Vec<Vec<ReelFrame>>,
}

/// Duração do pulso do card: o giro, o atraso dos anteriores e a chegada.
fn spin_ms(card: usize) -> u32 {
    SPIN_MS + SPIN_STAGGER_MS * card as u32
}

fn roll_ms(card: usize) -> u32 {
    spin_ms(card) + LAND_MS
}

/// Estado da aba entre passagens de construção.
pub struct BuildTab {
    sub: SubTab,
    /// A build exibida: sorteada, montada à mão ou vinda de uma build salva.
    build: Option<Build>,
    locks: Locks,
    /// Slot da build personalizada que os cliques na grade equipam.
    custom_slot: usize,
    search: String,
    /// Grade personalizada já filtrada; refeita só quando a busca muda.
    list: Vec<u32>,
    dirty: bool,
    /// Categoria com a lista aberta e o retângulo do campo que a abriu.
    open: Option<EquipSlot>,
    field: Rect,
    /// Classificação da wiki, montada na primeira visita (com o `equipment.json`).
    meta: Option<StratMeta>,
    /// Facção, dificuldade e estatísticas da sub-aba Meta.
    stats: MetaView,
    /// Builds salvas, lidas do disco na primeira visita à aba.
    loadouts: Vec<Loadout>,
    loaded: bool,
    /// Texto do campo de nome da build atual.
    name: String,
    /// Build salva de onde a exibida veio (aplicada, editada ou recém-salva),
    /// pelo id. É com ela que o card da build atual compara para dizer se há
    /// alteração a salvar; sortear outra build desfaz o vínculo.
    linked: Option<String>,
    /// O nome no campo é o de outra build salva: o salvar recusou.
    taken: bool,
    /// Ação destrutiva armada, e se o pulso dos 3s ainda precisa ser aceso.
    confirm: Option<Confirm>,
    confirm_pending: bool,
    /// Texto do toast, e se o pulso dele ainda precisa ser aceso.
    notice: Option<String>,
    notice_pending: bool,
    roll: Option<Roll>,
    /// Levar a página até a build na próxima construção.
    reveal: bool,
}

impl Default for BuildTab {
    fn default() -> BuildTab {
        BuildTab::new()
    }
}

impl BuildTab {
    pub fn new() -> BuildTab {
        BuildTab {
            sub: SubTab::Meta,
            build: None,
            locks: Locks::default(),
            custom_slot: 0,
            search: String::new(),
            list: Vec::new(),
            dirty: true,
            open: None,
            field: Rect::ZERO,
            meta: None,
            stats: MetaView::new(),
            loadouts: Vec::new(),
            loaded: false,
            name: String::new(),
            linked: None,
            taken: false,
            confirm: None,
            confirm_pending: false,
            notice: None,
            notice_pending: false,
            roll: None,
            reveal: false,
        }
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    /// Texto novo vindo do `EDIT` nativo.
    pub fn set_search(&mut self, text: String) {
        if self.search == text {
            return;
        }
        self.search = text;
        self.dirty = true;
    }

    /// Nome novo vindo do `EDIT` nativo.
    pub fn set_name(&mut self, text: String) {
        // O campo da v1 tinha `maxLength={24}`.
        let name: String = text.chars().take(NAME_MAX_CHARS).collect();
        if name == self.name {
            return;
        }
        self.name = name;
        // Nome novo: o aviso de nome ocupado e a pergunta de substituir eram
        // sobre o anterior.
        self.taken = false;
        if matches!(self.confirm, Some(Confirm::Replace(_))) {
            self.confirm = None;
        }
    }

    /// Texto que a aba tem para um `EDIT` dela. A janela o põe no filho nativo
    /// quando o cria: um nome preenchido pela aba antes de o campo existir não
    /// pode aparecer em branco.
    pub fn edit_text(&self, id: Id) -> Option<&str> {
        if id == search_id() {
            Some(&self.search)
        } else if id == name_id() {
            Some(&self.name)
        } else {
            None
        }
    }

    /// Builds salvas, para a janela gravá-las depois de uma mudança.
    pub fn loadouts(&self) -> &[Loadout] {
        &self.loadouts
    }

    /// Relê o `loadouts.json`: a importação de backup o reescreve por fora.
    pub fn reload_loadouts(&mut self) {
        self.loadouts = loadouts::load_loadouts();
        self.loaded = true;
        // O arquivo novo pode não ter mais a build vinculada, nem a armada.
        if let Some(id) = &self.linked {
            if builds::index_of(&self.loadouts, id).is_none() {
                self.linked = None;
            }
        }
        self.confirm = None;
    }

    /// Primeira visita à aba: as builds salvas saem do disco.
    fn ensure_loaded(&mut self) {
        if !self.loaded {
            self.reload_loadouts();
        }
    }

    /// Consulta de estatísticas que a janela precisa disparar, se houver.
    ///
    /// A aba não fala com a rede nem lê o cache: ela só registra o que falta, e
    /// quem resolve é a janela, que tem o `Shared` para o worker responder.
    pub fn take_meta_request(&mut self) -> Option<(Faction, u8)> {
        self.stats.pending.take()
    }

    /// Resposta de uma consulta. Resposta de outra combinação é descartada: o
    /// usuário pode ter trocado de facção enquanto a rede respondia.
    pub fn set_meta(&mut self, result: MetaResult, data: &GameData) {
        if result.key != self.stats.key() {
            return;
        }
        self.stats.state = match result.stats {
            Some(stats) => match (data::equipment(), data::stats_map()) {
                (Some(equipment), Some(map)) => {
                    MetaState::Ready(Box::new(builds::meta_lists(&stats, data, equipment, map)))
                }
                // Sem os JSONs de apoio não há como resolver slug nenhum.
                _ => MetaState::Failed,
            },
            None => MetaState::Failed,
        };
    }

    /// Tecla recebida pela janela. Só o Esc interessa, e só com a lista aberta.
    pub fn on_key(&mut self, vk: u16) -> Option<Action> {
        if vk == crate::keys::VK_ESCAPE && self.open.is_some() {
            self.open = None;
            return Some(Action::Redraw);
        }
        None
    }

    // --- Construção ---

    pub fn build(&mut self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        if self.dirty {
            self.list = builds::custom_list(ctx.data, &self.search);
            self.dirty = false;
        }
        self.ensure_loaded();
        self.tick(ui);
        // Primeira visita à aba: é aqui que `equipment.json` sai do disco (R1).
        let equipment = data::equipment();

        let view = Rect::new(
            area.x + PAGE_PADDING,
            area.y + PAGE_TOP,
            area.w - PAGE_PADDING * 2.0,
            area.h - PAGE_TOP,
        );
        let width = view.w - SCROLL_GUTTER;
        let offset = ui.scroll_begin(scroll_id(), view);
        let mut y = view.y - offset;

        self.sub_tabs(ui, Rect::new(view.x, y, width, SUBTAB_HEIGHT), ctx);
        y += SUBTAB_HEIGHT + SECTION_GAP;

        // As opções valem para os dois sorteios, e não para a montagem à mão.
        if matches!(self.sub, SubTab::Meta | SubTab::Random) {
            let height = options_height();
            self.options_card(ui, Rect::new(view.x, y, width, height), ctx);
            y += height + SECTION_GAP;
        }

        match self.sub {
            SubTab::Meta => {
                // Primeira visita (ou combinação recém-escolhida): a consulta é
                // registrada aqui e disparada pela janela logo depois.
                if matches!(self.stats.state, MetaState::Idle) {
                    self.stats.state = MetaState::Loading;
                    self.stats.pending = Some((self.stats.faction, self.stats.difficulty));
                }
                let height = self.meta_height();
                self.meta_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
                y += height + SECTION_GAP;
            }
            SubTab::Random => {
                let height = random_height(measure, width, ctx);
                self.random_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
                y += height + SECTION_GAP;
            }
            SubTab::Custom => {
                let height = self.custom_height(measure, width, ctx, equipment);
                self.custom_card(
                    ui,
                    measure,
                    Rect::new(view.x, y, width, height),
                    ctx,
                    equipment,
                );
                y += height + SECTION_GAP;
            }
            SubTab::Saved => {
                let height = self.saved_height(measure, width, ctx);
                self.saved_card(ui, measure, Rect::new(view.x, y, width, height), view, ctx);
                y += height + SECTION_GAP;
            }
        }

        // A build exibida: o card do salvar em cima (é o que se faz com ela) e
        // os itens embaixo. Na lista das salvas ela não aparece: lá cada linha
        // já mostra a sua.
        if let Some(build) = self.build.clone().filter(|_| self.sub != SubTab::Saved) {
            if std::mem::take(&mut self.reveal) {
                reveal(ui, view, y, offset);
            }
            let height = current_height();
            self.current_card(
                ui,
                measure,
                Rect::new(view.x, y, width, height),
                &build,
                ctx,
            );
            y += height + SECTION_GAP;

            let height = stratagems_height(measure, width, &build, ctx);
            self.stratagems_card(
                ui,
                measure,
                Rect::new(view.x, y, width, height),
                &build,
                ctx,
            );
            y += height + SECTION_GAP;

            if let Some(equipment) = equipment {
                let cards = self.equipment_cards(&build, equipment, ctx);
                if !cards.is_empty() {
                    let height = equipment_height(measure, width, &cards);
                    self.equipment_card(
                        ui,
                        measure,
                        Rect::new(view.x, y, width, height),
                        &cards,
                        ctx,
                    );
                    y += height + SECTION_GAP;
                }
            }
        }

        // A sombra do último painel e um respiro antes do rodapé.
        ui.scroll_end(
            scroll_id(),
            view,
            y - SECTION_GAP + theme::SHADOW + PAGE_TOP - (view.y - offset),
        );

        // A lista aberta sai por cima de tudo e fora do recorte da página: ela
        // precisa cobrir o que vem depois do campo que a abriu.
        if let Some(slot) = self.open {
            self.dropdown_list(ui, area, slot, ctx, equipment);
        }
        self.notice_toast(ui, area, ctx);
    }

    /// Relógio da aba: acende os pulsos que um clique pediu (o clique não tem o
    /// `Ui`) e encerra o que já venceu, a confirmação de 3s e o sorteio.
    fn tick(&mut self, ui: &mut Ui) {
        if std::mem::take(&mut self.confirm_pending) {
            ui.flash(confirm_flash_id(), CONFIRM_MS);
        }
        if self.confirm.is_some() && ui.anim(confirm_flash_id(), CONFIRM_MS) <= 0.0 {
            self.confirm = None;
        }

        if let Some(roll) = &mut self.roll {
            if std::mem::take(&mut roll.pending) && !ui.reduced_motion() {
                for (card, reel) in roll.reels.iter().enumerate() {
                    if !reel.is_empty() {
                        ui.flash(roll_id(card), roll_ms(card));
                    }
                }
            }
        }
        let done = self.roll.as_ref().is_some_and(|roll| {
            roll.reels
                .iter()
                .enumerate()
                .all(|(card, reel)| reel.is_empty() || ui.anim(roll_id(card), roll_ms(card)) <= 0.0)
        });
        if done {
            self.roll = None;
        }
    }

    /// Em que ponto do sorteio está o card: `Spin` com o rolo descendo e
    /// desacelerando até o item novo, `Land` logo depois de parar.
    fn card_motion(&self, ui: &mut Ui, card: usize) -> CardMotion<'_> {
        let Some(frames) = self
            .roll
            .as_ref()
            .and_then(|roll| roll.reels.get(card))
            .filter(|reel| !reel.is_empty())
        else {
            return CardMotion::Still;
        };
        let duration = roll_ms(card);
        let left = ui.anim(roll_id(card), duration);
        if left <= 0.0 {
            return CardMotion::Still;
        }
        let elapsed = (1.0 - left) * duration as f32;
        let spin = spin_ms(card) as f32;
        if elapsed < spin {
            // O ease-out do guia, sem overshoot: rápido no começo, parando
            // exatamente no último quadro.
            let travel = (frames.len() - 1) as f32;
            CardMotion::Spin {
                frames,
                position: travel * motion::ease_out(elapsed / spin),
            }
        } else {
            CardMotion::Land((elapsed - spin) / LAND_MS as f32)
        }
    }

    /// Grupo das sub-abas, centralizado: um controle segmentado com moldura e
    /// sombra `sm`. A das salvas leva a contagem.
    fn sub_tabs(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let palette = theme::palette();
        let count = SubTab::ALL.len() as f32;
        let width = SUBTAB_WIDTH * count + SUBTAB_PADDING * 2.0;
        let group = rect.centered(width, SUBTAB_HEIGHT);
        ui.fill(
            group.translate(theme::SHADOW_SM, theme::SHADOW_SM),
            palette.shadow,
        );
        ui.fill(group, palette.base_200);
        ui.stroke(group, theme::BORDER, palette.base_300);

        let inner = group.inset(SUBTAB_PADDING + 1.0);
        for (index, sub) in SubTab::ALL.into_iter().enumerate() {
            let cell = Rect::new(
                inner.x + (inner.w / count) * index as f32,
                inner.y,
                inner.w / count,
                inner.h,
            );
            let label = match sub {
                SubTab::Saved if !self.loadouts.is_empty() => {
                    format!("{} ({})", sub.label(ctx.tr()), self.loadouts.len())
                }
                _ => sub.label(ctx.tr()).to_string(),
            };
            sub_tab(ui, sub_tab_id(index), cell, &label, self.sub == sub);
        }
    }

    /// Card com as três opções de sorteio.
    fn options_card(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let mut content = widgets::card(ui, rect, None);
        for (index, option) in options(ctx).into_iter().enumerate() {
            let row = content.cut_top(TOGGLE_HEIGHT);
            widgets::toggle_row(
                ui,
                option_id(index),
                row,
                option.title,
                option.description,
                option.on,
            );
            content.skip_top(TOGGLE_GAP);
        }
    }

    /// Sub-aba Meta: facção × dificuldade e as listas do helldive.live.
    fn meta_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let palette = theme::palette();
        let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.build.meta, "log")));

        let row = content.cut_top(META_FACTION_HEIGHT);
        let cells = columns(row, Faction::ALL.len(), META_CHOICE_GAP);
        for ((index, faction), cell) in Faction::ALL.into_iter().enumerate().zip(cells) {
            widgets::choice_button(
                ui,
                faction_id(index),
                cell,
                tr.build.faction(faction),
                self.stats.faction == faction,
            );
        }
        content.skip_top(META_CHOICE_GAP);

        let row = content.cut_top(META_DIFFICULTY_HEIGHT);
        let cells = columns(row, DIFFICULTIES.len(), META_CHOICE_GAP);
        for ((index, difficulty), cell) in DIFFICULTIES.into_iter().enumerate().zip(cells) {
            let label = match difficulty {
                0 => tr.build.meta_difficulty_all.to_string(),
                level => format!("D{level}"),
            };
            widgets::choice_button(
                ui,
                difficulty_id(index),
                cell,
                &label,
                self.stats.difficulty == difficulty,
            );
        }
        content.skip_top(META_BLOCK_GAP);

        match self.stats.lists() {
            Some(lists) => self.meta_body(ui, measure, content, lists, ctx),
            None => {
                let status = content.with_h(META_STATUS_HEIGHT);
                match self.stats.state {
                    // O "carregando" do guia: caret piscando, sem spinner.
                    MetaState::Loading | MetaState::Idle => widgets::caret_text(
                        ui,
                        measure,
                        status,
                        tr.build.meta_loading,
                        styles::micro().align(Align::Center).middle(),
                        palette.muted,
                    ),
                    // Erro de carregamento (§8): banner no lugar do conteúdo e
                    // o `TENTAR DE NOVO`.
                    _ => {
                        let mut banner = status;
                        let width = widgets::icon_btn_width(measure, tr.settings.update_retry);
                        let retry = banner.cut_right(width).middle_row(widgets::ICON_BTN_HEIGHT);
                        banner.cut_right(12.0);
                        widgets::alert(ui, banner, palette.error, tr.build.meta_error, "");
                        widgets::icon_btn(
                            ui,
                            meta_retry_id(),
                            retry,
                            Glyph::Text(tr.settings.update_retry),
                            Tone::Plain,
                        );
                    }
                }
            }
        }
    }

    /// Botão de gerar, as duas colunas de listas e o crédito ao site.
    fn meta_body(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        lists: &MetaLists,
        ctx: &Ctx,
    ) {
        let tr = ctx.tr();
        let mut content = rect;
        widgets::button(
            ui,
            meta_generate_id(),
            content.cut_top(META_GENERATE_HEIGHT),
            tr.build.meta_generate,
            ButtonVariant::Primary,
        );
        content.skip_top(META_BLOCK_GAP + 4.0);

        let area = content.cut_top(meta_columns_height(lists));
        let pair = columns(area, 2, META_COLUMN_GAP);
        self.meta_stratagems(ui, measure, pair[0], lists, ctx);
        self.meta_gear(ui, measure, pair[1], lists, ctx);

        content.skip_top(META_BLOCK_GAP);
        ui.text(
            content.cut_top(LABEL_HEIGHT),
            format!(
                "{} \u{00B7} {} {}",
                tr.build.meta_credit,
                grouped(lists.games, ctx.settings.language),
                tr.build.meta_games
            )
            .to_uppercase(),
            styles::micro().align(Align::Center),
            theme::palette().muted,
        );
    }

    /// Coluna da esquerda: os estratagemas mais usados, com a barra proporcional
    /// ao primeiro colocado.
    fn meta_stratagems(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        lists: &MetaLists,
        ctx: &Ctx,
    ) {
        let mut cursor = rect;
        meta_section_label(ui, &mut cursor, ctx.tr().build.meta_top_strats);

        let best = lists
            .stratagems
            .first()
            .map(|pick| pick.stat.loadouts_percentage)
            .unwrap_or_default();
        for (index, pick) in lists
            .stratagems
            .iter()
            .take(builds::META_TOP_STRATS)
            .enumerate()
        {
            let row = meta_row(&mut cursor, index);
            let Some(strat) = ctx.data.by_id(pick.item) else {
                continue;
            };
            meta_stratagem_row(ui, measure, row, strat, pick.stat, best, ctx);
        }
    }

    /// Coluna da direita: armas por categoria e as passivas mais usadas.
    fn meta_gear(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        lists: &MetaLists,
        ctx: &Ctx,
    ) {
        let tr = ctx.tr();
        let equipment = data::equipment();
        let mut cursor = rect;

        for slot in [EquipSlot::Primary, EquipSlot::Secondary, EquipSlot::Grenade] {
            meta_section_label(ui, &mut cursor, tr.build.equip_label(slot));
            for (index, pick) in lists
                .weapons(slot)
                .iter()
                .take(builds::META_TOP_WEAPONS)
                .enumerate()
            {
                let row = meta_row(&mut cursor, index);
                let item = equipment.and_then(|equipment| equipment.find(slot, &pick.item));
                let Some(item) = item else {
                    continue;
                };
                meta_item_row(
                    ui,
                    measure,
                    row,
                    item.nome(),
                    Some(item.imagem()),
                    pick.stat,
                );
            }
            cursor.skip_top(META_BLOCK_GAP);
        }

        meta_section_label(ui, &mut cursor, tr.build.meta_top_passives);
        for (index, pick) in lists
            .passives
            .iter()
            .take(builds::META_TOP_PASSIVES)
            .enumerate()
        {
            let row = meta_row(&mut cursor, index);
            let icon = equipment
                .and_then(|equipment| equipment.passive(&pick.item))
                .map(|passive| passive.imagem.as_str());
            meta_item_row(ui, measure, row, &pick.item, icon, pick.stat);
        }
    }

    /// Sub-aba Aleatória: o CTA de sortear e a explicação.
    fn random_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let mut content = widgets::card(ui, rect, None);
        let button = content
            .cut_top(GENERATE_HEIGHT)
            .centered(GENERATE_WIDTH, GENERATE_HEIGHT);
        widgets::button(
            ui,
            generate_id(),
            button,
            tr.build.generate,
            ButtonVariant::Primary,
        );
        content.skip_top(LABEL_GAP + 6.0);

        let height = measure.text_size(tr.build.hint, hint_style(), content.w).1;
        ui.text(
            content.with_h(height),
            tr.build.hint,
            hint_style().align(Align::Center),
            theme::palette().muted,
        );
    }

    // --- Sub-aba personalizada ---

    fn custom_height(
        &self,
        measure: &mut dyn Measure,
        width: f32,
        ctx: &Ctx,
        equipment: Option<&Equipment>,
    ) -> f32 {
        let inner = width - widgets::CARD_PADDING * 2.0;
        let hint = measure
            .text_size(ctx.tr().build.custom_hint, hint_style(), inner)
            .1;
        let grid = self.custom_grid_height(inner);
        let equip = if equipment.is_some() {
            LABEL_HEIGHT
                + LABEL_GAP
                + equip_rows() * (LABEL_HEIGHT + 4.0 + widgets::CONTROL_HEIGHT)
                + (equip_rows() - 1.0) * EQUIP_ROW_GAP
                + SECTION_GAP
        } else {
            0.0
        };
        widgets::card_chrome(true)
            + hint
            + LABEL_GAP
            + CUSTOM_SLOT_HEIGHT
            + SECTION_GAP
            + widgets::CONTROL_HEIGHT
            + LABEL_GAP
            + 4.0
            + grid
            + equip
    }

    fn custom_grid_height(&self, width: f32) -> f32 {
        if self.list.is_empty() {
            return 64.0;
        }
        let cell = custom_cell(width);
        // A folga de baixo é a sombra do tile, que também precisa caber.
        (grid_height(
            self.list.len(),
            custom_cols(width),
            widgets::tile_height(cell),
            GRID_GAP,
        ) + theme::SHADOW)
            .min(CUSTOM_GRID_MAX_HEIGHT)
    }

    fn custom_card(
        &mut self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        ctx: &Ctx,
        equipment: Option<&Equipment>,
    ) {
        let tr = ctx.tr();
        let palette = theme::palette();
        let mut content =
            widgets::card(ui, rect, Some(CardHeader::new(tr.build.custom_title, "db")));
        self.custom_header_buttons(ui, measure, rect, ctx);

        let height = measure
            .text_size(tr.build.custom_hint, hint_style(), content.w)
            .1;
        ui.text(
            content.cut_top(height),
            tr.build.custom_hint,
            hint_style(),
            palette.muted,
        );
        content.skip_top(LABEL_GAP);

        // Os quatro slots em edição.
        let row = content.cut_top(CUSTOM_SLOT_HEIGHT);
        for (index, cell) in columns(row, SLOT_COUNT, GRID_GAP).into_iter().enumerate() {
            self.custom_slot(ui, index, cell, ctx);
        }
        content.skip_top(SECTION_GAP);

        // Busca e grade.
        let search = content.cut_top(widgets::CONTROL_HEIGHT);
        self.search_field(ui, search, ctx);
        content.skip_top(LABEL_GAP + 4.0);

        let grid = content.cut_top(self.custom_grid_height(content.w));
        if self.list.is_empty() {
            empty_state(
                ui,
                grid,
                tr.macros.nothing_here,
                &format!(
                    "{} \u{201c}{}\u{201d}",
                    tr.macros.search_no_results,
                    self.search.trim()
                ),
            );
        } else {
            self.custom_grid(ui, grid, ctx);
        }

        // Equipamento opcional.
        let Some(equipment) = equipment else {
            return;
        };
        content.skip_top(SECTION_GAP);
        widgets::section_label(ui, content.cut_top(LABEL_HEIGHT), tr.build.custom_equipment);
        content.skip_top(LABEL_GAP);
        self.equipment_fields(ui, content, ctx, equipment);
    }

    /// "Usar slots atuais" e "Limpar tudo", como `icon-btn` na barra de
    /// título do painel.
    fn custom_header_buttons(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let mut cursor = widgets::card_actions(rect);
        let width = widgets::icon_btn_width(measure, tr.build.custom_clear);
        let reset = cursor.cut_right(width);
        cursor.cut_right(8.0);
        let width = widgets::icon_btn_width(measure, tr.build.custom_import);
        let import = cursor.cut_right(width);
        widgets::icon_btn(
            ui,
            import_slots_id(),
            import,
            Glyph::Text(tr.build.custom_import),
            Tone::Plain,
        );
        widgets::icon_btn(
            ui,
            reset_id(),
            reset,
            Glyph::Text(tr.build.custom_clear),
            Tone::Danger,
        );
    }

    /// Um dos quatro slots em edição da build personalizada: caixa aninhada
    /// com a barra da categoria. O slot em edição fica erguido, com a barra em
    /// accent; os outros levantam no hover.
    fn custom_slot(&self, ui: &mut Ui, index: usize, rect: Rect, ctx: &Ctx) {
        let palette = theme::palette();
        let id = custom_slot_id(index);
        let active = self.custom_slot == index;
        let hover = ui.fade(id, ui.is_hot(id), motion::HOVER_MS);
        let strat = self
            .build
            .as_ref()
            .and_then(|build| build.stratagems[index])
            .and_then(|id| ctx.data.by_id(id));

        let lift = if active || ui.is_pressed(id) {
            0.0
        } else {
            theme::LIFT * hover
        };
        let face = rect.translate(0.0, -lift);
        let shadow = if active {
            theme::SHADOW
        } else {
            theme::SHADOW_SM + (theme::SHADOW - theme::SHADOW_SM) * hover
        };
        ui.fill(face.translate(shadow, shadow), palette.shadow);
        ui.fill(face, palette.base_100);

        let mut content = face;
        let bar = content.cut_top(CUSTOM_SLOT_BAR);
        if active {
            ui.fill(bar, palette.accent);
        }
        ui.fill(
            Rect::new(bar.x, bar.bottom() - theme::BORDER, bar.w, theme::BORDER),
            palette.base_300,
        );
        ui.text(
            bar,
            format!("{} {}", ctx.tr().build.stratagem, index + 1).to_uppercase(),
            styles::micro().align(Align::Center).middle(),
            if active {
                palette.accent_content
            } else {
                palette.muted
            },
        );

        let mut content = content.inset(8.0);
        let picture = content
            .cut_top(CUSTOM_SLOT_IMAGE)
            .centered(CUSTOM_SLOT_IMAGE, CUSTOM_SLOT_IMAGE);
        match strat {
            Some(strat) => {
                ui.image(picture, format!("icons/{}", strat.imagem), 1.0);
                ui.stroke(picture, theme::BORDER, palette.base_300);
            }
            None => ui.text(
                picture,
                widgets::bracketed(ctx.tr().macros.empty),
                styles::micro().align(Align::Center).middle(),
                palette.muted,
            ),
        }
        content.skip_top(6.0);
        ui.text(
            content,
            strat
                .map(|strat| strat.nome.to_uppercase())
                .unwrap_or_else(|| "-".into()),
            TextStyle::new(font::SIZE_MICRO, Weight::Black)
                .align(Align::Center)
                .wrap(),
            palette.content,
        );
        ui.stroke(face, theme::BORDER, palette.base_300);
        ui.hit(id, rect);

        // O × sai por cima e é registrado depois, então ganha a sobreposição.
        if strat.is_some() {
            let clear = custom_clear_id(index);
            let button = Rect::new(
                face.right() - CLEAR_SIZE + 4.0,
                face.y - 6.0,
                CLEAR_SIZE,
                CLEAR_SIZE,
            );
            widgets::icon_btn(ui, clear, button, Glyph::Close, Tone::Danger);
        }
    }

    fn search_field(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let focused = ctx.focused_edit == Some(search_id());
        let placeholder =
            (self.search.is_empty() && !focused).then_some(ctx.tr().macros.search_placeholder);
        let reserve = if self.search.is_empty() {
            0.0
        } else {
            SEARCH_CLEAR_SIZE
        };
        widgets::edit_host(ui, search_id(), rect, focused, placeholder, reserve);

        if self.search.is_empty() {
            return;
        }
        let button = Rect::new(
            rect.right() - SEARCH_CLEAR_SIZE - 9.0,
            rect.center_y() - SEARCH_CLEAR_SIZE / 2.0,
            SEARCH_CLEAR_SIZE,
            SEARCH_CLEAR_SIZE,
        );
        widgets::icon_btn(ui, clear_search_id(), button, Glyph::Close, Tone::Plain);
    }

    /// Grade de 5 colunas, rolável, com as mesmas regras de clique da aba de
    /// macros, só que mexendo na build, e não nos slots.
    fn custom_grid(&self, ui: &mut Ui, view: Rect, ctx: &Ctx) {
        let cell = custom_cell(view.w);
        let cols = custom_cols(view.w);
        let content = grid_height(self.list.len(), cols, widgets::tile_height(cell), GRID_GAP);
        let empty = Build::default();
        let build = self.build.as_ref().unwrap_or(&empty);

        let content = content + theme::SHADOW;
        let offset = ui.scroll_begin(grid_id(), view);
        for (index, strat_id) in self.list.iter().enumerate() {
            let rect = grid_cell(
                Rect::new(view.x, view.y - offset, view.w, view.h),
                cols,
                widgets::tile_height(cell),
                GRID_GAP,
                index,
            );
            // Fora da janela visível não há o que desenhar.
            if rect.bottom() < view.y || rect.y > view.bottom() {
                continue;
            }
            let Some(strat) = ctx.data.by_id(*strat_id) else {
                continue;
            };
            widgets::stratagem_card(
                ui,
                card_id(*strat_id),
                rect,
                strat,
                CardState {
                    disabled: builds::custom_disabled(build, self.custom_slot, strat, ctx.data),
                    in_active_slot: build.stratagems[self.custom_slot] == Some(*strat_id),
                },
            );
        }
        ui.scroll_end(grid_id(), view, content);
    }

    /// Os sete campos de equipamento da build personalizada.
    fn equipment_fields(&mut self, ui: &mut Ui, rect: Rect, ctx: &Ctx, equipment: &Equipment) {
        let tr = ctx.tr();
        let cell_height = LABEL_HEIGHT + 4.0 + widgets::CONTROL_HEIGHT;
        for (index, slot) in EquipSlot::ALL.into_iter().enumerate() {
            let mut cell = grid_cell(rect, EQUIP_COLS, cell_height, EQUIP_ROW_GAP, index);
            ui.text(
                cell.cut_top(LABEL_HEIGHT),
                tr.build.equip_label(slot).to_uppercase(),
                styles::micro(),
                theme::palette().muted,
            );
            cell.skip_top(4.0);

            let field = cell.with_h(widgets::CONTROL_HEIGHT);
            let value = self
                .build
                .as_ref()
                .and_then(|build| build.item(equipment, slot))
                .map(Item::nome)
                .unwrap_or(tr.build.equip_none);
            widgets::dropdown_field(ui, dropdown_id(slot), field, value, self.open == Some(slot));
            if self.open == Some(slot) {
                self.field = field;
            }
        }
    }

    /// Lista aberta do dropdown, desenhada por cima da página.
    fn dropdown_list(
        &self,
        ui: &mut Ui,
        area: Rect,
        slot: EquipSlot,
        ctx: &Ctx,
        equipment: Option<&Equipment>,
    ) {
        let Some(equipment) = equipment else {
            return;
        };
        // Clique fora fecha; a área cobre a aba inteira e fica embaixo da lista.
        ui.hit(scrim_id(), area);

        let count = equipment.count(slot) + 1;
        let content = widgets::DROPDOWN_ROW * count as f32 + 8.0;
        let height = content.min(widgets::DROPDOWN_MAX_HEIGHT);
        // Abaixo do campo, a não ser que não caiba; aí sobe. E, se nem assim
        // couber (campo perto da borda, ou rolado para fora), ela é presa dentro
        // da aba: fora dela a lista seria inalcançável.
        let below = self.field.bottom() + 4.0;
        let top = if below + height <= area.bottom() {
            below
        } else {
            self.field.y - 4.0 - height
        };
        let top = top.clamp(area.y, (area.bottom() - height).max(area.y));
        let rect = Rect::new(self.field.x, top, self.field.w, height);
        widgets::dropdown_panel(ui, rect);

        let view = rect.inset(4.0);
        let offset = ui.scroll_begin(dropdown_scroll_id(), view);
        let selected = self
            .build
            .as_ref()
            .and_then(|build| build.equip(slot))
            .unwrap_or_default();

        for index in 0..count {
            let row = Rect::new(
                view.x,
                view.y - offset + widgets::DROPDOWN_ROW * index as f32,
                view.w,
                widgets::DROPDOWN_ROW,
            );
            if row.bottom() < view.y || row.y > view.bottom() {
                continue;
            }
            // A primeira linha é a opção "Nenhum" da v1; as demais seguem a ordem
            // do `equipment.json`.
            let (label, is_selected) = match index.checked_sub(1) {
                None => (ctx.tr().build.equip_none, selected.is_empty()),
                Some(item) => match equipment.at(slot, item) {
                    Some(item) => (item.nome(), item.id() == selected),
                    None => continue,
                },
            };
            widgets::dropdown_row(ui, dropdown_row_id(slot, index), row, label, is_selected);
        }
        ui.scroll_end(dropdown_scroll_id(), view, content - 8.0);
    }

    // --- Builds salvas ---

    /// A build exibida em relação às salvas.
    fn save_state(&self, build: &Build, data: &GameData) -> SaveState<'_> {
        let Some(loadout) = self
            .linked
            .as_deref()
            .and_then(|id| builds::index_of(&self.loadouts, id))
            .map(|index| &self.loadouts[index])
        else {
            return SaveState::New;
        };
        let typed = self.name.trim();
        let renamed = !typed.is_empty() && typed != loadout.name;
        match builds::matches(loadout, build, data) && !renamed {
            true => SaveState::Saved(loadout),
            false => SaveState::Changed(loadout),
        }
    }

    /// Dá para rolar de novo daqui: nas sub-abas de sorteio, com o que sortear.
    fn can_reroll(&self) -> bool {
        match self.sub {
            SubTab::Random => true,
            SubTab::Meta => self.stats.lists().is_some(),
            _ => false,
        }
    }

    /// Card da build atual: aplicar e rolar de novo na barra, o estado dela em
    /// relação às salvas e a linha do salvar.
    fn current_card(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        build: &Build,
        ctx: &Ctx,
    ) {
        let tr = ctx.tr();
        let palette = theme::palette();
        let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.build.current, "sys")));

        let mut cursor = widgets::card_actions(rect);
        let width = widgets::icon_btn_width(measure, tr.build.apply_stratagems);
        widgets::icon_btn(
            ui,
            apply_id(),
            cursor.cut_right(width),
            Glyph::Text(tr.build.apply_stratagems),
            Tone::Accent,
        );
        if self.can_reroll() {
            cursor.cut_right(8.0);
            let width = widgets::icon_btn_width(measure, tr.build.reroll);
            widgets::icon_btn(
                ui,
                reroll_id(),
                cursor.cut_right(width),
                Glyph::Text(tr.build.reroll),
                Tone::Plain,
            );
        }

        let state = self.save_state(build, ctx.data);
        let replacing = matches!(self.confirm, Some(Confirm::Replace(_)));
        let mut status = content.cut_top(STATUS_HEIGHT);
        if matches!(state, SaveState::Changed(_)) {
            let width = widgets::icon_btn_width(measure, tr.build.discard);
            widgets::icon_btn(
                ui,
                discard_id(),
                status.cut_right(width),
                Glyph::Text(tr.build.discard),
                Tone::Plain,
            );
            status.cut_right(12.0);
        }
        let quoted = |name: &str| format!("\u{201c}{name}\u{201d}");
        let (color, text) = if replacing {
            (palette.error.fill, tr.build.status_replace.to_string())
        } else if self.taken {
            (palette.error.fill, tr.build.status_taken.to_string())
        } else if !build.has_stratagem() {
            (palette.warning.fill, tr.build.status_empty.to_string())
        } else {
            match state {
                SaveState::New => (palette.base_200, tr.build.status_new.to_string()),
                SaveState::Saved(loadout) => (
                    palette.success.fill,
                    format!("{} {}", tr.build.status_saved, quoted(&loadout.name)),
                ),
                SaveState::Changed(loadout) => (
                    palette.warning.fill,
                    format!(
                        "{} {} \u{00B7} {}",
                        tr.build.status_editing,
                        quoted(&loadout.name),
                        tr.build.status_unsaved
                    ),
                ),
            }
        };
        let square = status.cut_left(10.0).middle_row(10.0);
        widgets::status_square(ui, square, color);
        status.cut_left(8.0);
        let style = styles::micro().middle();
        ui.text(
            status,
            ellipsize(measure, &text.to_uppercase(), style, status.w),
            style,
            palette.content,
        );
        content.skip_top(LABEL_GAP);

        // Campo à esquerda, botões à direita: a ação principal na ponta.
        let mut row = content.cut_top(widgets::CONTROL_HEIGHT);
        let can_save = build.has_stratagem();
        match state {
            SaveState::New => {
                let (label, variant) = match (can_save, replacing) {
                    (false, _) => (tr.build.save_build, ButtonVariant::Disabled),
                    (true, true) => (tr.build.replace_build, ButtonVariant::Danger),
                    (true, false) => (tr.build.save_build, ButtonVariant::Primary),
                };
                let button = row.cut_right(button_width(measure, label));
                widgets::button(ui, save_id(), button, label, variant);
            }
            SaveState::Saved(_) | SaveState::Changed(_) => {
                let changed = matches!(state, SaveState::Changed(_)) && can_save;
                let label = tr.build.save_changes;
                let button = row.cut_right(button_width(measure, label));
                widgets::button(
                    ui,
                    save_changes_id(),
                    button,
                    label,
                    if changed {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Disabled
                    },
                );
                row.cut_right(widgets::CHIP_GAP);
                let label = tr.build.save_as_new;
                let button = row.cut_right(button_width(measure, label));
                widgets::button(
                    ui,
                    save_as_new_id(),
                    button,
                    label,
                    if can_save {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Disabled
                    },
                );
            }
        }
        row.cut_right(widgets::CHIP_GAP);

        let focused = ctx.focused_edit == Some(name_id());
        let placeholder = (self.name.is_empty() && !focused).then_some(tr.build.save_placeholder);
        widgets::edit_host(ui, name_id(), row, focused, placeholder, 0.0);
    }

    fn saved_height(&self, measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
        if self.loadouts.is_empty() {
            return widgets::card_chrome(true) + SAVED_EMPTY_HEIGHT;
        }
        let inner = width - widgets::CARD_PADDING * 2.0;
        let hint = measure
            .text_size(ctx.tr().build.saved_hint, hint_style(), inner)
            .1;
        let count = self.loadouts.len() as f32;
        widgets::card_chrome(true)
            + hint
            + LABEL_GAP
            + 4.0
            + count * SAVED_ROW_HEIGHT
            + (count - 1.0) * SAVED_ROW_GAP
    }

    /// Sub-aba Salvas: a explicação e uma linha por build, com as ações dela.
    /// Só as linhas dentro de `view` são construídas.
    fn saved_card(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        view: Rect,
        ctx: &Ctx,
    ) {
        let tr = ctx.tr();
        let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.build.saved, "db")));
        if self.loadouts.is_empty() {
            empty_state(ui, content, tr.macros.nothing_here, tr.build.saved_empty);
            return;
        }

        let height = measure
            .text_size(tr.build.saved_hint, hint_style(), content.w)
            .1;
        ui.text(
            content.cut_top(height),
            tr.build.saved_hint,
            hint_style(),
            theme::palette().muted,
        );
        content.skip_top(LABEL_GAP + 4.0);

        let active = builds::active_loadout(&self.loadouts, ctx.slots, ctx.data);
        for index in 0..self.loadouts.len() {
            let row = content.cut_top(SAVED_ROW_HEIGHT);
            content.skip_top(SAVED_ROW_GAP);
            if row.bottom() < view.y || row.y > view.bottom() {
                continue;
            }
            self.saved_row(ui, measure, row, index, active == Some(index), ctx);
        }
    }

    /// Uma build salva: os quatro estratagemas, o nome com a etiqueta de
    /// aplicada, o equipamento resumido e as três ações.
    fn saved_row(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        index: usize,
        active: bool,
        ctx: &Ctx,
    ) {
        let tr = ctx.tr();
        let palette = theme::palette();
        let loadout = &self.loadouts[index];
        ui.fill(rect, palette.base_100);
        let mut inner = rect.inset_xy(SAVED_ROW_PADDING, 0.0);

        // Ações, da direita para a esquerda. Excluir pede o segundo clique.
        let confirming = matches!(&self.confirm, Some(Confirm::Delete(id)) if *id == loadout.id);
        let delete_label = match confirming {
            true => tr.build.confirm_delete,
            false => tr.build.delete_build,
        };
        // O excluir já reserva a largura do "confirmar?": armar a pergunta não
        // empurra os outros botões para fora do lugar do mouse.
        let delete_width = widgets::icon_btn_width(measure, tr.build.delete_build)
            .max(widgets::icon_btn_width(measure, tr.build.confirm_delete));
        let actions = [
            (
                loadout_delete_id(index),
                delete_label,
                Tone::Danger,
                delete_width,
            ),
            (
                loadout_edit_id(index),
                tr.build.edit_build,
                Tone::Plain,
                widgets::icon_btn_width(measure, tr.build.edit_build),
            ),
            (
                loadout_apply_id(index),
                tr.build.apply_build,
                Tone::Accent,
                widgets::icon_btn_width(measure, tr.build.apply_build),
            ),
        ];
        for (id, label, tone, width) in actions {
            let button = inner.cut_right(width).middle_row(widgets::ICON_BTN_HEIGHT);
            widgets::icon_btn(ui, id, button, Glyph::Text(label), tone);
            if id == loadout_delete_id(index) && confirming {
                // O tempo que falta para a pergunta sumir.
                let left = ui.anim(confirm_flash_id(), CONFIRM_MS);
                ui.fill(
                    Rect::new(button.x, button.bottom() + 3.0, button.w * left, 2.0),
                    palette.error.fill,
                );
            }
            inner.cut_right(8.0);
        }
        inner.cut_right(8.0);

        let slots = loadouts::sanitize(&loadout.slot_ids, ctx.data);
        for id in slots {
            let cell = inner.cut_left(SAVED_ICON).middle_row(SAVED_ICON);
            if let Some(strat) = id.and_then(|id| ctx.data.by_id(id)) {
                ui.image(cell, format!("icons/{}", strat.imagem), 1.0);
            }
            ui.stroke(cell, theme::BORDER, palette.base_300);
            inner.cut_left(SAVED_ICON_GAP);
        }
        inner.cut_left(8.0);

        let mut text = inner.middle_row(34.0);
        let mut line = text.cut_top(18.0);
        let name_style = styles::label().middle();
        let name = loadout.name.to_uppercase();
        if active {
            // `tag-accent` logo depois do nome.
            let label = tr.build.in_slots.to_uppercase();
            let tag_style = TextStyle::new(font::SIZE_TINY, Weight::Black)
                .align(Align::Center)
                .middle();
            let tag_w = measure.text_size(&label, tag_style, f32::INFINITY).0 + 12.0;
            let room = (line.w - tag_w - 8.0).max(0.0);
            let name_w = measure
                .text_size(&name, name_style, f32::INFINITY)
                .0
                .min(room);
            let name_rect = line.cut_left(name_w);
            ui.text(
                name_rect,
                ellipsize(measure, &name, name_style, name_rect.w),
                name_style,
                palette.content,
            );
            line.cut_left(8.0);
            let tag = line.cut_left(tag_w).middle_row(16.0);
            ui.fill(tag, palette.accent);
            ui.stroke(tag, theme::BORDER, palette.base_300);
            ui.text(tag, label, tag_style, palette.accent_content);
        } else {
            ui.text(
                line,
                ellipsize(measure, &name, name_style, line.w),
                name_style,
                palette.content,
            );
        }
        let style = styles::micro().middle();
        let gear = gear_summary(loadout, data::equipment(), tr);
        ui.text(
            text,
            ellipsize(measure, &gear.to_uppercase(), style, text.w),
            style,
            palette.muted,
        );
        ui.stroke(rect, theme::BORDER, palette.base_300);
    }

    /// Toast do último salvar, aplicar ou excluir (§6.8), no canto de baixo,
    /// como o do backup.
    fn notice_toast(&mut self, ui: &mut Ui, area: Rect, ctx: &Ctx) {
        let Some(text) = &self.notice else {
            return;
        };
        if std::mem::take(&mut self.notice_pending) {
            ui.flash(notice_flash_id(), NOTICE_MS);
        }
        let left = ui.anim(notice_flash_id(), NOTICE_MS);
        if left <= 0.0 {
            self.notice = None;
            return;
        }
        let exit = motion::EXIT_MS as f32 / NOTICE_MS as f32;
        let visible = (left / exit).min(1.0);
        let rect = Rect::new(
            area.right() - TOAST_MARGIN - theme::SHADOW - widgets::TOAST_WIDTH,
            area.bottom() - TOAST_MARGIN - theme::SHADOW - widgets::TOAST_HEIGHT,
            widgets::TOAST_WIDTH,
            widgets::TOAST_HEIGHT,
        );
        let text = text.clone();
        widgets::toast(
            ui,
            rect,
            theme::palette().success,
            ctx.tr().settings.toast_done,
            &text,
            visible,
        );
    }

    /// Mostra um toast novo, que a próxima construção acende.
    fn notify(&mut self, text: String) {
        self.notice = Some(text);
        self.notice_pending = true;
    }

    // --- Build exibida ---

    fn stratagems_card(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        build: &Build,
        ctx: &Ctx,
    ) {
        let tr = ctx.tr();
        let content = widgets::card(ui, rect, Some(CardHeader::new(tr.build.stratagems, "sys")));

        for (index, cell) in columns(content, SLOT_COUNT, GRID_GAP)
            .into_iter()
            .enumerate()
        {
            let strat = build.stratagems[index].and_then(|id| ctx.data.by_id(id));
            let label = format!("{} {}", tr.build.stratagem, index + 1);
            let card = ItemCard {
                label: &label,
                name: strat.map(|strat| strat.nome.as_str()).unwrap_or("-"),
                image: strat.map(|strat| strat.imagem.as_str()),
                subtitle: None,
                description: None,
                badge: None,
                locked: self.locks.stratagem(index),
            };
            let height = widgets::item_card_height(measure, &card, cell.w);
            let phase = self.card_motion(ui, index);
            widgets::build_item_card(
                ui,
                measure,
                strat_lock_id(index),
                cell.with_h(height),
                &card,
                phase,
            );
        }
    }

    /// Os cards de equipamento, com os textos que cada categoria mostra.
    fn equipment_cards(&self, build: &Build, equipment: &Equipment, ctx: &Ctx) -> Vec<EquipCard> {
        let tr = ctx.tr();
        EquipSlot::ALL
            .into_iter()
            .filter_map(|slot| {
                let item = build.item(equipment, slot)?;
                let (subtitle, description) = match item {
                    Item::Weapon(weapon) => (
                        Some(
                            [weapon.tipo.as_str(), weapon.dano.as_str()]
                                .iter()
                                .filter(|part| !part.is_empty())
                                .copied()
                                .collect::<Vec<&str>>()
                                .join(" · "),
                        ),
                        None,
                    ),
                    Item::Armor(armor) => (
                        Some(format!(
                            "{} · ARM {} · VEL {} · STA {}",
                            tr.build.weight(&armor.peso),
                            armor.armor,
                            armor.speed,
                            armor.stamina
                        )),
                        Some(match equipment.passive(&armor.passive) {
                            Some(passive) => {
                                format!("{}: {}", armor.passive, passive.descricao)
                            }
                            None => armor.passive.clone(),
                        }),
                    ),
                    Item::Described(booster) if slot == EquipSlot::Booster => {
                        (None, Some(booster.descricao.clone()))
                    }
                    _ => (None, None),
                };
                Some(EquipCard {
                    slot,
                    label: tr.build.equip_label(slot).to_string(),
                    name: item.nome().to_string(),
                    image: Some(item.imagem().to_string()),
                    subtitle,
                    description,
                    badge: build
                        .is_set_piece(equipment, slot)
                        .then_some(tr.build.set_badge),
                    locked: self.locks.equip(slot),
                })
            })
            .collect()
    }

    fn equipment_card(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        cards: &[EquipCard],
        ctx: &Ctx,
    ) {
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader::new(ctx.tr().build.equipment, "sys")),
        );
        let width = cell_width(content.w, EQUIP_COLS);
        let mut y = content.y;

        for row in cards.chunks(EQUIP_COLS) {
            let height = row_height(measure, row, width);
            for (column, card) in row.iter().enumerate() {
                let rect = Rect::new(
                    content.x + (width + GRID_GAP) * column as f32,
                    y,
                    width,
                    height,
                );
                let phase = self.card_motion(ui, SLOT_COUNT + card.slot.index());
                widgets::build_item_card(
                    ui,
                    measure,
                    equip_lock_id(card.slot),
                    rect,
                    &card.as_item_card(),
                    phase,
                );
            }
            y += height + GRID_GAP;
        }
    }

    // --- Cliques ---

    /// Trata um clique da aba. `None` quando o id não é daqui, ou quando a
    /// regra de equipar recusou a jogada, que na v1 também não fazia nada.
    pub fn on_click(&mut self, clicked: Id, ctx: &Ctx) -> Option<Action> {
        // Com a lista aberta ela tem prioridade: o resto da tela está atrás dela.
        if let Some(slot) = self.open {
            if let Some(action) = self.on_dropdown_click(clicked, slot) {
                return Some(action);
            }
        }
        // Qualquer clique desarma a confirmação em curso; só o botão que a
        // armou a consome, no segundo clique.
        let confirm = self.confirm.take();

        for (index, sub) in SubTab::ALL.into_iter().enumerate() {
            if clicked == sub_tab_id(index) {
                self.sub = sub;
                return Some(Action::Redraw);
            }
        }
        for (index, option) in options(ctx).into_iter().enumerate() {
            if clicked == option_id(index) {
                return Some(Action::Setting((option.change)(!option.on)));
            }
        }
        if clicked == generate_id() || (clicked == reroll_id() && self.sub == SubTab::Random) {
            return Some(self.generate(ctx));
        }
        if clicked == meta_generate_id() || clicked == reroll_id() {
            return Some(self.generate_meta(ctx));
        }
        if clicked == meta_retry_id() {
            // A próxima construção registra a consulta de novo.
            self.stats.reset();
            return Some(Action::Redraw);
        }
        for (index, faction) in Faction::ALL.into_iter().enumerate() {
            if clicked == faction_id(index) {
                if self.stats.faction != faction {
                    self.stats.faction = faction;
                    self.stats.reset();
                }
                return Some(Action::Redraw);
            }
        }
        for (index, difficulty) in DIFFICULTIES.into_iter().enumerate() {
            if clicked == difficulty_id(index) {
                if self.stats.difficulty != difficulty {
                    self.stats.difficulty = difficulty;
                    self.stats.reset();
                }
                return Some(Action::Redraw);
            }
        }
        if clicked == apply_id() {
            let build = self.build.as_ref()?;
            let slots = crate::loadouts::sanitize(&build.stratagems, ctx.data);
            let text = match self.save_state(build, ctx.data) {
                SaveState::Saved(loadout) => notice(&loadout.name, ctx.tr().build.notice_applied),
                _ => ctx.tr().build.notice_applied_current.to_string(),
            };
            self.notify(text);
            return Some(Action::SlotsChanged(slots));
        }
        if clicked == import_slots_id() {
            let mut build = self.build.clone().unwrap_or_default();
            build.stratagems = ctx.slots;
            self.build = Some(build);
            return Some(Action::Redraw);
        }
        if clicked == reset_id() {
            self.build = None;
            self.custom_slot = 0;
            self.locks = Locks::default();
            self.linked = None;
            self.taken = false;
            self.set_search(String::new());
            self.name.clear();
            return Some(Action::SetEdits(vec![
                (search_id(), String::new()),
                (name_id(), String::new()),
            ]));
        }
        if clicked == search_id() {
            return Some(Action::FocusEdit(search_id()));
        }
        if clicked == clear_search_id() {
            return Some(Action::SetEdits(vec![(search_id(), String::new())]));
        }
        if clicked == name_id() {
            return Some(Action::FocusEdit(name_id()));
        }
        if clicked == save_id() {
            return self.save_new(ctx, confirm);
        }
        if clicked == save_changes_id() {
            return self.save_changes(ctx);
        }
        if clicked == save_as_new_id() {
            return self.save_copy(ctx);
        }
        if clicked == discard_id() {
            let index = builds::index_of(&self.loadouts, self.linked.as_deref()?)?;
            let loadout = self.loadouts[index].clone();
            self.load(&loadout, ctx);
            return Some(Action::SetEdits(vec![(name_id(), loadout.name)]));
        }

        for index in 0..self.loadouts.len() {
            if clicked == loadout_apply_id(index) {
                let loadout = self.loadouts[index].clone();
                let slots = self.load(&loadout, ctx);
                self.notify(notice(&loadout.name, ctx.tr().build.notice_applied));
                return Some(Action::Applied {
                    slots,
                    name: loadout.name,
                });
            }
            if clicked == loadout_edit_id(index) {
                // O editor é a sub-aba Personalizada, a partir do primeiro slot.
                let loadout = self.loadouts[index].clone();
                self.load(&loadout, ctx);
                self.sub = SubTab::Custom;
                self.custom_slot = 0;
                return Some(Action::SetEdits(vec![(name_id(), loadout.name)]));
            }
            if clicked == loadout_delete_id(index) {
                let id = self.loadouts[index].id.clone();
                if confirm != Some(Confirm::Delete(id.clone())) {
                    self.confirm = Some(Confirm::Delete(id));
                    self.confirm_pending = true;
                    return Some(Action::Redraw);
                }
                let removed = self.loadouts.remove(index);
                // A build exibida continua na tela, agora sem build salva por
                // trás: salvar de novo a recria.
                if self.linked.as_deref() == Some(removed.id.as_str()) {
                    self.linked = None;
                }
                self.notify(notice(&removed.name, ctx.tr().build.notice_deleted));
                return Some(Action::LoadoutsChanged);
            }
        }

        for index in 0..SLOT_COUNT {
            if clicked == custom_slot_id(index) {
                self.custom_slot = index;
                return Some(Action::Redraw);
            }
            if clicked == custom_clear_id(index) {
                let mut build = self.build.clone()?;
                build.stratagems[index] = None;
                self.build = Some(build);
                return Some(Action::Redraw);
            }
            if clicked == strat_lock_id(index) {
                self.locks.toggle_stratagem(index);
                return Some(Action::Redraw);
            }
        }
        for slot in EquipSlot::ALL {
            if clicked == equip_lock_id(slot) {
                self.locks.toggle_equip(slot);
                return Some(Action::Redraw);
            }
            if clicked == dropdown_id(slot) {
                self.open = Some(slot);
                return Some(Action::Redraw);
            }
        }

        let strat = ctx
            .data
            .all()
            .iter()
            .find(|strat| card_id(strat.id) == clicked)?;
        self.assign(strat, ctx)
    }

    /// Clique com a lista aberta: escolher um item, ou fechá-la.
    fn on_dropdown_click(&mut self, clicked: Id, slot: EquipSlot) -> Option<Action> {
        if clicked == scrim_id() || clicked == dropdown_id(slot) {
            self.open = None;
            return Some(Action::Redraw);
        }
        let equipment = data::equipment()?;
        for index in 0..=equipment.count(slot) {
            if clicked != dropdown_row_id(slot, index) {
                continue;
            }
            let id = index
                .checked_sub(1)
                .and_then(|item| equipment.at(slot, item))
                .map(|item| item.id().to_string());
            let mut build = self.build.clone().unwrap_or_default();
            build.set_equip(slot, id);
            self.build = Some(build);
            self.open = None;
            return Some(Action::Redraw);
        }
        None
    }

    /// Põe uma build salva na tela, vinculada a ela, e devolve os slots que ela
    /// define (já saneados).
    fn load(&mut self, loadout: &Loadout, ctx: &Ctx) -> Slots {
        let applied = builds::apply(loadout, self.build.as_ref(), ctx.data, data::equipment());
        self.build = Some(applied.build);
        self.linked = Some(loadout.id.clone());
        self.taken = false;
        self.roll = None;
        applied.slots
    }

    /// "Salvar" de uma build nova. Nome de outra build pede confirmação: o
    /// segundo clique, dentro dos 3s, substitui. Trocar o nome no meio desarma
    /// a pergunta ([`BuildTab::set_name`]), então ela sempre é sobre o nome que
    /// está no campo.
    fn save_new(&mut self, ctx: &Ctx, confirm: Option<Confirm>) -> Option<Action> {
        let build = self.build.clone()?;
        let replace = matches!(confirm, Some(Confirm::Replace(_)));
        match builds::save(&mut self.loadouts, &self.name, &build, replace) {
            Ok(index) => Some(self.saved(index, ctx.tr().build.notice_saved)),
            Err(SaveError::NameTaken(index)) => {
                self.confirm = Some(Confirm::Replace(self.loadouts[index].id.clone()));
                self.confirm_pending = true;
                Some(Action::Redraw)
            }
            Err(_) => Some(Action::Redraw),
        }
    }

    /// "Salvar alterações": grava por cima da build vinculada, com o nome do
    /// campo (renomear é só trocar o nome e salvar).
    fn save_changes(&mut self, ctx: &Ctx) -> Option<Action> {
        let build = self.build.clone()?;
        let id = self.linked.clone()?;
        match builds::update(&mut self.loadouts, &id, &self.name, &build) {
            Ok(index) => Some(self.saved(index, ctx.tr().build.notice_updated)),
            Err(SaveError::NameTaken(_)) => {
                self.taken = true;
                Some(Action::Redraw)
            }
            Err(SaveError::Missing) => {
                self.linked = None;
                Some(Action::Redraw)
            }
            Err(SaveError::Empty) => Some(Action::Redraw),
        }
    }

    /// "Salvar como nova": uma cópia, com o nome do campo se ele estiver livre
    /// ou com um número no fim.
    fn save_copy(&mut self, ctx: &Ctx) -> Option<Action> {
        let build = self.build.clone()?;
        let name = builds::copy_name(&self.loadouts, &self.name, NAME_MAX_CHARS);
        match builds::save(&mut self.loadouts, &name, &build, false) {
            Ok(index) => Some(self.saved(index, ctx.tr().build.notice_saved)),
            Err(_) => Some(Action::Redraw),
        }
    }

    /// Depois de gravar: a build exibida passa a ser a salva, o campo mostra o
    /// nome dela e o toast confirma.
    fn saved(&mut self, index: usize, suffix: &str) -> Action {
        let loadout = &self.loadouts[index];
        let name = loadout.name.clone();
        self.linked = Some(loadout.id.clone());
        self.taken = false;
        self.name = name.chars().take(NAME_MAX_CHARS).collect();
        self.notify(notice(&name, suffix));
        Action::Saved(name)
    }

    /// Build nova na tela: ela deixa de ser a salva de onde veio. O nome que o
    /// vínculo pôs no campo sai junto; um nome digitado à mão fica.
    fn unlink(&mut self) -> Action {
        self.taken = false;
        let linked = self
            .linked
            .take()
            .and_then(|id| builds::index_of(&self.loadouts, &id))
            .map(|index| self.loadouts[index].name.clone());
        match linked {
            Some(name) if self.name.trim() == name => {
                self.name.clear();
                Action::SetEdits(vec![(name_id(), String::new())])
            }
            _ => Action::Redraw,
        }
    }

    /// Sorteia uma build nova, preservando o que está travado.
    fn generate(&mut self, ctx: &Ctx) -> Action {
        let Some(equipment) = data::equipment() else {
            log::warn!("equipment.json indisponível: nada a sortear");
            return Action::Redraw;
        };
        let meta = self
            .meta
            .get_or_insert_with(|| StratMeta::build(ctx.data, equipment));
        let mut rng = rand::rng();
        let prev = self.build.take();
        let next = builds::generate(
            prev.as_ref(),
            &self.locks,
            Rules::from_settings(ctx.settings),
            ctx.data,
            equipment,
            meta,
            &mut rng,
        );
        let pool: Vec<u32> = ctx.data.all().iter().map(|strat| strat.id).collect();
        let reels = roll_reels(
            RollInput {
                prev: prev.as_ref(),
                next: &next,
                locks: &self.locks,
                data: ctx.data,
                equipment,
                stratagems: &pool,
                weapons: None,
            },
            &mut rng,
        );
        self.start_roll(next, reels)
    }

    /// Sorteia uma build a partir das estatísticas em tela.
    fn generate_meta(&mut self, ctx: &Ctx) -> Action {
        let Some(lists) = self.stats.lists() else {
            return Action::Redraw;
        };
        let Some(equipment) = data::equipment() else {
            log::warn!("equipment.json indisponível: nada a sortear");
            return Action::Redraw;
        };
        let kinds = self
            .meta
            .get_or_insert_with(|| StratMeta::build(ctx.data, equipment));
        let mut rng = rand::rng();
        let prev = self.build.take();
        let next = builds::generate_meta(
            prev.as_ref(),
            &self.locks,
            Rules::from_settings(ctx.settings),
            lists,
            ctx.data,
            equipment,
            kinds,
            &mut rng,
        );
        let pool: Vec<u32> = lists
            .stratagems
            .iter()
            .take(builds::META_TOP_STRATS)
            .map(|pick| pick.item)
            .collect();
        let reels = roll_reels(
            RollInput {
                prev: prev.as_ref(),
                next: &next,
                locks: &self.locks,
                data: ctx.data,
                equipment,
                stratagems: &pool,
                weapons: Some(lists),
            },
            &mut rng,
        );
        self.start_roll(next, reels)
    }

    /// A build sorteada entra na tela girando, e a página vai até ela.
    fn start_roll(&mut self, next: Build, reels: Vec<Vec<ReelFrame>>) -> Action {
        self.build = Some(next);
        self.roll = Some(Roll {
            pending: true,
            reels,
        });
        self.reveal = true;
        self.unlink()
    }

    /// Clique na grade personalizada. O slot em edição avança mesmo quando a
    /// regra recusa. É o que a v1 fazia, com o avanço fora do `setState`.
    fn assign(&mut self, strat: &Stratagem, ctx: &Ctx) -> Option<Action> {
        let mut build = self.build.clone().unwrap_or_default();
        builds::custom_assign(&mut build, self.custom_slot, strat, ctx.data);
        self.build = Some(build);
        if self.custom_slot + 1 < SLOT_COUNT {
            self.custom_slot += 1;
        }
        Some(Action::Redraw)
    }
}

/// Texto do toast: o nome entre aspas e o que aconteceu com ele.
fn notice(name: &str, suffix: &str) -> String {
    format!("\u{201c}{name}\u{201d} {suffix}")
}

/// Leva a página até a build recém-sorteada, se o topo dela estiver fora de
/// vista: o botão de sortear fica no alto e a build sai lá embaixo, depois das
/// listas. `top` é a posição dela na tela nesta passagem.
fn reveal(ui: &mut Ui, view: Rect, top: f32, offset: f32) {
    if top >= view.y && top <= view.bottom() - REVEAL_VISIBLE {
        return;
    }
    let target = offset + (top - view.y) - REVEAL_MARGIN;
    ui.scroll_to(scroll_id(), target, REVEAL_MS);
}

/// Largura de um botão do salvar: o rótulo com folga, nunca menor que o
/// "Salvar".
fn button_width(measure: &mut dyn Measure, label: &str) -> f32 {
    let text = measure
        .text_size(&label.to_uppercase(), styles::button(), f32::INFINITY)
        .0;
    (text + BUTTON_PADDING * 2.0).max(SAVE_WIDTH)
}

/// Equipamento da build salva numa linha: os nomes, na ordem da tela.
fn gear_summary(loadout: &Loadout, equipment: Option<&Equipment>, tr: &Tr) -> String {
    let names: Vec<&str> = match (equipment, &loadout.equip) {
        (Some(equipment), Some(equip)) => EquipSlot::ALL
            .into_iter()
            .filter_map(|slot| equipment.find(slot, equip.get(slot.key())?))
            .map(Item::nome)
            .collect(),
        _ => Vec::new(),
    };
    match names.is_empty() {
        true => tr.build.saved_no_gear.to_string(),
        false => names.join(" \u{00B7} "),
    }
}

/// O que monta os rolos de um sorteio.
struct RollInput<'a> {
    prev: Option<&'a Build>,
    next: &'a Build,
    locks: &'a Locks,
    data: &'a GameData,
    equipment: &'a Equipment,
    /// De onde os estratagemas saíram: o topo, na build meta.
    stratagems: &'a [u32],
    /// Na build meta, as armas saem do topo de cada categoria.
    weapons: Option<&'a MetaLists>,
}

/// Rolo de cada card, na ordem da tela: o item que estava lá, alguns da mesma
/// pool só para passar, e o sorteado no fim. Card travado, ou que ficou sem
/// item, não gira.
fn roll_reels<R: Rng + ?Sized>(input: RollInput, rng: &mut R) -> Vec<Vec<ReelFrame>> {
    let RollInput {
        prev,
        next,
        locks,
        data,
        equipment,
        stratagems,
        weapons,
    } = input;
    let mut reels = Vec::with_capacity(SLOT_COUNT + data::EQUIP_SLOT_COUNT);

    let strat_frame = |id: u32| {
        data.by_id(id).map(|strat| ReelFrame {
            image: Some(strat.imagem.clone()),
            name: strat.nome.clone(),
        })
    };
    for index in 0..SLOT_COUNT {
        let reel = match (locks.stratagem(index), next.stratagems[index]) {
            (false, Some(after)) => {
                let before = prev.and_then(|build| build.stratagems[index]);
                let skip: Vec<u32> = before.into_iter().chain([after]).collect();
                reel(
                    before.and_then(strat_frame),
                    samples(stratagems, &skip, rng)
                        .into_iter()
                        .filter_map(strat_frame)
                        .collect(),
                    strat_frame(after),
                )
            }
            _ => Vec::new(),
        };
        reels.push(reel);
    }

    let item_frame = |slot: EquipSlot, id: &str| {
        equipment.find(slot, id).map(|item| ReelFrame {
            image: Some(item.imagem().to_string()),
            name: item.nome().to_string(),
        })
    };
    for slot in EquipSlot::ALL {
        let reel = match (locks.equip(slot), next.equip(slot)) {
            (false, Some(after)) => {
                let before = prev.and_then(|build| build.equip(slot));
                let pool: Vec<&str> = match weapons.filter(|_| slot.is_weapon()) {
                    Some(lists) => lists
                        .weapons(slot)
                        .iter()
                        .take(builds::META_TOP_WEAPONS)
                        .map(|pick| pick.item.as_str())
                        .collect(),
                    None => (0..equipment.count(slot))
                        .filter_map(|index| equipment.at(slot, index))
                        .map(Item::id)
                        .collect(),
                };
                let skip: Vec<&str> = before.into_iter().chain([after]).collect();
                reel(
                    before.and_then(|id| item_frame(slot, id)),
                    samples(&pool, &skip, rng)
                        .into_iter()
                        .filter_map(|id| item_frame(slot, id))
                        .collect(),
                    item_frame(slot, after),
                )
            }
            _ => Vec::new(),
        };
        reels.push(reel);
    }
    reels
}

/// Até [`REEL_SAMPLES`] itens da pool, fora os de `skip`.
fn samples<T: Copy + PartialEq, R: Rng + ?Sized>(pool: &[T], skip: &[T], rng: &mut R) -> Vec<T> {
    let candidates: Vec<T> = pool
        .iter()
        .copied()
        .filter(|item| !skip.contains(item))
        .collect();
    candidates.sample(rng, REEL_SAMPLES).copied().collect()
}

/// Quadros de um rolo: o que sai, as amostras repetidas até completar, e o que
/// entra. Sem o item novo não há rolo.
fn reel(
    before: Option<ReelFrame>,
    samples: Vec<ReelFrame>,
    after: Option<ReelFrame>,
) -> Vec<ReelFrame> {
    let Some(after) = after else {
        return Vec::new();
    };
    let mut frames: Vec<ReelFrame> = before.into_iter().collect();
    for sample in samples
        .iter()
        .cycle()
        .take(REEL_FRAMES.saturating_sub(frames.len() + 1))
    {
        frames.push(sample.clone());
    }
    frames.push(after);
    frames
}

// --- Opções de sorteio ---

/// Uma das três linhas do card de opções.
struct Option_<'a> {
    title: &'a str,
    description: &'a str,
    on: bool,
    change: fn(bool) -> Change,
}

fn options<'a>(ctx: &Ctx<'a>) -> [Option_<'static>; 3] {
    let tr = ctx.tr();
    let settings = ctx.settings;
    [
        Option_ {
            title: tr.build.match_set,
            description: if settings.build_match_set {
                tr.build.match_set_on
            } else {
                tr.build.match_set_off
            },
            on: settings.build_match_set,
            change: Change::BuildMatchSet,
        },
        Option_ {
            title: tr.build.balanced,
            description: if settings.build_balanced {
                tr.build.balanced_on
            } else {
                tr.build.balanced_off
            },
            on: settings.build_balanced,
            change: Change::BuildBalanced,
        },
        Option_ {
            title: tr.build.max_sentry,
            description: if settings.build_max_one_sentry {
                tr.build.max_sentry_on
            } else {
                tr.build.max_sentry_off
            },
            on: settings.build_max_one_sentry,
            change: Change::BuildMaxOneSentry,
        },
    ]
}

/// Card de equipamento já com os textos prontos: os `&str` do widget precisam
/// de alguém que os mantenha vivos durante a construção.
struct EquipCard {
    slot: EquipSlot,
    label: String,
    name: String,
    image: Option<String>,
    subtitle: Option<String>,
    description: Option<String>,
    badge: Option<&'static str>,
    locked: bool,
}

impl EquipCard {
    fn as_item_card(&self) -> ItemCard<'_> {
        ItemCard {
            label: &self.label,
            name: &self.name,
            image: self.image.as_deref(),
            subtitle: self.subtitle.as_deref(),
            description: self.description.as_deref(),
            badge: self.badge,
            locked: self.locked,
        }
    }
}

/// Estado vazio (§8): kicker `> NADA AQUI` e a explicação apagada, sem
/// ilustração.
fn empty_state(ui: &mut Ui, rect: Rect, kicker: &str, message: &str) {
    let palette = theme::palette();
    let mut rect = rect.middle_row(34.0);
    ui.text(
        rect.cut_top(16.0),
        widgets::sigil(kicker),
        styles::micro().align(Align::Center).middle(),
        palette.accent_text,
    );
    ui.text(
        rect,
        message,
        styles::hint().align(Align::Center).middle(),
        palette.muted,
    );
}

/// Recorta a próxima linha de uma lista, com o respiro entre linhas, e sem
/// sobra depois da última, que é o que as alturas calculadas assumem.
fn meta_row(cursor: &mut Rect, index: usize) -> Rect {
    if index > 0 {
        cursor.skip_top(META_ROW_GAP);
    }
    cursor.cut_top(META_ROW_HEIGHT)
}

/// Cabeçalho de uma lista (`section-label`), com a linha de 2px embaixo como
/// o `thead` de uma tabela, já avançando o cursor.
fn meta_section_label(ui: &mut Ui, cursor: &mut Rect, label: &str) {
    let palette = theme::palette();
    let row = cursor.cut_top(LABEL_HEIGHT);
    widgets::section_label(ui, row, label);
    ui.fill(
        Rect::new(row.x, row.bottom() + 2.0, row.w, theme::BORDER),
        palette.base_300,
    );
    cursor.skip_top(LABEL_GAP);
}

/// Linha do top de estratagemas: ícone, nome, "NOVO", variação, o trilho de
/// uso e o percentual.
fn meta_stratagem_row(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    rect: Rect,
    strat: &Stratagem,
    stat: ItemStat,
    best: f64,
    ctx: &Ctx,
) {
    let palette = theme::palette();
    meta_rule(ui, rect);
    let mut row = rect;
    let icon = row.cut_left(META_ICON).middle_row(META_ICON);
    row.cut_left(META_CELL_GAP);
    ui.image(icon, format!("icons/{}", strat.imagem), 1.0);

    // As colunas de número são fixas; o nome fica com o que sobrar.
    let percent = row.cut_right(META_PERCENT_WIDTH);
    row.cut_right(META_CELL_GAP);
    let bar = row.cut_right(META_BAR_WIDTH).middle_row(META_BAR_HEIGHT);
    row.cut_right(META_CELL_GAP);
    let change = row.cut_right(META_CHANGE_WIDTH);
    row.cut_right(META_CELL_GAP);
    let badge = stat.is_new().then(|| {
        let badge = row.cut_right(META_NEW_WIDTH).middle_row(16.0);
        row.cut_right(META_CELL_GAP);
        badge
    });

    meta_name(ui, measure, row, &strat.nome);

    if let Some(badge) = badge {
        // `tag-accent`.
        ui.fill(badge, palette.accent);
        ui.stroke(badge, theme::BORDER, palette.base_300);
        ui.text(
            badge,
            ctx.tr().build.meta_new,
            TextStyle::new(font::SIZE_TINY, Weight::Black)
                .align(Align::Center)
                .middle(),
            palette.accent_content,
        );
    }

    let delta = stat.change();
    let (arrow, color) = match delta {
        delta if delta > 0.0 => ("\u{25B2}", palette.success.text),
        delta if delta < 0.0 => ("\u{25BC}", palette.error.text),
        _ => ("", palette.muted),
    };
    ui.text(
        change,
        format!("{arrow}{:.1}", delta.abs()),
        TextStyle::new(font::SIZE_MICRO, Weight::Bold)
            .align(Align::End)
            .middle(),
        color,
    );

    // Trilho proporcional ao primeiro colocado, e não a 100%.
    let share = match best > 0.0 {
        true => (stat.loadouts_percentage / best).clamp(0.0, 1.0) as f32,
        false => 0.0,
    };
    widgets::usage_bar(ui, bar, share);
    meta_percent(ui, percent, stat);
}

/// Linha de arma ou passiva: ícone (quando existe), nome e percentual.
fn meta_item_row(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    rect: Rect,
    name: &str,
    image: Option<&str>,
    stat: ItemStat,
) {
    meta_rule(ui, rect);
    let mut row = rect;
    let icon = row.cut_left(META_ICON).middle_row(META_ICON);
    row.cut_left(META_CELL_GAP);
    if let Some(path) = image {
        ui.image_styled(icon, format!("icons/{path}"), ImageStyle::FILL.contain());
    }

    let percent = row.cut_right(META_PERCENT_WIDTH);
    row.cut_right(META_CELL_GAP);
    meta_name(ui, measure, row, name);
    meta_percent(ui, percent, stat);
}

/// Divisor de linha de lista: 1 DIP da moldura a 30%, embaixo da linha.
fn meta_rule(ui: &mut Ui, rect: Rect) {
    ui.fill(
        Rect::new(
            rect.x,
            rect.bottom() - theme::HAIRLINE,
            rect.w,
            theme::HAIRLINE,
        ),
        theme::palette().rule(),
    );
}

fn meta_name(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, name: &str) {
    let style = TextStyle::new(font::SIZE_MICRO, Weight::Bold).middle();
    ui.text(
        rect,
        ellipsize(measure, &name.to_uppercase(), style, rect.w),
        style,
        theme::palette().content,
    );
}

/// Percentual em números tabulares: a fonte já é monoespaçada.
fn meta_percent(ui: &mut Ui, rect: Rect, stat: ItemStat) {
    ui.text(
        rect,
        format!("{:.1}%", stat.loadouts_percentage),
        TextStyle::new(font::SIZE_LABEL, Weight::Black)
            .align(Align::End)
            .middle(),
        theme::palette().accent_text,
    );
}

/// Corta o texto até caber, terminando em reticências (o `truncate` do legado).
/// Sem isso um nome comprido invadiria as colunas de número da linha.
fn ellipsize(measure: &mut dyn Measure, text: &str, style: TextStyle, width: f32) -> String {
    let full = measure.text_size(text, style, f32::INFINITY).0;
    if full <= width || width <= 0.0 {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    // Primeiro palpite pela proporção: quase sempre acerta de primeira, e o
    // laço só desce um caractere ou outro.
    let mut keep = ((chars.len() as f32 * width / full) as usize).min(chars.len());
    while keep > 0 {
        let candidate: String = chars[..keep]
            .iter()
            .collect::<String>()
            .trim_end()
            .chars()
            .chain(std::iter::once('\u{2026}'))
            .collect();
        if measure.text_size(&candidate, style, f32::INFINITY).0 <= width {
            return candidate;
        }
        keep -= 1;
    }
    "\u{2026}".to_string()
}

/// Milhar como o `toLocaleString()` do idioma: ponto em português, vírgula em
/// inglês.
fn grouped(value: u64, language: Language) -> String {
    let separator = match language {
        Language::Pt => '.',
        Language::En => ',',
    };
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

/// Segmento do grupo de sub-abas: o escolhido inunda de accent, os outros
/// ganham o fill de 8% no hover (como os itens de navegação).
fn sub_tab(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let palette = theme::palette();
    let hover = ui.fade(id, ui.is_hot(id), motion::HOVER_MS);
    let style = styles::label().align(Align::Center).middle();

    if selected {
        ui.fill(rect, palette.accent);
        ui.stroke(rect, theme::BORDER, palette.base_300);
        ui.text(rect, label.to_uppercase(), style, palette.accent_content);
    } else {
        if hover > 0.0 {
            ui.fill(rect, palette.hover_fill().faded(hover));
        }
        ui.text(rect, label.to_uppercase(), style, palette.content);
    }
    ui.hit(id, rect);
}

// --- Alturas (a rolagem precisa delas antes de desenhar) ---

fn options_height() -> f32 {
    widgets::card_chrome(false) + TOGGLE_HEIGHT * 3.0 + TOGGLE_GAP * 2.0
}

fn meta_rows_height(count: usize) -> f32 {
    match count {
        0 => 0.0,
        count => count as f32 * META_ROW_HEIGHT + (count - 1) as f32 * META_ROW_GAP,
    }
}

/// Cabeçalho mais as linhas de uma lista.
fn meta_section_height(count: usize) -> f32 {
    LABEL_HEIGHT + LABEL_GAP + meta_rows_height(count)
}

/// As duas colunas acompanham a mais alta, como o grid do legado.
fn meta_columns_height(lists: &MetaLists) -> f32 {
    let left = meta_section_height(lists.stratagems.len().min(builds::META_TOP_STRATS));
    let weapons: f32 = [EquipSlot::Primary, EquipSlot::Secondary, EquipSlot::Grenade]
        .into_iter()
        .map(|slot| {
            meta_section_height(lists.weapons(slot).len().min(builds::META_TOP_WEAPONS))
                + META_BLOCK_GAP
        })
        .sum();
    let passives = meta_section_height(lists.passives.len().min(builds::META_TOP_PASSIVES));
    left.max(weapons + passives)
}

impl BuildTab {
    fn meta_height(&self) -> f32 {
        let choices =
            META_FACTION_HEIGHT + META_CHOICE_GAP + META_DIFFICULTY_HEIGHT + META_BLOCK_GAP;
        let body = match self.stats.lists() {
            Some(lists) => {
                META_GENERATE_HEIGHT
                    + META_BLOCK_GAP
                    + 4.0
                    + meta_columns_height(lists)
                    + META_BLOCK_GAP
                    + LABEL_HEIGHT
            }
            None => META_STATUS_HEIGHT,
        };
        widgets::card_chrome(true) + choices + body
    }
}

fn random_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let hint = measure
        .text_size(ctx.tr().build.hint, hint_style(), inner)
        .1;
    widgets::card_chrome(false) + GENERATE_HEIGHT + LABEL_GAP + 6.0 + hint
}

fn current_height() -> f32 {
    widgets::card_chrome(true) + STATUS_HEIGHT + LABEL_GAP + widgets::CONTROL_HEIGHT
}

fn equip_rows() -> f32 {
    data::EQUIP_SLOT_COUNT.div_ceil(EQUIP_COLS) as f32
}

fn cell_width(width: f32, cols: usize) -> f32 {
    ((width - GRID_GAP * (cols - 1) as f32) / cols as f32).max(0.0)
}

/// Colunas que cabem numa grade personalizada de `width` de largura.
fn custom_cols(width: f32) -> usize {
    let fits = ((width + GRID_GAP) / (CUSTOM_TILE_TARGET + GRID_GAP)).floor();
    (fits.max(0.0) as usize).max(CUSTOM_GRID_MIN_COLS)
}

fn custom_cell(width: f32) -> f32 {
    cell_width(width, custom_cols(width))
}

fn stratagems_height(measure: &mut dyn Measure, width: f32, build: &Build, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let cell = cell_width(inner, SLOT_COUNT);
    let tallest = (0..SLOT_COUNT)
        .map(|index| {
            let name = build.stratagems[index]
                .and_then(|id| ctx.data.by_id(id))
                .map(|strat| strat.nome.as_str())
                .unwrap_or("-");
            let card = ItemCard {
                label: ctx.tr().build.stratagem,
                name,
                image: None,
                subtitle: None,
                description: None,
                badge: None,
                locked: false,
            };
            widgets::item_card_height(measure, &card, cell)
        })
        .fold(0.0f32, f32::max);
    widgets::card_chrome(true) + tallest
}

/// Altura de uma linha de cards: todos acompanham o mais alto, como o grid do
/// legado, que esticava os irmãos.
fn row_height(measure: &mut dyn Measure, row: &[EquipCard], width: f32) -> f32 {
    row.iter()
        .map(|card| widgets::item_card_height(measure, &card.as_item_card(), width))
        .fold(0.0f32, f32::max)
}

fn equipment_height(measure: &mut dyn Measure, width: f32, cards: &[EquipCard]) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let cell = cell_width(inner, EQUIP_COLS);
    let rows: f32 = cards
        .chunks(EQUIP_COLS)
        .map(|row| row_height(measure, row, cell) + GRID_GAP)
        .sum();
    widgets::card_chrome(true) + (rows - GRID_GAP).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::{Input, Visual};

    /// Medidor de largura fixa com quebra grosseira: o layout só precisa de uma
    /// medida plausível.
    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, max: f32) -> (f32, f32) {
            let width = text.chars().count() as f32 * style.size * 0.6;
            if style.wrap && max.is_finite() && width > max {
                let lines = (width / max).ceil();
                (max, style.size * 1.3 * lines)
            } else {
                (width, style.size)
            }
        }
    }

    const AREA: Rect = Rect::new(0.0, 0.0, 820.0, 532.0);

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    fn ctx<'a>(data: &'a GameData, settings: &'a Settings) -> Ctx<'a> {
        Ctx {
            data,
            settings,
            slots: Slots::default(),
            focused_edit: None,
        }
    }

    fn build_at(tab: &mut BuildTab, ui: &mut Ui, ctx: &Ctx, now: u64) {
        ui.begin(now);
        tab.build(ui, &mut Fixed, AREA, ctx);
        ui.end();
    }

    fn build(tab: &mut BuildTab, ui: &mut Ui, ctx: &Ctx) {
        build_at(tab, ui, ctx, 0);
    }

    fn texts(ui: &Ui) -> Vec<String> {
        ui.frame()
            .nodes
            .iter()
            .filter_map(|node| match &node.visual {
                Visual::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_sub_tabs_switch_the_content() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        // A tela abre na Meta, como a v1.
        assert!(texts(&ui).iter().any(|text| text.contains("META")));
        assert!(!ui.frame().has_hit(generate_id()));

        tab.on_click(sub_tab_id(1), &ctx(&data, &settings));
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(ui.frame().has_hit(generate_id()));

        tab.on_click(sub_tab_id(2), &ctx(&data, &settings));
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(ui.frame().has_hit(search_id()));
        // As opções de sorteio somem na montagem à mão.
        assert!(!ui.frame().has_hit(option_id(0)));
    }

    // --- Sub-aba Meta ---

    /// Resposta plausível: slugs reais do `statsMap.json`, em ordem estável.
    fn meta_stats_response() -> crate::meta_stats::Stats {
        use crate::meta_stats::{Section, Stats, Total};

        let map = data::stats_map().expect("statsMap.json do repositório");
        fn sorted(keys: Vec<&String>) -> Vec<&String> {
            let mut keys = keys;
            keys.sort();
            keys
        }
        let section = |slugs: Vec<&String>, games: u64| Section {
            items: slugs
                .iter()
                .enumerate()
                .map(|(rank, slug)| {
                    (
                        (*slug).clone(),
                        crate::meta_stats::ItemStat {
                            loadouts_percentage: 40.0 - rank as f64,
                            change: Some(if rank % 2 == 0 { 3.1 } else { -2.4 }),
                            is_new: Some(rank == 0),
                        },
                    )
                })
                .collect(),
            total: Total { games },
        };

        Stats {
            strategem: section(
                sorted(map.strategem.keys().collect())
                    .into_iter()
                    .take(12)
                    .collect(),
                4_009,
            ),
            weapons: section(sorted(map.weapons.keys().collect()), 0),
            armor: section(
                sorted(map.armor.keys().collect())
                    .into_iter()
                    .take(6)
                    .collect(),
                0,
            ),
        }
    }

    fn answer(tab: &mut BuildTab, data: &GameData, ok: bool) {
        let key = tab.stats.key();
        let stats = ok.then(|| std::sync::Arc::new(meta_stats_response()));
        tab.set_meta(MetaResult { key, stats }, data);
    }

    #[test]
    fn the_meta_tab_asks_for_the_stats_and_then_shows_them() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        // A tela abre na Meta: a primeira construção registra a consulta e
        // mostra o aviso de carregando.
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert_eq!(
            tab.take_meta_request(),
            Some((Faction::Terminid, DIFFICULTIES[0]))
        );
        // O "carregando" é o texto com o caret piscando no lugar das
        // reticências.
        let loading = i18n::tr(settings.language)
            .build
            .meta_loading
            .trim_end_matches('.')
            .to_uppercase();
        assert!(texts(&ui).contains(&loading));
        assert!(
            ui.animating(),
            "o caret pisca enquanto a consulta não volta"
        );
        assert!(!ui.frame().has_hit(meta_generate_id()));
        // E só pede uma vez enquanto a consulta não volta.
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert_eq!(tab.take_meta_request(), None);

        answer(&mut tab, &data, true);
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        assert!(ui.frame().has_hit(meta_generate_id()));
        let texts = texts(&ui);
        assert!(!texts.contains(&loading));
        // Percentuais, o crédito com o total de partidas e a etiqueta de novo.
        assert!(texts.iter().any(|text| text == "40.0%"));
        assert!(texts
            .iter()
            .any(|text| text.contains("4.009") && text.contains("HELLDIVE.LIVE")));
        assert!(texts.iter().any(|text| text == "NOVO"));
        assert!(texts.iter().any(|text| text.starts_with('\u{25B2}')));
        assert!(texts.iter().any(|text| text.starts_with('\u{25BC}')));
    }

    #[test]
    fn changing_faction_or_difficulty_asks_again_and_ignores_the_old_answer() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        tab.take_meta_request();

        tab.on_click(faction_id(1), &ctx(&data, &settings));
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert_eq!(
            tab.take_meta_request(),
            Some((Faction::Automaton, DIFFICULTIES[0]))
        );

        // Resposta da facção anterior chega atrasada e é descartada.
        tab.set_meta(
            MetaResult {
                key: meta_stats::cache_key(Faction::Terminid, 0),
                stats: Some(std::sync::Arc::new(meta_stats_response())),
            },
            &data,
        );
        assert!(tab.stats.lists().is_none());

        // Clicar na facção que já está escolhida não refaz a consulta.
        answer(&mut tab, &data, true);
        tab.on_click(faction_id(1), &ctx(&data, &settings));
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert_eq!(tab.take_meta_request(), None);

        tab.on_click(difficulty_id(3), &ctx(&data, &settings));
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert_eq!(
            tab.take_meta_request(),
            Some((Faction::Automaton, DIFFICULTIES[3]))
        );
    }

    #[test]
    fn a_failed_query_shows_the_warning_instead_of_the_lists() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        build(&mut tab, &mut ui, &ctx(&data, &settings));
        tab.take_meta_request();
        answer(&mut tab, &data, false);
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        let error = i18n::tr(settings.language).build.meta_error.to_uppercase();
        assert!(texts(&ui).contains(&format!("! {error}")));
        assert!(!ui.frame().has_hit(meta_generate_id()));
        // E o botão, se clicado assim mesmo, não sorteia nada.
        tab.on_click(meta_generate_id(), &ctx(&data, &settings));
        assert!(tab.build.is_none());

        // O banner oferece tentar de novo, e a próxima construção pede outra vez.
        assert!(ui.frame().has_hit(meta_retry_id()));
        assert_eq!(
            tab.on_click(meta_retry_id(), &ctx(&data, &settings)),
            Some(Action::Redraw)
        );
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert_eq!(
            tab.take_meta_request(),
            Some((Faction::Terminid, DIFFICULTIES[0]))
        );
    }

    #[test]
    fn the_meta_button_rolls_a_build_out_of_the_listed_stratagems() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        build(&mut tab, &mut ui, &ctx(&data, &settings));
        tab.take_meta_request();
        answer(&mut tab, &data, true);

        tab.on_click(meta_generate_id(), &ctx(&data, &settings));
        let rolled = tab.build.clone().expect("build meta");
        assert!(rolled.stratagems.iter().all(Option::is_some));

        let top: Vec<u32> = tab
            .stats
            .lists()
            .unwrap()
            .stratagems
            .iter()
            .take(builds::META_TOP_STRATS)
            .map(|pick| pick.item)
            .collect();
        for id in rolled.stratagems.iter().flatten() {
            assert!(top.contains(id), "{id} está fora do top exibido");
        }

        // A build sorteada aparece na tela como qualquer outra.
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(ui.frame().has_hit(strat_lock_id(0)));
    }

    #[test]
    fn long_names_are_cut_to_fit_the_row() {
        let style = TextStyle::new(font::SIZE_TINY, Weight::Black);
        let name = "ORBITAL PRECISION STRIKE";
        let full = Fixed.text_size(name, style, f32::INFINITY).0;

        assert_eq!(ellipsize(&mut Fixed, name, style, full), name);
        let cut = ellipsize(&mut Fixed, name, style, full / 2.0);
        assert!(cut.ends_with('\u{2026}') && cut.len() < name.len());
        assert!(Fixed.text_size(&cut, style, f32::INFINITY).0 <= full / 2.0);
    }

    #[test]
    fn the_match_count_is_grouped_like_the_locale() {
        assert_eq!(grouped(4_009, Language::Pt), "4.009");
        assert_eq!(grouped(4_009, Language::En), "4,009");
        assert_eq!(grouped(1_234_567, Language::Pt), "1.234.567");
        assert_eq!(grouped(0, Language::Pt), "0");
        assert_eq!(grouped(999, Language::En), "999");
    }

    #[test]
    fn the_options_report_the_opposite_of_the_current_value() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();

        assert_eq!(
            tab.on_click(option_id(0), &ctx(&data, &settings)),
            Some(Action::Setting(Change::BuildMatchSet(false))),
            "a opção de sets começa ligada"
        );
        assert_eq!(
            tab.on_click(option_id(1), &ctx(&data, &settings)),
            Some(Action::Setting(Change::BuildBalanced(true)))
        );
        assert_eq!(
            tab.on_click(option_id(2), &ctx(&data, &settings)),
            Some(Action::Setting(Change::BuildMaxOneSentry(true)))
        );
    }

    #[test]
    fn generating_fills_the_screen_and_the_apply_button_sends_the_slots() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        assert_eq!(
            tab.on_click(apply_id(), &ctx(&data, &settings)),
            None,
            "sem build não há o que aplicar"
        );

        tab.on_click(generate_id(), &ctx(&data, &settings));
        let build = tab.build.clone().expect("build sorteada");
        assert!(build.stratagems.iter().all(Option::is_some));

        let action = tab.on_click(apply_id(), &ctx(&data, &settings));
        assert_eq!(action, Some(Action::SlotsChanged(build.stratagems)));

        // A build sorteada aparece na tela, com os cards de equipamento.
        build_at(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(ui.frame().has_hit(strat_lock_id(0)));
        assert!(ui.frame().has_hit(equip_lock_id(EquipSlot::Armor)));
    }

    #[test]
    fn locks_are_toggled_by_the_padlocks() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();

        tab.on_click(strat_lock_id(2), &ctx(&data, &settings));
        assert!(tab.locks.stratagem(2));
        tab.on_click(strat_lock_id(2), &ctx(&data, &settings));
        assert!(!tab.locks.stratagem(2));

        tab.on_click(equip_lock_id(EquipSlot::Cape), &ctx(&data, &settings));
        assert!(tab.locks.equip(EquipSlot::Cape));
    }

    #[test]
    fn a_locked_stratagem_survives_the_next_roll() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();

        tab.on_click(generate_id(), &ctx(&data, &settings));
        let first = tab.build.clone().unwrap();
        tab.on_click(strat_lock_id(0), &ctx(&data, &settings));
        tab.on_click(generate_id(), &ctx(&data, &settings));

        let second = tab.build.clone().unwrap();
        assert_eq!(second.stratagems[0], first.stratagems[0]);
    }

    #[test]
    fn the_custom_tab_equips_advances_and_clears() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let first = data.all()[0].id;

        tab.on_click(sub_tab_id(2), &ctx(&data, &settings));
        tab.on_click(card_id(first), &ctx(&data, &settings));
        assert_eq!(tab.build.as_ref().unwrap().stratagems[0], Some(first));
        assert_eq!(tab.custom_slot, 1, "o slot em edição avança");

        // Voltar ao slot 0 e clicar no mesmo card remove.
        tab.on_click(custom_slot_id(0), &ctx(&data, &settings));
        tab.on_click(card_id(first), &ctx(&data, &settings));
        assert_eq!(tab.build.as_ref().unwrap().stratagems[0], None);

        // O × do slot também limpa.
        tab.on_click(custom_slot_id(2), &ctx(&data, &settings));
        tab.on_click(card_id(first), &ctx(&data, &settings));
        assert_eq!(tab.build.as_ref().unwrap().stratagems[2], Some(first));
        tab.on_click(custom_clear_id(2), &ctx(&data, &settings));
        assert_eq!(tab.build.as_ref().unwrap().stratagems[2], None);
    }

    #[test]
    fn the_custom_tab_imports_the_current_slots_and_clears_everything() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let slots: Slots = [Some(data.all()[1].id), None, None, Some(data.all()[2].id)];
        let context = Ctx {
            slots,
            ..ctx(&data, &settings)
        };

        tab.on_click(import_slots_id(), &context);
        assert_eq!(tab.build.as_ref().unwrap().stratagems, slots);

        tab.set_search("orbital".into());
        tab.set_name("Rascunho".into());
        assert_eq!(
            tab.on_click(reset_id(), &context),
            Some(Action::SetEdits(vec![
                (search_id(), String::new()),
                (name_id(), String::new())
            ]))
        );
        assert!(tab.name.is_empty());
        assert!(tab.build.is_none());
        assert_eq!(tab.custom_slot, 0);
        assert!(tab.search().is_empty());
    }

    #[test]
    fn the_custom_search_filters_the_grid_and_answers_with_focus_and_clear() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        tab.sub = SubTab::Custom;
        tab.set_search("orbital".into());
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(tab.list.len() < data.all().len());
        assert_eq!(
            tab.on_click(search_id(), &ctx(&data, &settings)),
            Some(Action::FocusEdit(search_id()))
        );
        assert_eq!(
            tab.on_click(clear_search_id(), &ctx(&data, &settings)),
            Some(Action::SetEdits(vec![(search_id(), String::new())]))
        );

        tab.set_search("zzzz".into());
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(tab.list.is_empty());
        assert!(texts(&ui).iter().any(|text| text.contains("zzzz")));
    }

    #[test]
    fn only_the_visible_part_of_the_custom_grid_is_built() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        tab.sub = SubTab::Custom;
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        let cards = data
            .all()
            .iter()
            .filter(|strat| ui.frame().has_hit(card_id(strat.id)))
            .count();
        assert!(cards > 0 && cards < data.all().len());
    }

    #[test]
    fn the_dropdown_opens_picks_an_item_and_closes() {
        let data = data();
        let settings = Settings::default();
        let equipment = data::equipment().unwrap();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        tab.sub = SubTab::Custom;
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(ui.frame().has_hit(dropdown_id(EquipSlot::Armor)));

        tab.on_click(dropdown_id(EquipSlot::Armor), &ctx(&data, &settings));
        assert_eq!(tab.open, Some(EquipSlot::Armor));
        build(&mut tab, &mut ui, &ctx(&data, &settings));
        // A lista aberta cobre a aba: fora dela o clique fecha.
        assert!(ui.frame().has_hit(scrim_id()));
        assert!(ui.frame().has_hit(dropdown_row_id(EquipSlot::Armor, 1)));

        tab.on_click(dropdown_row_id(EquipSlot::Armor, 1), &ctx(&data, &settings));
        assert_eq!(tab.open, None);
        assert_eq!(
            tab.build.as_ref().unwrap().equip(EquipSlot::Armor),
            Some(equipment.at(EquipSlot::Armor, 0).unwrap().id())
        );

        // A primeira linha é o "nenhum", que esvazia a categoria.
        tab.on_click(dropdown_id(EquipSlot::Armor), &ctx(&data, &settings));
        tab.on_click(dropdown_row_id(EquipSlot::Armor, 0), &ctx(&data, &settings));
        assert_eq!(tab.build.as_ref().unwrap().equip(EquipSlot::Armor), None);
    }

    #[test]
    fn a_click_outside_or_escape_closes_the_dropdown() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();

        tab.on_click(dropdown_id(EquipSlot::Primary), &ctx(&data, &settings));
        assert_eq!(
            tab.on_click(scrim_id(), &ctx(&data, &settings)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.open, None);

        tab.on_click(dropdown_id(EquipSlot::Primary), &ctx(&data, &settings));
        assert_eq!(tab.on_key(crate::keys::VK_ESCAPE), Some(Action::Redraw));
        assert_eq!(tab.open, None);
        // Sem lista aberta a tecla não é nossa.
        assert_eq!(tab.on_key(crate::keys::VK_ESCAPE), None);
    }

    #[test]
    fn the_open_dropdown_is_drawn_over_the_page_and_scrolls() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        tab.sub = SubTab::Custom;
        tab.on_click(dropdown_id(EquipSlot::Armor), &ctx(&data, &settings));
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        // 107 armaduras não cabem na altura máxima: a lista rola.
        let row = ui
            .frame()
            .nodes
            .iter()
            .rev()
            .find(|node| matches!(node.visual, Visual::Text { .. }))
            .expect("linha da lista");
        assert!(
            ui.input(Input::Wheel {
                x: row.rect.center_x(),
                y: row.rect.center_y(),
                delta: -1.0,
            })
            .redraw
        );
    }

    /// Aba com builds salvas em memória: `loaded` já marcado para o teste não
    /// depender do que houver no `loadouts.json` da máquina.
    fn with_loadouts(data: &GameData) -> BuildTab {
        let mut tab = BuildTab::new();
        tab.loaded = true;
        tab.loadouts = vec![
            Loadout {
                id: "1".into(),
                name: "Bug Sweep".into(),
                slot_ids: vec![Some(data.all()[0].id), None, None, None],
                equip: None,
            },
            Loadout {
                id: "2".into(),
                name: "Bot Drop".into(),
                slot_ids: vec![Some(data.all()[1].id), Some(data.all()[2].id), None, None],
                equip: None,
            },
        ];
        tab
    }

    /// Construção numa janela alta, onde a página inteira cabe sem rolar: o
    /// que os testes de clique procuram fica sempre dentro do recorte.
    const TALL: Rect = Rect::new(0.0, 0.0, 820.0, 4_000.0);

    fn build_tall(tab: &mut BuildTab, ui: &mut Ui, ctx: &Ctx, now: u64) {
        ui.begin(now);
        tab.build(ui, &mut Fixed, TALL, ctx);
        ui.end();
    }

    /// Duas passagens com tempo de sobra entre elas: rolo, confirmação e
    /// toast já venceram.
    fn settle(tab: &mut BuildTab, ui: &mut Ui, ctx: &Ctx) {
        build_tall(tab, ui, ctx, 0);
        build_tall(tab, ui, ctx, 10_000);
    }

    fn saved_tab(data: &GameData) -> BuildTab {
        let mut tab = with_loadouts(data);
        tab.sub = SubTab::Saved;
        tab
    }

    #[test]
    fn the_saved_tab_lists_every_build_with_its_actions() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        let mut ui = Ui::new();
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);

        let texts = texts(&ui);
        assert!(texts.iter().any(|text| text == "BUG SWEEP"));
        assert!(texts.iter().any(|text| text == "BOT DROP"));
        // A contagem vai na sub-aba.
        assert!(texts.iter().any(|text| text == "SALVAS (2)"));
        for index in 0..2 {
            assert!(ui.frame().has_hit(loadout_apply_id(index)));
            assert!(ui.frame().has_hit(loadout_edit_id(index)));
            // Excluir fica à vista, sem depender do hover.
            assert!(ui.frame().has_hit(loadout_delete_id(index)));
        }
        // Os ícones dos estratagemas de cada build.
        let icon = format!("icons/{}", data.all()[0].imagem);
        assert!(ui.frame().nodes.iter().any(
            |node| matches!(&node.visual, Visual::Image { path, .. } if path.to_string_lossy() == icon)
        ));
        // A lista não mostra a build exibida embaixo.
        assert!(!ui.frame().has_hit(save_id()));
    }

    #[test]
    fn applying_a_saved_build_fills_the_slots_and_tags_the_row() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        let mut ui = Ui::new();
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);

        let expected: Slots = [Some(data.all()[1].id), Some(data.all()[2].id), None, None];
        assert_eq!(
            tab.on_click(loadout_apply_id(1), &ctx(&data, &settings)),
            Some(Action::Applied {
                slots: expected,
                name: "Bot Drop".into()
            })
        );
        // A build aplicada volta para a tela, vinculada à salva.
        assert_eq!(tab.build.as_ref().unwrap().stratagems, expected);
        assert_eq!(tab.linked.as_deref(), Some("2"));

        // Com os slots novos, a linha ganha a etiqueta, e o toast confirma.
        let context = Ctx {
            slots: expected,
            ..ctx(&data, &settings)
        };
        build_tall(&mut tab, &mut ui, &context, 0);
        let texts = texts(&ui);
        assert!(texts.iter().any(|text| text == "NOS SLOTS"));
        assert!(texts
            .iter()
            .any(|text| text.contains("BOT DROP") && text.contains("SLOTS DE MACRO")));
    }

    #[test]
    fn deleting_asks_for_a_second_click() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        let mut ui = Ui::new();
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);

        // O primeiro clique só arma a pergunta.
        assert_eq!(
            tab.on_click(loadout_delete_id(0), &ctx(&data, &settings)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.loadouts().len(), 2);
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(texts(&ui).iter().any(|text| text == "CONFIRMAR?"));

        // O segundo exclui.
        assert_eq!(
            tab.on_click(loadout_delete_id(0), &ctx(&data, &settings)),
            Some(Action::LoadoutsChanged)
        );
        assert_eq!(tab.loadouts().len(), 1);
        assert_eq!(tab.loadouts()[0].name, "Bot Drop");
    }

    #[test]
    fn the_delete_question_goes_away_by_itself_or_with_another_click() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        let mut ui = Ui::new();
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);

        // Outro clique qualquer desarma.
        tab.on_click(loadout_delete_id(0), &ctx(&data, &settings));
        tab.on_click(sub_tab_id(3), &ctx(&data, &settings));
        assert_eq!(tab.confirm, None);
        tab.on_click(loadout_delete_id(0), &ctx(&data, &settings));
        assert_eq!(
            tab.on_click(loadout_delete_id(0), &ctx(&data, &settings)),
            Some(Action::LoadoutsChanged)
        );

        // E, sem clique nenhum, a pergunta some depois de 3s.
        tab.on_click(loadout_delete_id(0), &ctx(&data, &settings));
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(ui.animating(), "a contagem da pergunta está na tela");
        build_tall(
            &mut tab,
            &mut ui,
            &ctx(&data, &settings),
            u64::from(CONFIRM_MS) + 100,
        );
        assert_eq!(tab.confirm, None);
        assert!(!texts(&ui).iter().any(|text| text == "CONFIRMAR?"));
        assert_eq!(
            tab.on_click(loadout_delete_id(0), &ctx(&data, &settings)),
            Some(Action::Redraw),
            "vencida, o clique arma de novo em vez de excluir"
        );
    }

    #[test]
    fn saving_needs_a_build_and_takes_the_typed_name() {
        let data = data();
        let settings = Settings::default();
        let mut tab = with_loadouts(&data);
        let mut ui = Ui::new();

        // Sem build na tela o botão nem existe.
        assert_eq!(tab.on_click(save_id(), &ctx(&data, &settings)), None);
        assert_eq!(tab.loadouts().len(), 2);

        tab.sub = SubTab::Random;
        tab.on_click(generate_id(), &ctx(&data, &settings));
        settle(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(ui.frame().has_hit(save_id()));
        assert!(texts(&ui).iter().any(|text| text.contains("NOVA BUILD")));

        tab.set_name("Minha Build".into());
        assert_eq!(
            tab.on_click(save_id(), &ctx(&data, &settings)),
            Some(Action::Saved("Minha Build".into()))
        );
        assert_eq!(tab.loadouts().len(), 3);
        assert_eq!(tab.loadouts()[2].name, "Minha Build");
        // O campo continua com o nome, e a build agora está vinculada.
        assert_eq!(tab.name, "Minha Build");
        assert_eq!(tab.linked.as_ref(), Some(&tab.loadouts()[2].id));

        settle(&mut tab, &mut ui, &ctx(&data, &settings));
        let texts = texts(&ui);
        assert!(texts.iter().any(|text| text.contains("SALVA COMO")));
        // Sem alteração não há o que salvar por cima.
        assert!(!ui.frame().has_hit(save_changes_id()));
        assert!(ui.frame().has_hit(save_as_new_id()));
    }

    #[test]
    fn a_taken_name_asks_before_replacing() {
        let data = data();
        let settings = Settings::default();
        let mut tab = with_loadouts(&data);
        let mut ui = Ui::new();
        tab.sub = SubTab::Random;
        tab.on_click(generate_id(), &ctx(&data, &settings));

        tab.set_name("bug sweep".into());
        assert_eq!(
            tab.on_click(save_id(), &ctx(&data, &settings)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.loadouts().len(), 2);
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        let texts = texts(&ui);
        assert!(texts.iter().any(|text| text == "SUBSTITUIR?"));
        assert!(texts.iter().any(|text| text.contains("JÁ EXISTE")));

        // Mudar o nome desarma a pergunta; voltar a ele pergunta de novo.
        tab.set_name("bug sweep 2".into());
        assert_eq!(tab.confirm, None);
        tab.set_name("bug sweep".into());
        tab.on_click(save_id(), &ctx(&data, &settings));

        let rolled = tab.build.clone().unwrap();
        assert_eq!(
            tab.on_click(save_id(), &ctx(&data, &settings)),
            Some(Action::Saved("Bug Sweep".into()))
        );
        assert_eq!(tab.loadouts().len(), 2, "substituiu em vez de duplicar");
        assert_eq!(tab.loadouts()[0].slot_ids, rolled.stratagems.to_vec());
        assert_eq!(tab.loadouts()[0].id, "1");
    }

    #[test]
    fn editing_opens_the_build_in_the_editor_and_saves_it_back() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        let mut ui = Ui::new();
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);

        assert_eq!(
            tab.on_click(loadout_edit_id(0), &ctx(&data, &settings)),
            Some(Action::SetEdits(vec![(name_id(), "Bug Sweep".into())]))
        );
        // A janela entrega o texto ao campo, que devolve pela aba.
        tab.set_name("Bug Sweep".into());
        assert_eq!(tab.sub, SubTab::Custom);
        assert_eq!(tab.custom_slot, 0);
        settle(&mut tab, &mut ui, &ctx(&data, &settings));
        assert!(texts(&ui).iter().any(|text| text.contains("SALVA COMO")));

        // Troca o segundo slot pela grade: a build fica com alteração.
        let extra = data.all()[5].id;
        tab.on_click(custom_slot_id(1), &ctx(&data, &settings));
        tab.on_click(card_id(extra), &ctx(&data, &settings));
        settle(&mut tab, &mut ui, &ctx(&data, &settings));
        let texts = texts(&ui);
        assert!(texts
            .iter()
            .any(|text| text.contains("EDITANDO") && text.contains("NÃO SALVAS")));
        assert!(ui.frame().has_hit(discard_id()));

        // Salvar alterações grava por cima, no mesmo id, e renomeia junto.
        tab.set_name("Bug Hunt".into());
        assert_eq!(
            tab.on_click(save_changes_id(), &ctx(&data, &settings)),
            Some(Action::Saved("Bug Hunt".into()))
        );
        assert_eq!(tab.loadouts().len(), 2);
        assert_eq!(tab.loadouts()[0].id, "1");
        assert_eq!(tab.loadouts()[0].name, "Bug Hunt");
        assert_eq!(tab.loadouts()[0].slot_ids[1], Some(extra));
    }

    #[test]
    fn renaming_onto_another_build_is_refused() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        let mut ui = Ui::new();
        tab.on_click(loadout_edit_id(0), &ctx(&data, &settings));
        tab.set_name("BOT DROP".into());

        assert_eq!(
            tab.on_click(save_changes_id(), &ctx(&data, &settings)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.loadouts()[0].name, "Bug Sweep");
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(texts(&ui).iter().any(|text| text.contains("OUTRA BUILD")));
        // Mudar o nome apaga o aviso.
        tab.set_name("Bug Hunt".into());
        assert!(!tab.taken);
    }

    #[test]
    fn save_as_new_keeps_the_original_and_names_the_copy() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        tab.on_click(loadout_edit_id(0), &ctx(&data, &settings));
        tab.set_name("Bug Sweep".into());

        assert_eq!(
            tab.on_click(save_as_new_id(), &ctx(&data, &settings)),
            Some(Action::Saved("Bug Sweep 2".into()))
        );
        assert_eq!(tab.loadouts().len(), 3);
        assert_eq!(tab.loadouts()[0].name, "Bug Sweep");
        // A tela passa a ser a cópia.
        assert_eq!(tab.linked.as_ref(), Some(&tab.loadouts()[2].id));
    }

    #[test]
    fn discarding_brings_the_saved_version_back() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        tab.on_click(loadout_edit_id(0), &ctx(&data, &settings));
        tab.set_name("Bug Sweep".into());
        let saved = tab.build.clone().unwrap();

        tab.on_click(custom_slot_id(3), &ctx(&data, &settings));
        tab.on_click(card_id(data.all()[9].id), &ctx(&data, &settings));
        tab.set_name("Outro nome".into());
        assert_ne!(tab.build.as_ref(), Some(&saved));

        assert_eq!(
            tab.on_click(discard_id(), &ctx(&data, &settings)),
            Some(Action::SetEdits(vec![(name_id(), "Bug Sweep".into())]))
        );
        assert_eq!(tab.build.as_ref(), Some(&saved));
    }

    #[test]
    fn rolling_a_new_build_unlinks_it_from_the_saved_one() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        tab.on_click(loadout_apply_id(0), &ctx(&data, &settings));
        tab.set_name("Bug Sweep".into());

        tab.sub = SubTab::Random;
        // O nome que o vínculo pôs no campo sai junto.
        assert_eq!(
            tab.on_click(generate_id(), &ctx(&data, &settings)),
            Some(Action::SetEdits(vec![(name_id(), String::new())]))
        );
        assert_eq!(tab.linked, None);
        assert!(tab.name.is_empty());

        // Um nome digitado à mão fica.
        tab.set_name("Rascunho".into());
        assert_eq!(
            tab.on_click(generate_id(), &ctx(&data, &settings)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.name, "Rascunho");
    }

    #[test]
    fn deleting_the_linked_build_keeps_it_on_screen_to_save_again() {
        let data = data();
        let settings = Settings::default();
        let mut tab = saved_tab(&data);
        tab.on_click(loadout_apply_id(0), &ctx(&data, &settings));
        tab.on_click(loadout_delete_id(0), &ctx(&data, &settings));
        tab.on_click(loadout_delete_id(0), &ctx(&data, &settings));

        assert_eq!(tab.loadouts().len(), 1);
        assert_eq!(tab.linked, None);
        assert!(tab.build.is_some());
    }

    #[test]
    fn an_empty_list_says_so() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.loaded = true;
        tab.sub = SubTab::Saved;
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        let tr = i18n::tr(settings.language);
        let texts = texts(&ui);
        assert!(texts.contains(&tr.build.saved_empty.to_string()));
        assert!(texts.contains(&widgets::sigil(tr.macros.nothing_here)));
        assert!(!ui.frame().has_hit(loadout_apply_id(0)));
        // Sem builds, a sub-aba não leva contagem.
        assert!(texts.iter().any(|text| text == "SALVAS"));
    }

    // --- Sorteio animado ---

    fn image_paths(ui: &Ui) -> Vec<String> {
        ui.frame()
            .nodes
            .iter()
            .filter_map(|node| match &node.visual {
                Visual::Image { path, .. } => Some(path.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_roll_spins_the_cards_and_then_settles() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.loaded = true;
        tab.sub = SubTab::Random;
        let mut ui = Ui::new();

        tab.on_click(generate_id(), &ctx(&data, &settings));
        settle(&mut tab, &mut ui, &ctx(&data, &settings));
        let first = tab.build.clone().unwrap();

        tab.on_click(strat_lock_id(0), &ctx(&data, &settings));
        tab.on_click(reroll_id(), &ctx(&data, &settings));
        let second = tab.build.clone().unwrap();
        assert_eq!(second.stratagems[0], first.stratagems[0], "o travado fica");

        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 1_000);
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 1_100);
        assert!(ui.animating(), "o rolo pede quadros");
        let reel = tab.roll.as_ref().expect("sorteio em curso");
        assert!(reel.reels[0].is_empty(), "card travado não gira");
        assert!(reel.reels[1..].iter().all(|frames| !frames.is_empty()));
        // O rolo do segundo slot começa no item que estava lá e termina no novo.
        let frames = &reel.reels[1];
        let name_of = |id: Option<u32>| data.by_id(id.unwrap()).unwrap().nome.clone();
        assert_eq!(frames.first().unwrap().name, name_of(first.stratagems[1]));
        assert_eq!(frames.last().unwrap().name, name_of(second.stratagems[1]));
        assert!(frames.len() <= REEL_FRAMES);
        // Enquanto gira, o nome do item sorteado ainda não está no card. As
        // amostras só evitam os itens do próprio card, então outro rolo pode
        // passar pelo mesmo nome: a checagem usa um sorteado que só o rolo dele
        // contém.
        let rolled = (1..4)
            .map(|slot| (slot, name_of(second.stratagems[slot])))
            .find(|(slot, name)| {
                reel.reels.iter().enumerate().all(|(other, frames)| {
                    other == *slot || frames.iter().all(|frame| frame.name != *name)
                })
            })
            .map(|(_, name)| name.to_uppercase())
            .expect("algum sorteado só aparece no próprio rolo");
        assert!(!texts(&ui).contains(&rolled));

        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 5_000);
        assert!(tab.roll.is_none());
        assert!(texts(&ui).contains(&rolled));
        // Parado, cada card mostra um ícone só: o do item sorteado.
        let booster = second
            .item(data::equipment().unwrap(), EquipSlot::Booster)
            .unwrap();
        let path = format!("icons/{}", booster.imagem());
        assert_eq!(
            image_paths(&ui)
                .iter()
                .filter(|image| **image == path)
                .count(),
            1
        );
        assert!(path.ends_with(".webp"), "o booster tem ícone: {path}");
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 6_000);
        assert!(!ui.animating(), "tudo parado, sem timer");
    }

    #[test]
    fn the_roll_is_skipped_when_motion_is_reduced() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.sub = SubTab::Random;
        let mut ui = Ui::new();
        ui.set_reduced_motion(true);

        tab.on_click(generate_id(), &ctx(&data, &settings));
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(tab.roll.is_none());
        let name = data
            .by_id(tab.build.as_ref().unwrap().stratagems[0].unwrap())
            .unwrap()
            .nome
            .to_uppercase();
        assert!(texts(&ui).contains(&name));
    }

    #[test]
    fn rolling_brings_the_page_down_to_the_build() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.loaded = true;
        let mut ui = Ui::new();
        let context = ctx(&data, &settings);

        // Na Meta o botão fica em cima e a build sai depois das listas.
        build(&mut tab, &mut ui, &context);
        tab.take_meta_request();
        answer(&mut tab, &data, true);
        build(&mut tab, &mut ui, &context);
        tab.on_click(meta_generate_id(), &context);
        for now in [0, 50, 200, 1_000] {
            build_at(&mut tab, &mut ui, &context, now);
        }
        let current = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(&node.visual, Visual::Text { text, .. } if text == "BUILD_ATUAL.SYS"))
            .expect("barra da build atual")
            .rect;
        assert!(
            current.y >= AREA.y && current.y < AREA.bottom() - REVEAL_VISIBLE + 40.0,
            "a build ficou fora de vista: {current:?}"
        );

        // Rolar de novo pela barra da build não mexe na página.
        let before = current.y;
        assert!(ui.frame().has_hit(reroll_id()));
        tab.on_click(reroll_id(), &context);
        for now in [1_100, 1_400, 3_000] {
            build_at(&mut tab, &mut ui, &context, now);
        }
        let after = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(&node.visual, Visual::Text { text, .. } if text == "BUILD_ATUAL.SYS"))
            .unwrap()
            .rect;
        assert_eq!(after.y, before);
    }

    #[test]
    fn reroll_only_lives_where_there_is_something_to_roll() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.loaded = true;
        let mut ui = Ui::new();

        tab.sub = SubTab::Random;
        tab.on_click(generate_id(), &ctx(&data, &settings));
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(ui.frame().has_hit(reroll_id()));

        // Na montagem à mão não há o que sortear.
        tab.sub = SubTab::Custom;
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(!ui.frame().has_hit(reroll_id()));
        assert!(ui.frame().has_hit(apply_id()));

        // Na Meta, só com as estatísticas carregadas.
        tab.sub = SubTab::Meta;
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(!ui.frame().has_hit(reroll_id()));
    }

    #[test]
    fn the_toast_confirms_and_then_leaves() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.loaded = true;
        tab.sub = SubTab::Random;
        let mut ui = Ui::new();

        tab.on_click(generate_id(), &ctx(&data, &settings));
        tab.set_name("Toast".into());
        tab.on_click(save_id(), &ctx(&data, &settings));
        let toast = "\u{201c}TOAST\u{201d} SALVA".to_string();
        build_tall(&mut tab, &mut ui, &ctx(&data, &settings), 0);
        assert!(texts(&ui).contains(&toast));
        build_tall(
            &mut tab,
            &mut ui,
            &ctx(&data, &settings),
            u64::from(NOTICE_MS) + 100,
        );
        assert!(tab.notice.is_none());
        assert!(!texts(&ui).contains(&toast));
    }

    #[test]
    fn the_field_text_is_handed_to_a_new_native_child() {
        let mut tab = BuildTab::new();
        tab.set_name("Bug Sweep".into());
        assert_eq!(tab.edit_text(name_id()), Some("Bug Sweep"));
        tab.set_search("orb".into());
        assert_eq!(tab.edit_text(search_id()), Some("orb"));
        assert_eq!(tab.edit_text(id("outro")), None);
    }

    #[test]
    fn the_name_field_is_capped_like_the_legacy_input() {
        let mut tab = BuildTab::new();
        tab.set_name("x".repeat(40));
        assert_eq!(tab.name.chars().count(), NAME_MAX_CHARS);
    }

    #[test]
    fn the_whole_tab_follows_the_language() {
        let data = data();
        let mut tab = BuildTab::new();
        let mut ui = Ui::new();

        let pt = Settings::default();
        build(&mut tab, &mut ui, &ctx(&data, &pt));
        assert!(texts(&ui).iter().any(|text| text.contains("ALEATÓRIA")));

        let en = Settings {
            language: crate::settings::Language::En,
            ..Settings::default()
        };
        build(&mut tab, &mut ui, &ctx(&data, &en));
        let after = texts(&ui);
        assert!(after.iter().any(|text| text.contains("RANDOM")));
        assert!(!after.iter().any(|text| text.contains("ALEATÓRIA")));
    }

    #[test]
    fn an_unknown_click_is_not_ours() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        assert_eq!(tab.on_click(id("outra.tela"), &ctx(&data, &settings)), None);
    }
}
