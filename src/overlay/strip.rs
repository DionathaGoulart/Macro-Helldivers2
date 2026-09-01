//! Conteúdo do strip: os quatro slots de macro desenhados por cima do jogo.
//!
//! É a barra de slots da janela principal na escala que a v1 usava no estado
//! minimal (`scale-[0.70] origin-bottom`, `legacy/src/renderer/App.jsx` ~904).
//! Como a janela é click-through, aqui não há hover, botão de limpar nem slot em
//! edição: sobra o que o jogador precisa ver de relance — ícone, atalho e as
//! piscadas de disparo e bloqueio.
//!
//! O módulo é puro (só empurra nós num [`Ui`]), então roda e é testado no host.

use crate::data::{GameData, Stratagem};
use crate::settings::Settings;
use crate::shared::{FlashKind, Slots};
use crate::ui::theme::{self, font};
use crate::ui::toolkit::{Align, ImageStyle, Measure, Rect, TextStyle, Ui, Weight};
use crate::ui::widgets;

/// Escala do strip (`scale-[0.70]` da v1).
pub const SCALE: f32 = 0.70;
/// `bottom-2`: folga entre a barra e a base da tela.
const BOTTOM_MARGIN: f32 = 8.0;
/// `p-4` e `gap-4` da barra, antes da escala.
const PADDING: f32 = 16.0;
const GAP: f32 = 16.0;
/// `rounded-3xl`.
const RADIUS: f32 = 24.0;
/// Abaixo disto o texto do atalho deixa de ser legível de relance; a escala para
/// aqui, e só para ele.
const MIN_TAG_TEXT: f32 = 7.0;

pub struct Ctx<'a> {
    pub data: &'a GameData,
    pub settings: &'a Settings,
    pub slots: Slots,
}

/// Lado do quadrado de um slot no strip.
fn slot_size() -> f32 {
    widgets::SLOT_SIZE * SCALE
}

/// Tamanho da barra desenhada, em DIP. A janela é maior que isso: sobra a folga
/// do brilho e da piscada, que passam da borda da barra.
pub fn size(slots: usize) -> (f32, f32) {
    let (slot, gap, padding) = (slot_size(), GAP * SCALE, PADDING * SCALE);
    (
        slot * slots as f32 + gap * (slots.saturating_sub(1)) as f32 + padding * 2.0,
        slot + padding * 2.0,
    )
}

/// Desenha a barra centralizada na horizontal e presa embaixo de `area`.
pub fn build(ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
    let equipped = ctx.data.resolve(&ctx.slots);
    let (width, height) = size(equipped.len());
    let bar = Rect::new(
        area.center_x() - width / 2.0,
        area.bottom() - BOTTOM_MARGIN - height,
        width,
        height,
    );

    // A v1 escurecia o fundo com `bg-slate-950/20` mais um `backdrop-blur-sm`.
    // Sem desfoque de fundo (custaria uma camada de composição por quadro sobre
    // o jogo), a mesma leitura vem de um véu um pouco mais opaco.
    ui.fill(bar, RADIUS * SCALE, theme::BG_DEEP.alpha(0.55));
    ui.stroke(
        bar,
        RADIUS * SCALE,
        theme::HAIRLINE_WIDTH,
        theme::HAIRLINE.alpha(0.4),
    );

    let (slot, gap, padding) = (slot_size(), GAP * SCALE, PADDING * SCALE);
    let mut x = bar.x + padding;
    for (index, strat) in equipped.iter().enumerate() {
        let fallback = format!("F{}", index + 1);
        let shortcut = ctx.settings.shortcut(index).unwrap_or(&fallback);
        square(
            ui,
            measure,
            index,
            Rect::new(x, bar.y + padding, slot, slot),
            *strat,
            shortcut,
        );
        x += slot + gap;
    }
}

/// Um slot do strip. Mesma leitura do `slot_square` da janela principal, com as
/// medidas proporcionais ao quadrado — na escala do strip, as constantes em DIP
/// da versão grande sairiam desproporcionais.
fn square(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    index: usize,
    rect: Rect,
    strat: Option<&Stratagem>,
    shortcut: &str,
) {
    let triggered = ui.anim(
        widgets::flash_id(index, false, FlashKind::Triggered),
        widgets::FLASH_TRIGGERED_MS,
    );
    let blocked = ui.anim(
        widgets::flash_id(index, false, FlashKind::Blocked),
        widgets::FLASH_BLOCKED_MS,
    );
    let radius = theme::RADIUS_SLOT * SCALE;

    // Bloqueio: mancha vermelha em volta do quadrado (`-inset-1` da v1).
    if blocked > 0.0 {
        let halo = 4.0 * SCALE;
        ui.fill(
            rect.inset(-halo),
            radius + halo,
            theme::RED.alpha(0.3 * blocked),
        );
    }

    let background = if triggered > 0.0 {
        theme::YELLOW
            .alpha(0.4 * triggered)
            .over(theme::SURFACE_ACTIVE)
    } else {
        theme::CARD_BG
    };
    ui.fill(rect, radius, background);

    match strat {
        Some(strat) => {
            ui.image_styled(
                rect,
                format!("icons/{}", strat.imagem),
                ImageStyle::FILL
                    .opacity(0.8)
                    .rounded(radius)
                    .zoom(1.0 + 0.10 * triggered),
            );
            // Escurece o topo para o atalho continuar legível sobre o ícone.
            ui.gradient(
                rect.with_h(rect.h * 0.6),
                0.0,
                theme::BG_DEEP.alpha(0.6),
                theme::BG_DEEP.alpha(0.0),
            );
        }
        None => {
            // Slot vazio: só a marca tracejada da v1 — o "OPEN" não sobreviveria
            // à escala.
            ui.stroke(
                rect.centered(rect.w * 0.32, rect.h * 0.32),
                radius / 2.0,
                theme::HAIRLINE_WIDTH * 2.0,
                theme::TEXT_DIM.alpha(0.2),
            );
        }
    }

    tag(ui, measure, rect, shortcut);

    let border = if triggered > 0.0 {
        theme::YELLOW
    } else {
        theme::BORDER.alpha(0.8)
    };
    ui.stroke(rect, radius, 2.0 * SCALE, border);
    if triggered > 0.0 {
        ui.glow(rect, radius, theme::YELLOW.alpha(triggered));
    }
}

/// Etiqueta do atalho, presa no topo do quadrado.
fn tag(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, shortcut: &str) {
    let size = (font::SIZE_TINY * SCALE).max(MIN_TAG_TEXT);
    let style = TextStyle::new(size, Weight::Black)
        .align(Align::Center)
        .middle();
    let padding = 6.0 * SCALE;
    let width = measure.text_size(shortcut, style, f32::INFINITY).0 + padding * 2.0;
    let tag = Rect::new(
        rect.center_x() - width / 2.0,
        rect.y + 3.0 * SCALE,
        width,
        size + 4.0,
    );
    ui.fill(tag, 3.0, theme::BG_DEEP.alpha(0.85));
    ui.stroke(tag, 3.0, theme::HAIRLINE_WIDTH, theme::BORDER);
    ui.text(tag, shortcut, style, theme::TEXT_DIM);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SLOT_COUNT;
    use crate::ui::toolkit::Visual;

    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, _max: f32) -> (f32, f32) {
            (text.chars().count() as f32 * style.size * 0.6, style.size)
        }
    }

    const AREA: Rect = Rect::new(
        0.0,
        0.0,
        super::super::STRIP_WIDTH,
        super::super::STRIP_HEIGHT,
    );

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    fn build_strip(ui: &mut Ui, data: &GameData, settings: &Settings, slots: Slots) {
        ui.begin(0);
        build(
            ui,
            &mut Fixed,
            AREA,
            &Ctx {
                data,
                settings,
                slots,
            },
        );
        ui.end();
    }

    #[test]
    fn the_bar_fits_the_window_and_sits_at_the_bottom() {
        let (width, height) = size(SLOT_COUNT);
        assert!(width <= super::super::STRIP_WIDTH, "{width}");
        assert!(
            height + BOTTOM_MARGIN <= super::super::STRIP_HEIGHT,
            "{height}"
        );

        let data = data();
        let settings = Settings::default();
        let mut ui = Ui::new();
        build_strip(&mut ui, &data, &settings, Slots::default());

        let bar = ui.frame().nodes[0].rect;
        assert!(
            (bar.center_x() - AREA.center_x()).abs() < 0.5,
            "centralizada"
        );
        assert_eq!(bar.bottom(), AREA.bottom() - BOTTOM_MARGIN);
    }

    #[test]
    fn every_slot_shows_its_icon_and_shortcut() {
        let data = data();
        let settings = Settings::default();
        let ids: Vec<u32> = data.all().iter().take(2).map(|strat| strat.id).collect();
        let mut ui = Ui::new();
        build_strip(
            &mut ui,
            &data,
            &settings,
            [Some(ids[0]), Some(ids[1]), None, None],
        );

        let icons = ui
            .frame()
            .nodes
            .iter()
            .filter(|node| matches!(node.visual, Visual::Image { .. }))
            .count();
        assert_eq!(icons, 2, "só os slots ocupados desenham ícone");

        for shortcut in ["F1", "F2", "F3", "F4"] {
            assert!(
                ui.frame().nodes.iter().any(|node| matches!(
                    &node.visual,
                    Visual::Text { text, .. } if text == shortcut
                )),
                "{shortcut} não apareceu"
            );
        }
    }

    #[test]
    fn the_strip_never_answers_the_mouse() {
        // A janela é click-through; registrar área clicável seria mentira.
        let data = data();
        let settings = Settings::default();
        let mut ui = Ui::new();
        build_strip(&mut ui, &data, &settings, Slots::default());

        assert_eq!(
            ui.frame().hit_at(AREA.center_x(), AREA.bottom() - 40.0),
            None
        );
    }

    #[test]
    fn a_trigger_flashes_the_slot_and_asks_for_the_timer() {
        let data = data();
        let settings = Settings::default();
        let id = data.all()[0].id;
        let slots: Slots = [Some(id), None, None, None];
        let mut ui = Ui::new();

        build_strip(&mut ui, &data, &settings, slots);
        assert!(!ui.animating(), "parado, o strip não acorda a CPU");

        ui.flash(
            widgets::flash_id(0, false, FlashKind::Triggered),
            widgets::FLASH_TRIGGERED_MS,
        );
        build_strip(&mut ui, &data, &settings, slots);
        assert!(ui.animating(), "a piscada pede o timer de animação");
        assert!(
            ui.frame().nodes.iter().any(|node| matches!(
                &node.visual,
                Visual::Stroke { color, .. } if *color == theme::YELLOW
            )),
            "o slot disparado ganha a borda amarela"
        );
    }
}
