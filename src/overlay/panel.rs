//! Conteúdo do painel do overlay: atribuir estratagemas aos slots e aplicar uma
//! build salva, tudo pelo mouse, sem sair do jogo.
//!
//! **Escopo enxuto (decisão do plano, §Fase 9):** só as duas seções acima.
//! Configurações e geração de builds continuam só na janela principal — o painel
//! existe para o que se faz no meio da partida.
//!
//! A janela é `WS_EX_NOACTIVATE`: recebe o mouse sem roubar o foco do jogo, mas
//! por isso mesmo nunca recebe o teclado. Por isso a grade vem sem campo de
//! busca ([`MacroTab::for_overlay`]) e as builds salvas só podem ser aplicadas —
//! nomear, salvar e excluir exigiriam digitação. A v1 tinha a mesma limitação.
//!
//! Como as abas da janela principal, o módulo é puro: entra estado, saem nós e
//! áreas clicáveis, e o teste roda no host.

use crate::builds;
use crate::data::GameData;
use crate::i18n::{self, Tr};
use crate::loadouts::{self, Loadout};
use crate::settings::Settings;
use crate::shared::Slots;
use crate::ui::macro_tab::{self, MacroTab};
use crate::ui::theme::{self, font, Color};
use crate::ui::toolkit::{id, id_at, Align, Id, Measure, Rect, TextStyle, Ui, Weight};
use crate::ui::widgets;

/// Folga entre a janela (840×660) e o vidro (820×640): é onde a sombra do
/// legado se espalhava.
const MARGIN: f32 = 10.0;
/// `rounded-3xl` do container.
const RADIUS: f32 = 24.0;
/// Faixa do título e do botão de fechar.
const HEADER_HEIGHT: f32 = 44.0;
/// Aviso de tela cheia exclusiva, quando o jogo está nesse modo.
const WARNING_HEIGHT: f32 = 52.0;
/// Rodapé com as builds salvas: rótulo + duas fileiras de chips.
const SAVED_HEIGHT: f32 = 108.0;
/// `px-6` das faixas fixas.
const PAGE_PADDING: f32 = 24.0;
const CLOSE_SIZE: f32 = 28.0;
const HOVER_MS: u32 = 180;

/// Id do × que fecha o painel.
fn close_id() -> Id {
    id("overlay.close")
}

/// Id do chip da build salva `index`.
fn chip_id(index: usize) -> Id {
    id_at("overlay.loadout", index)
}

/// O que a thread do overlay faz depois de um clique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Só redesenhar.
    Redraw,
    /// Fechar o painel (mesmo caminho do Ctrl+H).
    Close,
    /// Slots novos: gravar, refazer a tabela de atalhos e avisar a janela.
    SlotsChanged(Slots),
}

/// O que o painel precisa saber do resto do app.
pub struct Ctx<'a> {
    pub data: &'a GameData,
    pub settings: &'a Settings,
    pub slots: Slots,
}

impl Ctx<'_> {
    fn tr(&self) -> &'static Tr {
        i18n::tr(self.settings.language)
    }

    fn macro_ctx(&self) -> macro_tab::Ctx<'_> {
        macro_tab::Ctx {
            data: self.data,
            settings: self.settings,
            slots: self.slots,
            // Sem teclado na janela, nenhum campo de texto tem foco.
            focused_edit: None,
        }
    }
}

/// Estado do painel entre passagens de construção.
pub struct Panel {
    macro_tab: MacroTab,
    loadouts: Vec<Loadout>,
    warning: bool,
}

impl Default for Panel {
    fn default() -> Panel {
        Panel::new()
    }
}

impl Panel {
    pub fn new() -> Panel {
        Panel {
            macro_tab: MacroTab::for_overlay(),
            loadouts: Vec::new(),
            warning: false,
        }
    }

    /// Relê o `loadouts.json`. A janela principal é quem grava o arquivo e
    /// avisa por `OverlayCmd::LoadoutsChanged`.
    pub fn reload_loadouts(&mut self) {
        self.loadouts = loadouts::load_loadouts();
    }

    pub fn loadouts(&self) -> &[Loadout] {
        &self.loadouts
    }

    /// Jogo em "Tela Cheia" exclusiva: o aviso entra no topo do painel.
    pub fn set_warning(&mut self, warning: bool) {
        self.warning = warning;
    }

    // --- Construção ---

    pub fn build(&mut self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        let glass = area.inset(MARGIN);
        ui.fill(glass, RADIUS, theme::BG_DEEP.alpha(0.85));
        ui.stroke(
            glass,
            RADIUS,
            theme::HAIRLINE_WIDTH,
            Color::rgba(0xFFFFFF, 0.10),
        );

        let mut body = glass;
        let header = body.cut_top(HEADER_HEIGHT);
        self.header(ui, header, ctx);
        if self.warning {
            let banner = body.cut_top(WARNING_HEIGHT);
            warning(ui, banner, ctx.tr().overlay.fullscreen_warning);
        }
        let saved = body.cut_bottom(SAVED_HEIGHT);

        self.macro_tab.build(ui, measure, body, &ctx.macro_ctx());
        self.saved(ui, measure, saved, ctx);
    }

    /// Título à esquerda e o × à direita, com o filete do header embaixo.
    fn header(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let mut row = rect.inset_xy(PAGE_PADDING, 0.0);
        ui.fill(
            Rect::new(
                rect.x + RADIUS,
                rect.bottom() - theme::HAIRLINE_WIDTH,
                (rect.w - RADIUS * 2.0).max(0.0),
                theme::HAIRLINE_WIDTH,
            ),
            0.0,
            theme::HAIRLINE,
        );

        let close = row.cut_right(CLOSE_SIZE).middle_row(CLOSE_SIZE);
        ui.text(
            row.middle_row(16.0),
            ctx.tr().tabs.macro_tab.to_uppercase(),
            TextStyle::new(font::SIZE_LABEL, Weight::Black).tracking(font::TRACKING_WIDE),
            theme::CYAN,
        );

        // `text-slate-500 hover:text-red-500` do botão de fechar da v1.
        let id = close_id();
        let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
        if hover > 0.0 {
            ui.fill(close, 8.0, theme::SURFACE.alpha(0.6 * hover));
        }
        ui.text(
            close,
            "×",
            TextStyle::new(font::SIZE_TITLE, Weight::Black)
                .align(Align::Center)
                .middle(),
            theme::TEXT_DIM.mix(theme::RED, hover),
        );
        ui.hit(id, close);
    }

    /// Rodapé: os chips das builds salvas, só para aplicar.
    fn saved(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let rect = rect.inset_xy(PAGE_PADDING, 0.0);
        ui.fill(rect.with_h(theme::HAIRLINE_WIDTH), 0.0, theme::HAIRLINE);

        let mut area = rect;
        area.skip_top(12.0);
        let label = area.cut_top(14.0);
        ui.text(
            label,
            ctx.tr().build.saved.to_uppercase(),
            TextStyle::new(font::SIZE_TINY, Weight::Black).tracking(font::TRACKING_LABEL),
            theme::TEXT_DIM,
        );
        area.skip_top(8.0);

        if self.loadouts.is_empty() {
            ui.text(
                area.with_h(widgets::CHIP_HEIGHT),
                ctx.tr().build.saved_empty.to_uppercase(),
                widgets::chip_style().align(Align::Start),
                theme::TEXT_DIM,
            );
            return;
        }

        let layout = widgets::chip_layout(
            measure,
            self.loadouts.iter().map(|loadout| loadout.name.as_str()),
            area.w,
        );
        let active = builds::active_loadout(&self.loadouts, ctx.slots);
        // O rodapé tem altura fixa: o que não couber fica de fora (a lista longa
        // continua inteira na janela principal, que rola).
        ui.push_clip(area);
        for chip in &layout.chips {
            let rect = chip.rect(area);
            if rect.bottom() > area.bottom() {
                continue;
            }
            widgets::loadout_chip(
                ui,
                chip_id(chip.index),
                None,
                rect,
                &self.loadouts[chip.index].name,
                active == Some(chip.index),
            );
        }
        ui.pop_clip();
    }

    // --- Cliques ---

    /// Trata um clique. `None` quando o id não é de nada nesta tela.
    pub fn on_click(&mut self, clicked: Id, ctx: &Ctx) -> Option<Action> {
        if clicked == close_id() {
            return Some(Action::Close);
        }
        for index in 0..self.loadouts.len() {
            if clicked != chip_id(index) {
                continue;
            }
            // No painel a build salva só move os slots de macro: não há build
            // exibida nem equipamento aqui, então resolver os ids contra os
            // dados atuais é o trabalho inteiro (o mesmo que `builds::apply`
            // faria com a parte de estratagemas).
            let slots = loadouts::sanitize(&self.loadouts[index].slot_ids, ctx.data);
            return Some(Action::SlotsChanged(slots));
        }

        match self.macro_tab.on_click(clicked, &ctx.macro_ctx())? {
            macro_tab::Action::SlotsChanged(slots) => Some(Action::SlotsChanged(slots)),
            // Busca não existe no painel; o resto é só redesenhar.
            _ => Some(Action::Redraw),
        }
    }
}

/// Faixa amarela avisando que o modo de vídeo do jogo não convive com overlay.
fn warning(ui: &mut Ui, rect: Rect, message: &str) {
    let rect = rect.inset_xy(PAGE_PADDING, 4.0);
    ui.fill(rect, theme::RADIUS_BUTTON, theme::YELLOW.alpha(0.12));
    ui.stroke(
        rect,
        theme::RADIUS_BUTTON,
        theme::HAIRLINE_WIDTH,
        theme::YELLOW.alpha(0.5),
    );
    ui.text(
        rect.inset_xy(16.0, 8.0),
        message,
        TextStyle::new(font::SIZE_TINY, Weight::Regular)
            .align(Align::Center)
            .wrap(),
        theme::YELLOW,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::Visual;
    use std::collections::HashMap;

    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, max: f32) -> (f32, f32) {
            let width = text.chars().count() as f32 * style.size * 0.6;
            if style.wrap && max.is_finite() && width > max {
                (max, style.size * 1.3 * (width / max).ceil())
            } else {
                (width, style.size)
            }
        }
    }

    const AREA: Rect = Rect::new(
        0.0,
        0.0,
        super::super::PANEL_WIDTH,
        super::super::PANEL_HEIGHT,
    );

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    fn ctx<'a>(data: &'a GameData, settings: &'a Settings, slots: Slots) -> Ctx<'a> {
        Ctx {
            data,
            settings,
            slots,
        }
    }

    fn build_panel(panel: &mut Panel, ui: &mut Ui, ctx: &Ctx) {
        ui.begin(0);
        panel.build(ui, &mut Fixed, AREA, ctx);
        ui.end();
    }

    /// Builds salvas de mentira: o teste não pode depender do `loadouts.json`
    /// da máquina.
    fn with_loadouts(data: &GameData) -> Panel {
        let ids: Vec<u32> = data.all().iter().take(3).map(|strat| strat.id).collect();
        let mut panel = Panel::new();
        panel.loadouts = vec![
            Loadout {
                id: "1".into(),
                name: "Bug Sweep".into(),
                slot_ids: vec![Some(ids[0]), Some(ids[1]), None, None],
                equip: Some(HashMap::new()),
            },
            Loadout {
                id: "2".into(),
                name: "Bot Drop".into(),
                slot_ids: vec![Some(ids[2]), None, None, None],
                equip: None,
            },
        ];
        panel
    }

    #[test]
    fn the_panel_has_the_grid_the_slots_and_the_saved_builds() {
        let data = data();
        let settings = Settings::default();
        let mut panel = with_loadouts(&data);
        let mut ui = Ui::new();
        build_panel(
            &mut panel,
            &mut ui,
            &ctx(&data, &settings, Slots::default()),
        );

        assert!(ui.frame().has_hit(close_id()));
        assert!(ui.frame().has_hit(widgets::slot_id(0)));
        assert!(ui.frame().has_hit(chip_id(0)) && ui.frame().has_hit(chip_id(1)));
        assert!(
            ui.frame()
                .nodes
                .iter()
                .any(|node| matches!(&node.visual, Visual::Image { .. })),
            "a grade de estratagemas está na tela"
        );
    }

    #[test]
    fn there_is_no_text_field_in_the_panel() {
        // A janela nunca recebe teclado: um `EDIT` ali seria um campo morto.
        let data = data();
        let settings = Settings::default();
        let mut panel = with_loadouts(&data);
        let mut ui = Ui::new();
        build_panel(
            &mut panel,
            &mut ui,
            &ctx(&data, &settings, Slots::default()),
        );

        assert!(ui.frame().edits().is_empty());
        assert!(!ui.frame().has_hit(crate::ui::macro_tab::search_id()));
    }

    #[test]
    fn a_saved_build_only_moves_the_macro_slots() {
        let data = data();
        let settings = Settings::default();
        let mut panel = with_loadouts(&data);
        let expected = loadouts::sanitize(&panel.loadouts[0].slot_ids, &data);

        assert_eq!(
            panel.on_click(chip_id(0), &ctx(&data, &settings, Slots::default())),
            Some(Action::SlotsChanged(expected))
        );
    }

    #[test]
    fn the_close_button_asks_for_the_same_toggle_as_the_hotkey() {
        let data = data();
        let settings = Settings::default();
        let mut panel = Panel::new();
        assert_eq!(
            panel.on_click(close_id(), &ctx(&data, &settings, Slots::default())),
            Some(Action::Close)
        );
    }

    #[test]
    fn clicking_a_stratagem_assigns_it_to_the_active_slot() {
        let data = data();
        let settings = Settings::default();
        let mut panel = Panel::new();
        let mut ui = Ui::new();
        build_panel(
            &mut panel,
            &mut ui,
            &ctx(&data, &settings, Slots::default()),
        );

        // O primeiro card visível da grade responde como na janela principal.
        let strat = data
            .all()
            .iter()
            .find(|strat| ui.frame().has_hit(crate::ui::macro_tab::card_id(strat.id)))
            .expect("algum card na primeira tela");
        let action = panel.on_click(
            crate::ui::macro_tab::card_id(strat.id),
            &ctx(&data, &settings, Slots::default()),
        );
        assert_eq!(
            action,
            Some(Action::SlotsChanged([Some(strat.id), None, None, None]))
        );
    }

    #[test]
    fn the_fullscreen_warning_only_shows_in_that_video_mode() {
        let data = data();
        let settings = Settings::default();
        let mut panel = Panel::new();
        let mut ui = Ui::new();

        let warned = |ui: &Ui| {
            ui.frame().nodes.iter().any(|node| {
                matches!(
                    &node.visual,
                    Visual::Text { text, .. } if text.contains("Tela Cheia")
                )
            })
        };

        build_panel(
            &mut panel,
            &mut ui,
            &ctx(&data, &settings, Slots::default()),
        );
        assert!(!warned(&ui));

        panel.set_warning(true);
        build_panel(
            &mut panel,
            &mut ui,
            &ctx(&data, &settings, Slots::default()),
        );
        assert!(warned(&ui));
    }

    #[test]
    fn an_unknown_click_is_not_ours() {
        let data = data();
        let settings = Settings::default();
        let mut panel = Panel::new();
        assert_eq!(
            panel.on_click(id("outra.tela"), &ctx(&data, &settings, Slots::default())),
            None
        );
    }
}
