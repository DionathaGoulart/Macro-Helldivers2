//! Aba de macros: a grade de estratagemas por categoria, a busca e a barra dos
//! quatro slots.
//!
//! A tela é uma função do estado — settings, slots e o texto da busca entram,
//! nós e áreas clicáveis saem. Quem clica devolve uma [`Action`] para a janela
//! executar (gravar, refazer a tabela de atalhos, avisar o overlay); assim as
//! regras de equipar, que são o coração da aba, ficam testáveis no host.
//!
//! Comportamento portado de `legacy/src/renderer/App.jsx` (~460–535 e ~900–935).

use crate::data::{self, GameData, Stratagem};
use crate::i18n::{self, Tr};
use crate::settings::{Settings, SLOT_COUNT};
use crate::shared::Slots;
use crate::ui::theme::{self, font, Color};
use crate::ui::toolkit::{
    grid_cell, grid_height, id, id_at, Align, Id, Measure, Rect, TextStyle, Ui, Weight,
};
use crate::ui::widgets::{self, CardState};

/// `px-6` da coluna de conteúdo.
const PAGE_PADDING: f32 = 24.0;
const TITLE_HEIGHT: f32 = 24.0;
/// Folga dos dois lados do título, entre o texto e os filetes.
const TITLE_GAP: f32 = 16.0;
const SEARCH_GAP: f32 = 16.0;
/// `p-5` das seções.
const SECTION_PADDING: f32 = 20.0;
const SECTION_HEADER: f32 = 16.0;
/// `mb-5` entre o título da seção e a grade.
const SECTION_HEADER_GAP: f32 = 20.0;
/// `space-y-6` entre seções.
const SECTION_GAP: f32 = 24.0;
const GRID_COLS: usize = 4;
/// `gap-3` da grade.
const GRID_GAP: f32 = 12.0;
/// Espaço à direita reservado para a barra de rolagem, para ela não cair em
/// cima dos cards da última coluna.
const SCROLL_GUTTER: f32 = 12.0;
/// `p-4` e `gap-4` da barra de slots.
const SLOT_BAR_PADDING: f32 = 16.0;
const SLOT_BAR_GAP: f32 = 16.0;
/// `rounded-3xl` da barra.
const SLOT_BAR_RADIUS: f32 = 24.0;
const CLEAR_SIZE: f32 = 20.0;
const HOVER_MS: u32 = 180;

/// Ordem fixa das categorias; o que não estiver aqui vai depois, em ordem
/// alfabética (`sortedTags` do legado).
const TAG_ORDER: [&str; 3] = ["Offensive", "Supply", "Defensive"];

/// Id do campo de busca. A janela precisa dele para reconhecer o `EDIT` nativo.
pub fn search_id() -> Id {
    id("macro.search")
}

fn clear_search_id() -> Id {
    id("macro.search.clear")
}

fn grid_id() -> Id {
    id("macro.grid")
}

/// Id do card de um estratagema. Vem do id do estratagema, não da posição: o
/// hover não pula de card quando a busca reordena a grade.
pub fn card_id(stratagem: u32) -> Id {
    id_at("macro.card", stratagem as usize)
}

/// Altura da barra de slots, que fica presa no rodapé da aba.
pub fn slot_bar_height() -> f32 {
    widgets::SLOT_SIZE + SLOT_BAR_PADDING * 2.0
}

/// O que a aba precisa saber do resto do app para se desenhar.
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

/// O que a janela faz depois de um clique na aba.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Só reconstruir a tela.
    Redraw,
    /// Slots novos: gravar em disco, refazer a tabela de atalhos e avisar o
    /// overlay.
    SlotsChanged(Slots),
    /// Trazer o `EDIT` da busca para a frente.
    FocusSearch,
    /// Esvaziar o `EDIT` da busca.
    ClearSearch,
}

/// Uma categoria da grade, já filtrada pela busca.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Section {
    /// Primeira tag do estratagema; vazia quando ele não tem nenhuma.
    tag: String,
    ids: Vec<u32>,
}

/// Estado da aba entre passagens de construção.
pub struct MacroTab {
    active_slot: usize,
    search: String,
    sections: Vec<Section>,
    /// A busca mudou (ou nunca foi agrupada): refazer as seções na próxima
    /// construção, e não a cada quadro.
    dirty: bool,
    /// Campo de busca na tela. O painel do overlay não o tem: a janela é
    /// `WS_EX_NOACTIVATE` — ela recebe o mouse mas nunca o teclado —, e a v1
    /// escondia o campo lá pelo mesmo motivo (`{!isOverlay && ...}`).
    searchable: bool,
}

impl Default for MacroTab {
    fn default() -> MacroTab {
        MacroTab::new()
    }
}

impl MacroTab {
    pub fn new() -> MacroTab {
        MacroTab {
            active_slot: 0,
            search: String::new(),
            sections: Vec::new(),
            dirty: true,
            searchable: true,
        }
    }

    /// A mesma aba sem o campo de busca, para o painel do overlay.
    pub fn for_overlay() -> MacroTab {
        MacroTab {
            searchable: false,
            ..MacroTab::new()
        }
    }

    /// Slot que os cliques na grade equipam.
    pub fn active_slot(&self) -> usize {
        self.active_slot
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

    /// Agrupa por categoria e aplica a busca. Ordem dentro da seção é a do
    /// arquivo, como no legado.
    fn regroup(&mut self, data: &GameData) {
        let needle = data::normalize_text(self.search.trim());
        let mut sections: Vec<Section> = Vec::new();

        for strat in data.all() {
            if !needle.is_empty() && !data::normalize_text(&strat.nome).contains(&needle) {
                continue;
            }
            let tag = strat.primary_tag().unwrap_or_default();
            match sections.iter_mut().find(|section| section.tag == tag) {
                Some(section) => section.ids.push(strat.id),
                None => sections.push(Section {
                    tag: tag.to_string(),
                    ids: vec![strat.id],
                }),
            }
        }

        sections.sort_by(|a, b| {
            tag_rank(&a.tag)
                .cmp(&tag_rank(&b.tag))
                .then(a.tag.cmp(&b.tag))
        });
        self.sections = sections;
        self.dirty = false;
    }

    // --- Construção ---

    pub fn build(&mut self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        if self.dirty {
            self.regroup(ctx.data);
        }

        let mut area = area.inset_xy(PAGE_PADDING, 16.0);
        let bar = area.cut_bottom(slot_bar_height());
        area.cut_bottom(SEARCH_GAP);
        let title = area.cut_top(TITLE_HEIGHT);
        let search = self.searchable.then(|| {
            area.skip_top(8.0);
            area.cut_top(widgets::CONTROL_HEIGHT)
        });
        area.skip_top(SEARCH_GAP);

        self.title(ui, measure, title, ctx);
        if let Some(search) = search {
            self.search_field(ui, search, ctx);
        }
        if self.sections.is_empty() {
            self.no_results(ui, area, ctx);
        } else {
            self.grid(ui, area, ctx);
        }
        self.slot_bar(ui, measure, bar, ctx);
    }

    /// "SELECIONAR ESTRATAGEMAS PARA O SLOT F1", entre dois filetes.
    fn title(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let shortcut = ctx.settings.shortcut(self.active_slot).unwrap_or("—");
        let label = format!("{} {}", ctx.tr().macros.select_title, shortcut).to_uppercase();
        let style = TextStyle::new(font::SIZE_LABEL, Weight::Black)
            .tracking(font::TRACKING_LABEL)
            .align(Align::Center)
            .middle();

        let width = measure.text_size(&label, style, f32::INFINITY).0;
        ui.text(rect, label, style, theme::TEXT_DIM);

        let line = theme::BORDER.alpha(0.6);
        let y = rect.center_y();
        let left = Rect::new(
            rect.x,
            y,
            (rect.w - width) / 2.0 - TITLE_GAP,
            theme::HAIRLINE_WIDTH,
        );
        ui.fill(left, 0.0, line);
        ui.fill(
            Rect::new(rect.right() - left.w, y, left.w, theme::HAIRLINE_WIDTH),
            0.0,
            line,
        );
    }

    fn search_field(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let focused = ctx.focused_edit == Some(search_id());
        let placeholder =
            (self.search.is_empty() && !focused).then_some(ctx.tr().macros.search_placeholder);
        widgets::edit_host(ui, search_id(), rect, focused, placeholder);

        if self.search.is_empty() {
            return;
        }
        // × dentro do campo, do jeito que a v1 o colocava.
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

    fn no_results(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        ui.text(
            rect.with_h(48.0),
            format!(
                "{} \u{201c}{}\u{201d}",
                ctx.tr().macros.search_no_results,
                self.search.trim()
            ),
            TextStyle::new(font::SIZE_LABEL, Weight::Black)
                .tracking(font::TRACKING_LABEL)
                .align(Align::Center)
                .middle(),
            theme::TEXT_DIM,
        );
    }

    /// As seções, dentro do container rolável.
    fn grid(&self, ui: &mut Ui, view: Rect, ctx: &Ctx) {
        let width = view.w - SCROLL_GUTTER;
        let cell = cell_size(width);
        let content: f32 = self
            .sections
            .iter()
            .map(|section| section_height(section.ids.len(), cell) + SECTION_GAP)
            .sum::<f32>()
            - SECTION_GAP;

        let equipped = ctx.data.resolve(&ctx.slots);
        let offset = ui.scroll_begin(grid_id(), view);
        let mut y = view.y - offset;
        for section in &self.sections {
            let height = section_height(section.ids.len(), cell);
            let rect = Rect::new(view.x, y, width, height);
            // Fora da janela visível não há o que desenhar — é o que segura o
            // custo de uma grade de 91 ícones.
            if rect.bottom() >= view.y && rect.y <= view.bottom() {
                self.section(ui, rect, section, cell, view, &equipped, ctx);
            }
            y += height + SECTION_GAP;
        }
        ui.scroll_end(grid_id(), view, content.max(0.0));
    }

    #[allow(clippy::too_many_arguments)]
    fn section(
        &self,
        ui: &mut Ui,
        rect: Rect,
        section: &Section,
        cell: f32,
        view: Rect,
        equipped: &[Option<&Stratagem>],
        ctx: &Ctx,
    ) {
        let accent = tag_color(&section.tag);
        ui.fill(rect, theme::RADIUS_CARD, theme::CARD_BG);
        ui.stroke(
            rect,
            theme::RADIUS_CARD,
            theme::HAIRLINE_WIDTH,
            theme::BORDER,
        );
        widgets::card_left_accent(ui, rect, accent.alpha(0.5));

        let mut content = rect.inset(SECTION_PADDING);
        let mut header = content.cut_top(SECTION_HEADER);
        let dot = header.cut_left(20.0).middle_row(8.0).with_w(8.0);
        ui.ellipse(dot, accent);
        ui.glow(dot, 4.0, accent);
        ui.text(
            header,
            ctx.tr()
                .settings
                .tag(&section.tag, ctx.tr().macros.others)
                .to_uppercase(),
            TextStyle::new(font::SIZE_BODY, Weight::Black)
                .tracking(font::TRACKING_LABEL)
                .middle(),
            theme::TEXT_DIM,
        );
        content.skip_top(SECTION_HEADER_GAP);

        for (index, id) in section.ids.iter().enumerate() {
            let cell_rect = grid_cell(content, GRID_COLS, cell, GRID_GAP, index);
            if cell_rect.bottom() < view.y || cell_rect.y > view.bottom() {
                continue;
            }
            let Some(strat) = ctx.data.by_id(*id) else {
                continue;
            };
            widgets::stratagem_card(
                ui,
                card_id(*id),
                cell_rect,
                strat,
                CardState {
                    disabled: is_disabled(strat, equipped, self.active_slot),
                    in_active_slot: ctx.slots[self.active_slot] == Some(*id),
                    accent,
                },
            );
        }
    }

    /// Barra dos quatro slots, centralizada no rodapé da aba.
    fn slot_bar(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let width = widgets::SLOT_SIZE * SLOT_COUNT as f32
            + SLOT_BAR_GAP * (SLOT_COUNT - 1) as f32
            + SLOT_BAR_PADDING * 2.0;
        let panel = rect.centered(width, slot_bar_height());
        ui.fill(panel, SLOT_BAR_RADIUS, theme::SURFACE);
        ui.stroke(
            panel,
            SLOT_BAR_RADIUS,
            theme::HAIRLINE_WIDTH,
            theme::HAIRLINE,
        );

        let equipped = ctx.data.resolve(&ctx.slots);
        let mut x = panel.x + SLOT_BAR_PADDING;
        for (index, strat) in equipped.iter().enumerate() {
            let fallback = format!("F{}", index + 1);
            let shortcut = ctx.settings.shortcut(index).unwrap_or(&fallback);
            widgets::slot_square(
                ui,
                measure,
                index,
                Rect::new(
                    x,
                    panel.y + SLOT_BAR_PADDING,
                    widgets::SLOT_SIZE,
                    widgets::SLOT_SIZE,
                ),
                *strat,
                shortcut,
                index == self.active_slot,
            );
            x += widgets::SLOT_SIZE + SLOT_BAR_GAP;
        }
    }

    // --- Cliques ---

    /// Trata um clique da aba. `None` quando o id não é daqui — ou quando a
    /// regra de equipar recusou a jogada, que na v1 também não fazia nada.
    pub fn on_click(&mut self, clicked: Id, ctx: &Ctx) -> Option<Action> {
        if clicked == search_id() {
            return Some(Action::FocusSearch);
        }
        if clicked == clear_search_id() {
            return Some(Action::ClearSearch);
        }
        for index in 0..SLOT_COUNT {
            if clicked == widgets::slot_id(index) {
                self.active_slot = index;
                return Some(Action::Redraw);
            }
            if clicked == widgets::slot_clear_id(index) {
                let mut slots = ctx.slots;
                slots[index] = None;
                return Some(Action::SlotsChanged(slots));
            }
        }

        let strat = ctx
            .data
            .all()
            .iter()
            .find(|strat| card_id(strat.id) == clicked)?;
        self.assign(strat, ctx)
    }

    /// Regras de equipar (`handleAssignStratagem` da v1).
    fn assign(&mut self, strat: &Stratagem, ctx: &Ctx) -> Option<Action> {
        let active = self.active_slot;
        let mut slots = ctx.slots;

        // Clicar no estratagema que já está no slot em edição o remove.
        if slots[active] == Some(strat.id) {
            slots[active] = None;
            return Some(Action::SlotsChanged(slots));
        }
        if slots.contains(&Some(strat.id)) {
            return None;
        }
        let equipped = ctx.data.resolve(&slots);
        if data::has_exclusive_conflict(strat, &equipped, active) {
            return None;
        }

        slots[active] = Some(strat.id);
        // Equipar anda para o próximo slot, mas para no último.
        if active + 1 < SLOT_COUNT {
            self.active_slot = active + 1;
        }
        Some(Action::SlotsChanged(slots))
    }
}

/// Posição da categoria na ordem fixa; o resto vai depois, em ordem alfabética.
fn tag_rank(tag: &str) -> usize {
    TAG_ORDER
        .iter()
        .position(|known| *known == tag)
        .unwrap_or(TAG_ORDER.len())
}

/// Cor da categoria, igual à da v1: ofensivo vermelho, defensivo verde e o
/// resto ciano.
fn tag_color(tag: &str) -> Color {
    match tag {
        "Offensive" => theme::RED,
        "Defensive" => theme::GREEN,
        _ => theme::CYAN,
    }
}

/// Lado do card quadrado numa seção de `width` de largura.
fn cell_size(width: f32) -> f32 {
    let inner = width - SECTION_PADDING * 2.0;
    ((inner - GRID_GAP * (GRID_COLS - 1) as f32) / GRID_COLS as f32).max(0.0)
}

fn section_height(count: usize, cell: f32) -> f32 {
    SECTION_PADDING * 2.0
        + SECTION_HEADER
        + SECTION_HEADER_GAP
        + grid_height(count, GRID_COLS, cell, GRID_GAP)
}

/// Um card só é clicável se couber no slot em edição (`isCardDisabled` da v1).
fn is_disabled(strat: &Stratagem, equipped: &[Option<&Stratagem>], active: usize) -> bool {
    if equipped[active].is_some_and(|current| current.id == strat.id) {
        return false;
    }
    if equipped
        .iter()
        .any(|slot| slot.is_some_and(|other| other.id == strat.id))
    {
        return true;
    }
    data::has_exclusive_conflict(strat, equipped, active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::Visual;

    /// Medidor de largura fixa: o layout só precisa de uma largura plausível.
    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, _max: f32) -> (f32, f32) {
            (text.chars().count() as f32 * style.size * 0.6, style.size)
        }
    }

    const AREA: Rect = Rect::new(0.0, 0.0, 820.0, 520.0);

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    fn ctx<'a>(data: &'a GameData, settings: &'a Settings, slots: Slots) -> Ctx<'a> {
        Ctx {
            data,
            settings,
            slots,
            focused_edit: None,
        }
    }

    fn build(tab: &mut MacroTab, ui: &mut Ui, ctx: &Ctx) {
        ui.begin(0);
        tab.build(ui, &mut Fixed, AREA, ctx);
        ui.end();
    }

    #[test]
    fn sections_follow_the_categories_in_the_legacy_order() {
        let data = data();
        let mut tab = MacroTab::new();
        tab.regroup(&data);

        let tags: Vec<&str> = tab.sections.iter().map(|s| s.tag.as_str()).collect();
        assert_eq!(tags, ["Offensive", "Supply", "Defensive"]);
        assert_eq!(
            tab.sections.iter().map(|s| s.ids.len()).sum::<usize>(),
            data.all().len(),
            "todo estratagema cai numa seção"
        );
    }

    #[test]
    fn the_search_ignores_case_and_accents_and_drops_empty_sections() {
        let data = data();
        let mut tab = MacroTab::new();

        // A busca normaliza os dois lados (`normalizeText` da v1), então caixa
        // e acento não interferem.
        tab.set_search("ORBITAL".to_string());
        tab.regroup(&data);
        assert!(!tab.sections.is_empty());
        assert!(tab.sections.iter().all(|section| section
            .ids
            .iter()
            .all(|id| data::normalize_text(&data.by_id(*id).unwrap().nome).contains("orbital"))));
        assert!(
            tab.sections.len() < 3,
            "categoria sem resultado some da tela"
        );

        tab.set_search("   ".to_string());
        tab.regroup(&data);
        assert_eq!(tab.sections.len(), 3, "busca em branco mostra tudo");
    }

    #[test]
    fn a_search_without_matches_shows_the_message_instead_of_the_grid() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        let mut ui = Ui::new();

        tab.set_search("zzzz".to_string());
        build(&mut tab, &mut ui, &ctx(&data, &settings, Slots::default()));

        assert!(tab.sections.is_empty());
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            &node.visual,
            Visual::Text { text, .. } if text.contains("zzzz")
        )));
        // A barra de slots continua lá, e a grade não.
        assert_eq!(ui.frame().hit_at(350.0, 450.0), Some(widgets::slot_id(1)));
    }

    #[test]
    fn clicking_a_stratagem_fills_the_active_slot_and_moves_on() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        let strat = &data.all()[0];

        let action = tab.on_click(card_id(strat.id), &ctx(&data, &settings, Slots::default()));
        assert_eq!(
            action,
            Some(Action::SlotsChanged([Some(strat.id), None, None, None]))
        );
        assert_eq!(tab.active_slot(), 1, "o slot em edição avança");

        // No último slot ele para em vez de dar a volta.
        tab.active_slot = SLOT_COUNT - 1;
        let slots = [Some(strat.id), None, None, None];
        let other = &data.all()[1];
        tab.on_click(card_id(other.id), &ctx(&data, &settings, slots));
        assert_eq!(tab.active_slot(), SLOT_COUNT - 1);
    }

    #[test]
    fn clicking_the_stratagem_of_the_active_slot_clears_it() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        let strat = &data.all()[0];
        let slots = [Some(strat.id), None, None, None];

        assert_eq!(
            tab.on_click(card_id(strat.id), &ctx(&data, &settings, slots)),
            Some(Action::SlotsChanged([None, None, None, None]))
        );
        assert_eq!(tab.active_slot(), 0, "remover não avança o slot");
    }

    #[test]
    fn a_stratagem_equipped_elsewhere_or_in_conflict_is_refused() {
        let data = data();
        let settings = Settings::default();
        let mechas: Vec<&Stratagem> = data
            .all()
            .iter()
            .filter(|strat| strat.has_tag("Mecha"))
            .collect();
        let strat = &data.all()[0];

        // Já equipado noutro slot.
        let mut tab = MacroTab::new();
        tab.active_slot = 1;
        let slots = [Some(strat.id), None, None, None];
        assert_eq!(
            tab.on_click(card_id(strat.id), &ctx(&data, &settings, slots)),
            None
        );

        // Dois exos não cabem na mesma build.
        let mut tab = MacroTab::new();
        tab.active_slot = 1;
        let slots = [Some(mechas[0].id), None, None, None];
        assert_eq!(
            tab.on_click(card_id(mechas[1].id), &ctx(&data, &settings, slots)),
            None
        );
        // Mas trocar o próprio exo pelo outro é permitido.
        let mut tab = MacroTab::new();
        let slots = [Some(mechas[0].id), None, None, None];
        assert!(matches!(
            tab.on_click(card_id(mechas[1].id), &ctx(&data, &settings, slots)),
            Some(Action::SlotsChanged(_))
        ));
    }

    #[test]
    fn cards_that_cannot_be_equipped_do_not_answer_the_mouse() {
        let data = data();
        let equipped_elsewhere = &data.all()[0];
        let mechas: Vec<&Stratagem> = data
            .all()
            .iter()
            .filter(|strat| strat.has_tag("Mecha"))
            .collect();
        let slots = [Some(equipped_elsewhere.id), Some(mechas[0].id), None, None];
        let resolved = data.resolve(&slots);

        assert!(is_disabled(equipped_elsewhere, &resolved, 2));
        assert!(is_disabled(mechas[1], &resolved, 2));
        assert!(
            !is_disabled(equipped_elsewhere, &resolved, 0),
            "o próprio estratagema do slot em edição continua clicável (para remover)"
        );
        assert!(
            !is_disabled(mechas[1], &resolved, 1),
            "trocar o exo pelo outro no mesmo slot não é conflito"
        );
    }

    #[test]
    fn the_slot_bar_selects_and_clears() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        let strat = &data.all()[0];
        let slots = [None, None, Some(strat.id), None];

        assert_eq!(
            tab.on_click(widgets::slot_id(2), &ctx(&data, &settings, slots)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.active_slot(), 2);

        assert_eq!(
            tab.on_click(widgets::slot_clear_id(2), &ctx(&data, &settings, slots)),
            Some(Action::SlotsChanged([None, None, None, None]))
        );
    }

    #[test]
    fn the_search_field_answers_with_focus_and_clear() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        let mut ui = Ui::new();

        tab.set_search("orbital".to_string());
        build(&mut tab, &mut ui, &ctx(&data, &settings, Slots::default()));

        let context = ctx(&data, &settings, Slots::default());
        assert_eq!(
            tab.on_click(search_id(), &context),
            Some(Action::FocusSearch)
        );
        assert_eq!(
            tab.on_click(clear_search_id(), &context),
            Some(Action::ClearSearch)
        );
        // O × só existe com texto na busca; sem ele, o campo responde inteiro.
        let inside_clear = (770.0, 68.0);
        assert_eq!(
            ui.frame().hit_at(inside_clear.0, inside_clear.1),
            Some(clear_search_id())
        );

        tab.set_search(String::new());
        build(&mut tab, &mut ui, &ctx(&data, &settings, Slots::default()));
        assert_eq!(
            ui.frame().hit_at(inside_clear.0, inside_clear.1),
            Some(search_id())
        );
    }

    #[test]
    fn only_the_visible_part_of_the_grid_is_built() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings, Slots::default()));

        let cards = data
            .all()
            .iter()
            .filter(|strat| ui.frame().has_hit(card_id(strat.id)))
            .count();
        assert!(cards > 0, "a primeira tela tem cards");
        assert!(
            cards < data.all().len(),
            "os 91 estratagemas não são desenhados de uma vez"
        );
    }

    #[test]
    fn the_overlay_variant_drops_the_search_field() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::for_overlay();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &ctx(&data, &settings, Slots::default()));

        assert!(
            ui.frame().edits().is_empty(),
            "o painel do overlay não recebe teclado: nada de EDIT nativo"
        );
        assert!(!ui.frame().has_hit(search_id()));
        // O resto da aba continua inteiro.
        assert!(ui.frame().has_hit(widgets::slot_id(0)));
        assert!(data
            .all()
            .iter()
            .any(|strat| ui.frame().has_hit(card_id(strat.id))));
    }

    #[test]
    fn an_unknown_click_is_not_ours() {
        let data = data();
        let settings = Settings::default();
        let mut tab = MacroTab::new();
        assert_eq!(
            tab.on_click(id("outra.tela"), &ctx(&data, &settings, Slots::default())),
            None
        );
    }
}
