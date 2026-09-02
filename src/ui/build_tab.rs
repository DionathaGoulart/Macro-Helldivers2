//! Aba de builds: opções de sorteio, as sub-abas Meta/Aleatória/Personalizada e
//! a build exibida.
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

use crate::builds::{self, Build, Locks, MetaLists, Rules};
use crate::data::{self, EquipSlot, Equipment, GameData, Item, StratMeta, Stratagem};
use crate::i18n::{self, Tr};
use crate::loadouts::{self, Loadout};
use crate::meta_stats::{self, Faction, ItemStat, MetaResult, DIFFICULTIES};
use crate::settings::{Language, Settings, SLOT_COUNT};
use crate::shared::Slots;
use crate::ui::settings_tab::Change;
use crate::ui::theme::{self, font, Color};
use crate::ui::toolkit::{
    columns, grid_cell, grid_height, id, id_at, Align, Id, ImageStyle, Measure, Rect, TextStyle,
    Ui, Weight,
};
use crate::ui::widgets::{self, ButtonVariant, CardHeader, CardState, ChipLayout, ItemCard};

/// `px-6` da coluna de conteúdo.
const PAGE_PADDING: f32 = 24.0;
/// Espaço reservado à direita para a barra de rolagem da página.
const SCROLL_GUTTER: f32 = 12.0;
/// `space-y-6` entre os blocos.
const SECTION_GAP: f32 = 24.0;

/// Grupo das sub-abas (`p-1` em volta de botões `py-2.5`).
const SUBTAB_HEIGHT: f32 = 44.0;
const SUBTAB_PADDING: f32 = 4.0;
const SUBTAB_WIDTH: f32 = 132.0;

/// `p-4` das linhas de opção.
const TOGGLE_HEIGHT: f32 = 52.0;
const TOGGLE_GAP: f32 = 12.0;
/// Altura de um rótulo de campo.
const LABEL_HEIGHT: f32 = 14.0;
const LABEL_GAP: f32 = 8.0;
/// `py-4 px-10` do botão de sortear.
const GENERATE_HEIGHT: f32 = 52.0;
const GENERATE_WIDTH: f32 = 260.0;
/// Botões do header do card personalizado (`py-2 px-4`).
const HEADER_BUTTON_HEIGHT: f32 = 30.0;
const HEADER_BUTTON_WIDTH: f32 = 132.0;
/// A faixa do header de um card começa logo depois do padding de cima.
const CARD_HEADER_HEIGHT: f32 = 30.0;

/// Slot em edição da build personalizada (`p-3`, ícone `w-14 h-14`).
const CUSTOM_SLOT_HEIGHT: f32 = 116.0;
const CUSTOM_SLOT_IMAGE: f32 = 56.0;
/// `grid-cols-5` da grade personalizada.
const CUSTOM_GRID_COLS: usize = 5;
/// `max-h-[420px]` da grade.
const CUSTOM_GRID_MAX_HEIGHT: f32 = 420.0;
/// `gap-3` das grades.
const GRID_GAP: f32 = 12.0;
/// `grid-cols-4` do equipamento.
const EQUIP_COLS: usize = 4;
/// × que esvazia um slot em edição.
const CLEAR_SIZE: f32 = 20.0;
const HOVER_MS: u32 = 180;

/// Botões de facção (`py-3`) e de dificuldade (`py-2`) da sub-aba Meta.
const META_FACTION_HEIGHT: f32 = 38.0;
const META_DIFFICULTY_HEIGHT: f32 = 32.0;
/// `gap-2` entre eles e `mb-4` depois das escolhas.
const META_CHOICE_GAP: f32 = 8.0;
const META_BLOCK_GAP: f32 = 16.0;
/// `w-full py-4` do botão de gerar a build meta.
const META_GENERATE_HEIGHT: f32 = 48.0;
/// Altura do aviso de carregando/erro (`py-6`).
const META_STATUS_HEIGHT: f32 = 48.0;
/// `gap-6` entre a coluna dos estratagemas e a das armas.
const META_COLUMN_GAP: f32 = 24.0;
/// Linha de item das listas e o `space-y-1` entre elas.
const META_ROW_HEIGHT: f32 = 24.0;
const META_ROW_GAP: f32 = 4.0;
/// `gap-2` entre as partes de uma linha.
const META_CELL_GAP: f32 = 8.0;
/// Ícone (`w-6 h-6`), barra proporcional (`w-14 h-1.5`) e as colunas de número.
const META_ICON: f32 = 24.0;
const META_BAR_WIDTH: f32 = 56.0;
const META_BAR_HEIGHT: f32 = 6.0;
const META_NEW_WIDTH: f32 = 34.0;
const META_CHANGE_WIDTH: f32 = 38.0;
const META_PERCENT_WIDTH: f32 = 44.0;
/// Período do pulso do aviso de carregando (`animate-pulse`).
const META_PULSE_MS: u32 = 1_400;

/// Largura do campo de nome da build e do botão de salvar. Os chips em si vêm
/// de `widgets` — o painel do overlay mostra a mesma fileira.
const NAME_WIDTH: f32 = 200.0;
const SAVE_WIDTH: f32 = 120.0;
/// `maxLength={24}` do campo de nome da v1.
const NAME_MAX_CHARS: usize = 24;

/// Sub-abas da tela, na ordem da v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubTab {
    Meta,
    Random,
    Custom,
}

impl SubTab {
    const ALL: [SubTab; 3] = [SubTab::Meta, SubTab::Random, SubTab::Custom];

    fn label(self, tr: &'static Tr) -> &'static str {
        match self {
            SubTab::Meta => tr.build.sub_meta,
            SubTab::Random => tr.build.sub_random,
            SubTab::Custom => tr.build.sub_custom,
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
    /// Uma build foi salva: além de gravar, o campo de nome é esvaziado.
    Saved,
    /// Trazer um `EDIT` da aba para a frente.
    FocusEdit(Id),
    /// Esvaziar um `EDIT` da aba.
    ClearEdit(Id),
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
/// contador — nenhuma delas chega perto de mil itens.
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

fn loadout_id(index: usize) -> Id {
    id_at("build.loadout", index)
}

fn loadout_delete_id(index: usize) -> Id {
    id_at("build.loadout.delete", index)
}

/// Id do campo com o nome da build. A janela precisa dele para reconhecer o
/// `EDIT` nativo.
pub fn name_id() -> Id {
    id("build.name")
}

fn save_id() -> Id {
    id("build.save")
}

// --- Estilos ---

fn label_style() -> TextStyle {
    TextStyle::new(font::SIZE_LABEL, Weight::Black).tracking(font::TRACKING_LABEL)
}

fn hint_style() -> TextStyle {
    TextStyle::new(font::SIZE_TINY, Weight::Regular).wrap()
}

/// Em que pé está a consulta da sub-aba Meta.
enum MetaState {
    /// Combinação ainda não pedida — a próxima construção registra o pedido.
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

/// Estado da aba entre passagens de construção.
pub struct BuildTab {
    sub: SubTab,
    /// A build exibida — sorteada, montada à mão ou vinda de uma build salva.
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
    /// Nome digitado para a próxima build salva.
    name: String,
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
        self.name = text.chars().take(NAME_MAX_CHARS).collect();
    }

    /// Builds salvas, para a janela gravá-las depois de uma mudança.
    pub fn loadouts(&self) -> &[Loadout] {
        &self.loadouts
    }

    /// Relê o `loadouts.json` — a importação de backup o reescreve por fora.
    pub fn reload_loadouts(&mut self) {
        self.loadouts = loadouts::load_loadouts();
        self.loaded = true;
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

    /// Resposta de uma consulta. Resposta de outra combinação é descartada — o
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
        // Primeira visita à aba: é aqui que `equipment.json` sai do disco (R1).
        let equipment = data::equipment();

        let view = area.inset_xy(PAGE_PADDING, 16.0);
        let width = view.w - SCROLL_GUTTER;
        let offset = ui.scroll_begin(scroll_id(), view);
        let mut y = view.y - offset;

        self.sub_tabs(ui, Rect::new(view.x, y, width, SUBTAB_HEIGHT), ctx);
        y += SUBTAB_HEIGHT + SECTION_GAP;

        // As opções valem para os dois sorteios, e não para a montagem à mão.
        if self.sub != SubTab::Custom {
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
        }

        // As builds salvas ficam entre o sorteio e a build exibida, como na v1.
        let chips = self.chip_layout(measure, width - widgets::CARD_PADDING * 2.0);
        let height = saved_height(&chips);
        self.saved_card(ui, Rect::new(view.x, y, width, height), &chips, ctx);
        y += height + SECTION_GAP;

        if let Some(build) = self.build.clone() {
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

        ui.scroll_end(scroll_id(), view, y - SECTION_GAP - (view.y - offset));

        // A lista aberta sai por cima de tudo e fora do recorte da página: ela
        // precisa cobrir o que vem depois do campo que a abriu.
        if let Some(slot) = self.open {
            self.dropdown_list(ui, area, slot, ctx, equipment);
        }
    }

    /// Grupo das três sub-abas, centralizado.
    fn sub_tabs(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let width = SUBTAB_WIDTH * SubTab::ALL.len() as f32 + SUBTAB_PADDING * 2.0;
        let group = rect.centered(width, SUBTAB_HEIGHT);
        ui.fill(group, theme::RADIUS_CARD, theme::SURFACE);
        ui.stroke(
            group,
            theme::RADIUS_CARD,
            theme::HAIRLINE_WIDTH,
            theme::HAIRLINE,
        );

        let inner = group.inset(SUBTAB_PADDING);
        for (index, sub) in SubTab::ALL.into_iter().enumerate() {
            let cell = Rect::new(
                inner.x + SUBTAB_WIDTH * index as f32,
                inner.y,
                SUBTAB_WIDTH,
                inner.h,
            );
            sub_tab(
                ui,
                sub_tab_id(index),
                cell,
                sub.label(ctx.tr()),
                self.sub == sub,
            );
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
        let mut content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: tr.build.meta,
                accent: theme::RED,
            }),
        );

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
            difficulty_button(
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
                let (message, color) = match self.stats.state {
                    MetaState::Loading | MetaState::Idle => {
                        // O pulso do legado: o aviso respira enquanto a consulta
                        // não volta, e o timer morre junto com ele.
                        let pulse = 0.45 + 0.55 * ui.pulse(META_PULSE_MS);
                        (tr.build.meta_loading, theme::CYAN.alpha(0.7 * pulse))
                    }
                    _ => (tr.build.meta_error, theme::RED.alpha(0.8)),
                };
                ui.text(
                    content.with_h(META_STATUS_HEIGHT),
                    message.to_uppercase(),
                    label_style().align(Align::Center).middle(),
                    color,
                );
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
            &format!("\u{1F3C6} {}", tr.build.meta_generate),
            ButtonVariant::Secondary,
            theme::YELLOW,
        );
        content.skip_top(META_BLOCK_GAP);

        let area = content.cut_top(meta_columns_height(lists));
        let pair = columns(area, 2, META_COLUMN_GAP);
        self.meta_stratagems(ui, measure, pair[0], lists, ctx);
        self.meta_gear(ui, measure, pair[1], lists, ctx);

        content.skip_top(META_BLOCK_GAP);
        ui.text(
            content.cut_top(LABEL_HEIGHT),
            format!(
                "{} · {} {}",
                tr.build.meta_credit,
                grouped(lists.games, ctx.settings.language),
                tr.build.meta_games
            )
            .to_uppercase(),
            TextStyle::new(8.0, Weight::Black)
                .tracking(font::TRACKING_WIDE)
                .align(Align::Center),
            theme::TEXT_DIM,
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
        ui.text(
            cursor.cut_top(LABEL_HEIGHT),
            ctx.tr().build.meta_top_strats.to_uppercase(),
            label_style(),
            theme::TEXT_DIM,
        );
        cursor.skip_top(LABEL_GAP);

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
                    raster_icon(item.imagem()).as_deref(),
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
            // O ícone da passiva vem em SVG, que o decodificador não lê: a linha
            // mostra só o nome, como no legado.
            meta_item_row(ui, measure, row, &pick.item, None, pick.stat);
        }
    }

    /// Sub-aba Aleatória: o botão de sortear e a explicação.
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
            &format!("\u{1F3B2} {}", tr.build.generate),
            ButtonVariant::Primary,
            theme::YELLOW,
        );
        content.skip_top(LABEL_GAP);

        let height = measure.text_size(tr.build.hint, hint_style(), content.w).1;
        ui.text(
            content.with_h(height),
            tr.build.hint,
            hint_style().align(Align::Center),
            theme::TEXT_DIM,
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
                + (equip_rows() - 1.0) * TOGGLE_GAP
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
            + grid
            + equip
    }

    fn custom_grid_height(&self, width: f32) -> f32 {
        if self.list.is_empty() {
            return 48.0;
        }
        let cell = custom_cell(width);
        grid_height(self.list.len(), CUSTOM_GRID_COLS, cell, GRID_GAP).min(CUSTOM_GRID_MAX_HEIGHT)
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
        let mut content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: tr.build.custom_title,
                accent: theme::CYAN,
            }),
        );
        self.custom_header_buttons(ui, rect, ctx);

        let height = measure
            .text_size(tr.build.custom_hint, hint_style(), content.w)
            .1;
        ui.text(
            content.cut_top(height),
            tr.build.custom_hint,
            hint_style(),
            theme::TEXT_DIM,
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
        content.skip_top(LABEL_GAP);

        let grid = content.cut_top(self.custom_grid_height(content.w));
        if self.list.is_empty() {
            ui.text(
                grid,
                format!(
                    "{} \u{201c}{}\u{201d}",
                    tr.macros.search_no_results,
                    self.search.trim()
                ),
                label_style().align(Align::Center).middle(),
                theme::TEXT_DIM,
            );
        } else {
            self.custom_grid(ui, grid, ctx);
        }

        // Equipamento opcional.
        let Some(equipment) = equipment else {
            return;
        };
        content.skip_top(SECTION_GAP);
        ui.text(
            content.cut_top(LABEL_HEIGHT),
            tr.build.custom_equipment.to_uppercase(),
            label_style(),
            theme::TEXT_DIM,
        );
        content.skip_top(LABEL_GAP);
        self.equipment_fields(ui, content, ctx, equipment);
    }

    /// "Usar slots atuais" e "Limpar tudo", encostados no header do card.
    fn custom_header_buttons(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let row = Rect::new(
            rect.x + widgets::CARD_PADDING,
            rect.y + widgets::CARD_PADDING,
            rect.w - widgets::CARD_PADDING * 2.0,
            CARD_HEADER_HEIGHT,
        )
        .middle_row(HEADER_BUTTON_HEIGHT);

        let mut cursor = row;
        let reset = cursor.cut_right(HEADER_BUTTON_WIDTH);
        cursor.cut_right(GRID_GAP);
        let import = cursor.cut_right(HEADER_BUTTON_WIDTH);
        widgets::button(
            ui,
            import_slots_id(),
            import,
            tr.build.custom_import,
            ButtonVariant::Secondary,
            theme::CYAN,
        );
        widgets::button(
            ui,
            reset_id(),
            reset,
            tr.build.custom_clear,
            ButtonVariant::Secondary,
            theme::RED,
        );
    }

    /// Um dos quatro slots em edição da build personalizada.
    fn custom_slot(&self, ui: &mut Ui, index: usize, rect: Rect, ctx: &Ctx) {
        let id = custom_slot_id(index);
        let active = self.custom_slot == index;
        let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
        let strat = self
            .build
            .as_ref()
            .and_then(|build| build.stratagems[index])
            .and_then(|id| ctx.data.by_id(id));

        ui.fill(
            rect,
            theme::RADIUS_CARD,
            if active {
                theme::CYAN.alpha(0.05).over(theme::CARD_BG)
            } else {
                theme::CARD_BG
            },
        );
        ui.stroke(
            rect,
            theme::RADIUS_CARD,
            2.0,
            if active {
                theme::CYAN.alpha(0.6)
            } else {
                theme::BORDER.mix(theme::CYAN.alpha(0.3), hover)
            },
        );
        if active {
            ui.glow(rect, theme::RADIUS_CARD, theme::CYAN);
        }

        let mut content = rect.inset(12.0);
        ui.text(
            content.cut_top(LABEL_HEIGHT),
            format!("{} {}", ctx.tr().build.stratagem, index + 1).to_uppercase(),
            TextStyle::new(8.0, Weight::Black)
                .tracking(font::TRACKING_WIDE)
                .align(Align::Center),
            if active { theme::CYAN } else { theme::TEXT_DIM },
        );

        let picture = content
            .cut_top(CUSTOM_SLOT_IMAGE)
            .centered(CUSTOM_SLOT_IMAGE, CUSTOM_SLOT_IMAGE);
        match strat {
            Some(strat) => ui.image_styled(
                picture,
                format!("icons/{}", strat.imagem),
                ImageStyle::FILL.contain(),
            ),
            None => ui.text(
                picture,
                "\u{25A3}",
                TextStyle::new(20.0, Weight::Regular)
                    .align(Align::Center)
                    .middle(),
                theme::BORDER,
            ),
        }
        ui.text(
            content,
            strat
                .map(|strat| strat.nome.to_uppercase())
                .unwrap_or_else(|| "—".into()),
            TextStyle::new(font::SIZE_TINY, Weight::Black)
                .align(Align::Center)
                .wrap(),
            theme::TEXT,
        );
        ui.hit(id, rect);

        // O × sai por cima e é registrado depois, então ganha a sobreposição.
        if strat.is_some() {
            let clear = custom_clear_id(index);
            let button = Rect::new(
                rect.right() - CLEAR_SIZE / 2.0,
                rect.y - CLEAR_SIZE / 2.0,
                CLEAR_SIZE,
                CLEAR_SIZE,
            );
            let strong = ui.is_hot(clear);
            ui.ellipse(button, theme::RED.alpha(if strong { 1.0 } else { 0.8 }));
            ui.text(
                button,
                "×",
                TextStyle::new(font::SIZE_BODY, Weight::Black)
                    .align(Align::Center)
                    .middle(),
                theme::TEXT,
            );
            ui.hit(clear, button);
        }
    }

    fn search_field(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let focused = ctx.focused_edit == Some(search_id());
        let placeholder =
            (self.search.is_empty() && !focused).then_some(ctx.tr().macros.search_placeholder);
        widgets::edit_host(ui, search_id(), rect, focused, placeholder);

        if self.search.is_empty() {
            return;
        }
        let id = clear_search_id();
        let button = Rect::new(
            rect.right() - CLEAR_SIZE - 14.0,
            rect.center_y() - CLEAR_SIZE / 2.0,
            CLEAR_SIZE,
            CLEAR_SIZE,
        );
        let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
        ui.text(
            button,
            "×",
            TextStyle::new(font::SIZE_CARD_HEADER, Weight::Black)
                .align(Align::Center)
                .middle(),
            theme::TEXT_DIM.mix(theme::TEXT, hover),
        );
        ui.hit(id, button);
    }

    /// Grade de 5 colunas, rolável, com as mesmas regras de clique da aba de
    /// macros — só que mexendo na build, e não nos slots.
    fn custom_grid(&self, ui: &mut Ui, view: Rect, ctx: &Ctx) {
        let cell = custom_cell(view.w);
        let content = grid_height(self.list.len(), CUSTOM_GRID_COLS, cell, GRID_GAP);
        let empty = Build::default();
        let build = self.build.as_ref().unwrap_or(&empty);

        let offset = ui.scroll_begin(grid_id(), view);
        for (index, strat_id) in self.list.iter().enumerate() {
            let rect = grid_cell(
                Rect::new(view.x, view.y - offset, view.w, view.h),
                CUSTOM_GRID_COLS,
                cell,
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
                    accent: tag_color(strat.primary_tag().unwrap_or_default()),
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
            let mut cell = grid_cell(rect, EQUIP_COLS, cell_height, TOGGLE_GAP, index);
            ui.text(
                cell.cut_top(LABEL_HEIGHT),
                tr.build.equip_label(slot).to_uppercase(),
                TextStyle::new(8.0, Weight::Black).tracking(font::TRACKING_WIDE),
                theme::TEXT_DIM,
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
        // Abaixo do campo, a não ser que não caiba — aí sobe. E, se nem assim
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
            // A primeira linha é o "— Nenhum —" da v1; as demais seguem a ordem
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

    /// Onde cada chip cai, respeitando a largura disponível.
    fn chip_layout(&self, measure: &mut dyn Measure, width: f32) -> ChipLayout {
        widgets::chip_layout(
            measure,
            self.loadouts.iter().map(|loadout| loadout.name.as_str()),
            width,
        )
    }

    fn saved_card(&self, ui: &mut Ui, rect: Rect, layout: &ChipLayout, ctx: &Ctx) {
        let tr = ctx.tr();
        let mut content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: tr.build.saved,
                accent: theme::YELLOW,
            }),
        );

        let area = content.cut_top(layout.height);
        if self.loadouts.is_empty() {
            ui.text(
                area.with_h(widgets::CHIP_HEIGHT),
                tr.build.saved_empty.to_uppercase(),
                label_style().middle(),
                theme::TEXT_DIM,
            );
        } else {
            let active = builds::active_loadout(&self.loadouts, ctx.slots, ctx.data);
            for chip in &layout.chips {
                widgets::loadout_chip(
                    ui,
                    loadout_id(chip.index),
                    Some(loadout_delete_id(chip.index)),
                    chip.rect(area),
                    &self.loadouts[chip.index].name,
                    active == Some(chip.index),
                );
            }
        }
        content.skip_top(widgets::CHIP_GAP);

        // Nome e botão, encostados à direita como o `ml-auto` da v1.
        let mut row = content.cut_top(widgets::CONTROL_HEIGHT);
        let save = row.cut_right(SAVE_WIDTH);
        row.cut_right(widgets::CHIP_GAP);
        let field = row.cut_right(NAME_WIDTH);

        let focused = ctx.focused_edit == Some(name_id());
        let placeholder = (self.name.is_empty() && !focused).then_some(tr.build.save_placeholder);
        widgets::edit_host(ui, name_id(), field, focused, placeholder);

        // Sem build na tela não há o que salvar: o botão fica só com o texto
        // apagado, que é como o `disabled:opacity-30` da v1 se lia.
        let can_save = self.build.as_ref().is_some_and(Build::has_stratagem);
        widgets::button(
            ui,
            save_id(),
            save,
            tr.build.save_build,
            if can_save {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
            theme::YELLOW,
        );
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
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: tr.build.stratagems,
                accent: theme::CYAN,
            }),
        );
        // O botão de aplicar mora no header, como na v1.
        let header = Rect::new(
            rect.x + widgets::CARD_PADDING,
            rect.y + widgets::CARD_PADDING,
            rect.w - widgets::CARD_PADDING * 2.0,
            CARD_HEADER_HEIGHT,
        )
        .middle_row(HEADER_BUTTON_HEIGHT);
        let mut cursor = header;
        let apply = cursor.cut_right(HEADER_BUTTON_WIDTH * 1.6);
        widgets::button(
            ui,
            apply_id(),
            apply,
            tr.build.apply_stratagems,
            ButtonVariant::Secondary,
            theme::CYAN,
        );

        for (index, cell) in columns(content, SLOT_COUNT, GRID_GAP)
            .into_iter()
            .enumerate()
        {
            let strat = build.stratagems[index].and_then(|id| ctx.data.by_id(id));
            let label = format!("{} {}", tr.build.stratagem, index + 1);
            let card = ItemCard {
                label: &label,
                name: strat.map(|strat| strat.nome.as_str()).unwrap_or("—"),
                image: strat.map(|strat| strat.imagem.as_str()),
                subtitle: None,
                description: None,
                badge: None,
                locked: self.locks.stratagem(index),
            };
            let height = widgets::item_card_height(measure, &card, cell.w);
            widgets::build_item_card(
                ui,
                measure,
                strat_lock_id(index),
                cell.with_h(height),
                &card,
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
                    // Booster e passiva vêm da wiki em SVG, que o decodificador
                    // não lê: o card mostra o marcador vazio no lugar.
                    image: raster_icon(item.imagem()),
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
            Some(CardHeader {
                title: ctx.tr().build.equipment,
                accent: theme::YELLOW,
            }),
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
                widgets::build_item_card(
                    ui,
                    measure,
                    equip_lock_id(card.slot),
                    rect,
                    &card.as_item_card(),
                );
            }
            y += height + GRID_GAP;
        }
    }

    // --- Cliques ---

    /// Trata um clique da aba. `None` quando o id não é daqui — ou quando a
    /// regra de equipar recusou a jogada, que na v1 também não fazia nada.
    pub fn on_click(&mut self, clicked: Id, ctx: &Ctx) -> Option<Action> {
        // Com a lista aberta ela tem prioridade: o resto da tela está atrás dela.
        if let Some(slot) = self.open {
            if let Some(action) = self.on_dropdown_click(clicked, slot) {
                return Some(action);
            }
        }

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
        if clicked == generate_id() {
            self.generate(ctx);
            return Some(Action::Redraw);
        }
        if clicked == meta_generate_id() {
            self.generate_meta(ctx);
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
            self.set_search(String::new());
            return Some(Action::ClearEdit(search_id()));
        }
        if clicked == search_id() {
            return Some(Action::FocusEdit(search_id()));
        }
        if clicked == clear_search_id() {
            return Some(Action::ClearEdit(search_id()));
        }
        if clicked == name_id() {
            return Some(Action::FocusEdit(name_id()));
        }
        if clicked == save_id() {
            // Sem estratagema nenhum não há build para salvar — o botão fica
            // apagado, e o clique não faz nada (a v1 o desabilitava).
            let build = self.build.clone()?;
            if !builds::save(&mut self.loadouts, &self.name, &build) {
                return Some(Action::Redraw);
            }
            self.name.clear();
            return Some(Action::Saved);
        }

        for index in 0..self.loadouts.len() {
            if clicked == loadout_id(index) {
                let applied = builds::apply(
                    &self.loadouts[index],
                    self.build.as_ref(),
                    ctx.data,
                    data::equipment(),
                );
                self.build = Some(applied.build);
                return Some(Action::SlotsChanged(applied.slots));
            }
            if clicked == loadout_delete_id(index) {
                self.loadouts.remove(index);
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

    /// Sorteia uma build nova, preservando o que está travado.
    fn generate(&mut self, ctx: &Ctx) {
        let Some(equipment) = data::equipment() else {
            log::warn!("equipment.json indisponível: nada a sortear");
            return;
        };
        let meta = self
            .meta
            .get_or_insert_with(|| StratMeta::build(ctx.data, equipment));
        self.build = Some(builds::generate(
            self.build.as_ref(),
            &self.locks,
            Rules::from_settings(ctx.settings),
            ctx.data,
            equipment,
            meta,
            &mut rand::rng(),
        ));
    }

    /// Sorteia uma build a partir das estatísticas em tela.
    fn generate_meta(&mut self, ctx: &Ctx) {
        let Some(lists) = self.stats.lists() else {
            return;
        };
        let Some(equipment) = data::equipment() else {
            log::warn!("equipment.json indisponível: nada a sortear");
            return;
        };
        let kinds = self
            .meta
            .get_or_insert_with(|| StratMeta::build(ctx.data, equipment));
        self.build = Some(builds::generate_meta(
            self.build.as_ref(),
            &self.locks,
            Rules::from_settings(ctx.settings),
            lists,
            ctx.data,
            equipment,
            kinds,
            &mut rand::rng(),
        ));
    }

    /// Clique na grade personalizada. O slot em edição avança mesmo quando a
    /// regra recusa — é o que a v1 fazia, com o avanço fora do `setState`.
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

/// Card de equipamento já com os textos prontos — os `&str` do widget precisam
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

/// Só os formatos que o decodificador lê (R2: WebP e PNG). Os ícones de booster
/// e de passiva vêm da wiki em SVG e caem no marcador vazio do card.
fn raster_icon(path: &str) -> Option<String> {
    let lower = path.to_lowercase();
    (lower.ends_with(".webp") || lower.ends_with(".png")).then(|| path.to_string())
}

/// Cor da categoria, igual à da aba de macros.
fn tag_color(tag: &str) -> Color {
    match tag {
        "Offensive" => theme::RED,
        "Defensive" => theme::GREEN,
        _ => theme::CYAN,
    }
}

/// Botão de dificuldade: o escolhido acende em ciano, e não em amarelo — é o que
/// separa a linha da dificuldade da linha da facção na v1.
fn difficulty_button(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    let style = TextStyle::new(font::SIZE_TINY, Weight::Black)
        .tracking(font::TRACKING_LABEL)
        .align(Align::Center)
        .middle();

    if selected {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::CYAN.alpha(0.2));
        ui.stroke(rect, theme::RADIUS_BUTTON, 2.0, theme::CYAN.alpha(0.6));
        ui.text(rect, label.to_uppercase(), style, theme::CYAN);
    } else {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
        ui.stroke(
            rect,
            theme::RADIUS_BUTTON,
            2.0,
            theme::BORDER.mix(theme::CYAN.alpha(0.4), hover),
        );
        ui.text(
            rect,
            label.to_uppercase(),
            style,
            theme::TEXT_DIM.mix(theme::TEXT, hover),
        );
    }
    ui.hit(id, rect);
}

/// Recorta a próxima linha de uma lista, com o respiro entre linhas — e sem
/// sobra depois da última, que é o que as alturas calculadas assumem.
fn meta_row(cursor: &mut Rect, index: usize) -> Rect {
    if index > 0 {
        cursor.skip_top(META_ROW_GAP);
    }
    cursor.cut_top(META_ROW_HEIGHT)
}

/// Título de uma seção da coluna da direita, já avançando o cursor.
fn meta_section_label(ui: &mut Ui, cursor: &mut Rect, label: &str) {
    ui.text(
        cursor.cut_top(LABEL_HEIGHT),
        label.to_uppercase(),
        label_style(),
        theme::TEXT_DIM,
    );
    cursor.skip_top(LABEL_GAP);
}

/// Linha do top de estratagemas: ícone, nome, "NOVO", variação, barra e o
/// percentual (~678–691).
fn meta_stratagem_row(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    rect: Rect,
    strat: &Stratagem,
    stat: ItemStat,
    best: f64,
    ctx: &Ctx,
) {
    let mut row = rect;
    let icon = row.cut_left(META_ICON).middle_row(META_ICON);
    row.cut_left(META_CELL_GAP);
    ui.image_styled(
        icon,
        format!("icons/{}", strat.imagem),
        ImageStyle::FILL.contain(),
    );

    // As colunas de número são fixas; o nome fica com o que sobrar.
    let percent = row.cut_right(META_PERCENT_WIDTH);
    row.cut_right(META_CELL_GAP);
    let bar = row.cut_right(META_BAR_WIDTH).middle_row(META_BAR_HEIGHT);
    row.cut_right(META_CELL_GAP);
    let change = row.cut_right(META_CHANGE_WIDTH);
    row.cut_right(META_CELL_GAP);
    let badge = stat.is_new().then(|| {
        let badge = row.cut_right(META_NEW_WIDTH).middle_row(14.0);
        row.cut_right(META_CELL_GAP);
        badge
    });

    meta_name(ui, measure, row, &strat.nome);

    if let Some(badge) = badge {
        ui.fill(badge, 3.0, theme::YELLOW);
        ui.text(
            badge,
            ctx.tr().build.meta_new,
            TextStyle::new(7.0, Weight::Black)
                .align(Align::Center)
                .middle(),
            theme::TEXT_ON_ACCENT,
        );
    }

    let delta = stat.change();
    let (arrow, color) = match delta {
        delta if delta > 0.0 => ("\u{25B2}", theme::GREEN),
        delta if delta < 0.0 => ("\u{25BC}", theme::RED),
        _ => ("", theme::TEXT_DIM),
    };
    ui.text(
        change,
        format!("{arrow}{:.1}", delta.abs()),
        TextStyle::new(8.0, Weight::Black)
            .align(Align::End)
            .middle(),
        color,
    );

    // Barra proporcional ao primeiro colocado, e não a 100%.
    ui.fill(bar, META_BAR_HEIGHT / 2.0, theme::BORDER);
    let share = match best > 0.0 {
        true => (stat.loadouts_percentage / best).clamp(0.0, 1.0) as f32,
        false => 0.0,
    };
    if share > 0.0 {
        ui.fill(
            bar.with_w(bar.w * share),
            META_BAR_HEIGHT / 2.0,
            theme::CYAN,
        );
    }
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

fn meta_name(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, name: &str) {
    let style = TextStyle::new(font::SIZE_TINY, Weight::Black).middle();
    ui.text(
        rect,
        ellipsize(measure, &name.to_uppercase(), style, rect.w),
        style,
        theme::TEXT,
    );
}

fn meta_percent(ui: &mut Ui, rect: Rect, stat: ItemStat) {
    ui.text(
        rect,
        format!("{:.1}%", stat.loadouts_percentage),
        TextStyle::new(font::SIZE_TINY, Weight::Black)
            .align(Align::End)
            .middle(),
        theme::CYAN,
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

fn sub_tab(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    let style = TextStyle::new(font::SIZE_LABEL, Weight::Black)
        .tracking(font::TRACKING_WIDE)
        .align(Align::Center)
        .middle();

    if selected {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::YELLOW);
        ui.glow(rect, theme::RADIUS_BUTTON, theme::YELLOW);
        ui.text(rect, label.to_uppercase(), style, theme::TEXT_ON_ACCENT);
    } else {
        if hover > 0.0 {
            ui.fill(
                rect,
                theme::RADIUS_BUTTON,
                theme::SURFACE_HOVER.alpha(0.6 * hover),
            );
        }
        ui.text(
            rect,
            label.to_uppercase(),
            style,
            theme::TEXT_DIM.mix(theme::TEXT, hover),
        );
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

/// Título mais as linhas de uma seção da coluna da direita.
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
    widgets::card_chrome(false) + GENERATE_HEIGHT + LABEL_GAP + hint
}

fn saved_height(layout: &ChipLayout) -> f32 {
    widgets::card_chrome(true) + layout.height + widgets::CHIP_GAP + widgets::CONTROL_HEIGHT
}

fn equip_rows() -> f32 {
    data::EQUIP_SLOT_COUNT.div_ceil(EQUIP_COLS) as f32
}

fn cell_width(width: f32, cols: usize) -> f32 {
    ((width - GRID_GAP * (cols - 1) as f32) / cols as f32).max(0.0)
}

fn custom_cell(width: f32) -> f32 {
    cell_width(width, CUSTOM_GRID_COLS)
}

fn stratagems_height(measure: &mut dyn Measure, width: f32, build: &Build, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let cell = cell_width(inner, SLOT_COUNT);
    let tallest = (0..SLOT_COUNT)
        .map(|index| {
            let name = build.stratagems[index]
                .and_then(|id| ctx.data.by_id(id))
                .map(|strat| strat.nome.as_str())
                .unwrap_or("—");
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
        let loading = i18n::tr(settings.language)
            .build
            .meta_loading
            .to_uppercase();
        assert!(texts(&ui).contains(&loading));
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
        assert!(texts(&ui).contains(&error));
        assert!(!ui.frame().has_hit(meta_generate_id()));
        // E o botão, se clicado assim mesmo, não sorteia nada.
        tab.on_click(meta_generate_id(), &ctx(&data, &settings));
        assert!(tab.build.is_none());
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
        assert_eq!(
            tab.on_click(reset_id(), &context),
            Some(Action::ClearEdit(search_id()))
        );
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
            Some(Action::ClearEdit(search_id()))
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

    #[test]
    fn saved_builds_show_up_as_chips_and_apply_to_the_slots() {
        let data = data();
        let settings = Settings::default();
        let mut tab = with_loadouts(&data);
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        assert!(texts(&ui).iter().any(|text| text == "BUG SWEEP"));
        assert!(ui.frame().has_hit(loadout_id(0)));
        assert!(ui.frame().has_hit(loadout_id(1)));

        let expected: Slots = [Some(data.all()[1].id), Some(data.all()[2].id), None, None];
        assert_eq!(
            tab.on_click(loadout_id(1), &ctx(&data, &settings)),
            Some(Action::SlotsChanged(expected))
        );
        // A build aplicada também volta para a tela.
        assert_eq!(tab.build.as_ref().unwrap().stratagems, expected);
        // E o chip da build que bate com os slots é o destacado.
        assert_eq!(
            builds::active_loadout(tab.loadouts(), expected, &data),
            Some(1)
        );
    }

    #[test]
    fn the_delete_button_only_exists_under_the_mouse() {
        let data = data();
        let settings = Settings::default();
        let mut tab = with_loadouts(&data);
        let mut ui = Ui::new();
        // Janela alta: os chips ficam abaixo da dobra numa janela padrão, e o
        // hover não alcança o que está fora do recorte da página.
        let tall = Rect::new(0.0, 0.0, 820.0, 1_200.0);
        let build = |tab: &mut BuildTab, ui: &mut Ui, now: u64| {
            ui.begin(now);
            tab.build(ui, &mut Fixed, tall, &ctx(&data, &settings));
            ui.end();
        };
        build(&mut tab, &mut ui, 0);

        let chip = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(&node.visual, Visual::Text { text, .. } if text == "BUG SWEEP"))
            .expect("chip")
            .rect;
        assert!(!ui.frame().has_hit(loadout_delete_id(0)));

        // O fade de hover precisa de tempo para abrir.
        ui.input(Input::Move {
            x: chip.center_x(),
            y: chip.center_y(),
        });
        build(&mut tab, &mut ui, 0);
        build(&mut tab, &mut ui, 1_000);
        assert!(ui.frame().has_hit(loadout_delete_id(0)));

        assert_eq!(
            tab.on_click(loadout_delete_id(0), &ctx(&data, &settings)),
            Some(Action::LoadoutsChanged)
        );
        assert_eq!(tab.loadouts().len(), 1);
        assert_eq!(tab.loadouts()[0].name, "Bot Drop");
    }

    #[test]
    fn saving_needs_a_build_and_takes_the_typed_name() {
        let data = data();
        let settings = Settings::default();
        let mut tab = with_loadouts(&data);

        // Sem build na tela o botão não salva nada.
        assert_eq!(tab.on_click(save_id(), &ctx(&data, &settings)), None);
        assert_eq!(tab.loadouts().len(), 2);

        tab.on_click(generate_id(), &ctx(&data, &settings));
        tab.set_name("Minha Build".into());
        assert_eq!(
            tab.on_click(save_id(), &ctx(&data, &settings)),
            Some(Action::Saved)
        );
        assert_eq!(tab.loadouts().len(), 3);
        assert_eq!(tab.loadouts()[2].name, "Minha Build");
        assert!(tab.name.is_empty(), "o campo é esvaziado depois de salvar");

        // O mesmo nome sobrescreve em vez de duplicar.
        tab.on_click(generate_id(), &ctx(&data, &settings));
        tab.set_name("minha build".into());
        tab.on_click(save_id(), &ctx(&data, &settings));
        assert_eq!(tab.loadouts().len(), 3);
    }

    #[test]
    fn the_name_field_is_capped_like_the_legacy_input() {
        let mut tab = BuildTab::new();
        tab.set_name("x".repeat(40));
        assert_eq!(tab.name.chars().count(), NAME_MAX_CHARS);
    }

    #[test]
    fn an_empty_list_says_so_instead_of_showing_chips() {
        let data = data();
        let settings = Settings::default();
        let mut tab = BuildTab::new();
        tab.loaded = true;
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings));

        let empty = i18n::tr(settings.language).build.saved_empty.to_uppercase();
        assert!(texts(&ui).contains(&empty));
        assert!(!ui.frame().has_hit(loadout_id(0)));
        // O campo de nome continua lá, com a dica no lugar do filho nativo.
        assert!(ui.frame().has_hit(name_id()));
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
