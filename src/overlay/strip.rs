//! Conteúdo do strip: os quatro slots de macro desenhados por cima do jogo.
//!
//! É a barra de slots da janela principal a 70%: o painel com moldura e
//! sombra dura, os slots quadrados com a etiqueta do atalho colada na borda.
//! Como a janela é click-through, aqui não há hover, botão de limpar nem slot em
//! edição: sobra o que o jogador precisa ver de relance (ícone, atalho e as
//! piscadas de disparo e bloqueio).
//!
//! O módulo é puro (só empurra nós num [`Ui`]), então roda e é testado no host.

use crate::data::{GameData, Stratagem};
use crate::settings::Settings;
use crate::shared::{FlashKind, Slots};
use crate::ui::theme::{self, font};
use crate::ui::toolkit::{Align, Measure, Rect, TextStyle, Ui, Weight};
use crate::ui::widgets;

/// Escala do strip em relação à barra da janela principal.
pub const SCALE: f32 = 0.70;
/// Folga entre a barra e a base da tela: cabe a sombra dura.
const BOTTOM_MARGIN: f32 = 8.0;
/// Padding e espaço da barra, antes da escala. O de cima cabe a etiqueta.
const PADDING: f32 = 18.0;
const GAP: f32 = 16.0;
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

    // Painel sólido: sobre o jogo, qualquer transparência vira ruído.
    let palette = theme::palette();
    ui.fill(
        bar.translate(theme::SHADOW_SM, theme::SHADOW_SM),
        palette.shadow,
    );
    ui.fill(bar, palette.base_200);
    ui.stroke(bar, theme::BORDER, palette.base_300);

    let (slot, gap, padding) = (slot_size(), GAP * SCALE, PADDING * SCALE);
    let empty = widgets::bracketed(crate::i18n::tr(ctx.settings.language).macros.empty);
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
            &empty,
        );
        x += slot + gap;
    }
}

/// Um slot do strip. Mesma leitura do `slot_square` da janela principal, com as
/// medidas proporcionais ao quadrado: na escala do strip, as constantes em DIP
/// da versão grande sairiam desproporcionais.
fn square(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    index: usize,
    rect: Rect,
    strat: Option<&Stratagem>,
    shortcut: &str,
    empty: &str,
) {
    let triggered = ui.anim(
        widgets::flash_id(index, false, FlashKind::Triggered),
        widgets::FLASH_TRIGGERED_MS,
    );
    let blocked = ui.anim(
        widgets::flash_id(index, false, FlashKind::Blocked),
        widgets::FLASH_BLOCKED_MS,
    );
    let palette = theme::palette();

    // Bloqueio: moldura de erro em volta do quadrado.
    if blocked > 0.0 {
        ui.stroke(
            rect.inset(-4.0),
            theme::BORDER,
            palette.error.fill.faded(blocked),
        );
    }

    ui.fill(rect, palette.base_100);
    match strat {
        Some(strat) => ui.image(
            rect.inset(theme::BORDER),
            format!("icons/{}", strat.imagem),
            1.0,
        ),
        None => ui.text(
            rect,
            empty,
            TextStyle::new(MIN_TAG_TEXT, Weight::Bold)
                .align(Align::Center)
                .middle(),
            palette.muted,
        ),
    }
    // Disparo: o slot inunda de accent.
    if triggered > 0.0 {
        ui.fill(
            rect.inset(theme::BORDER),
            palette.accent.faded(0.6 * triggered),
        );
    }
    ui.stroke(rect, theme::BORDER, palette.base_300);
    tag(ui, measure, rect, shortcut, triggered > 0.0);
}

/// Etiqueta do atalho, meio para fora da borda de cima do quadrado.
fn tag(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, shortcut: &str, lit: bool) {
    let palette = theme::palette();
    let size = (font::SIZE_TINY * SCALE).max(MIN_TAG_TEXT);
    let style = TextStyle::new(size, Weight::Black)
        .align(Align::Center)
        .middle();
    let padding = 6.0 * SCALE;
    let width = measure.text_size(shortcut, style, f32::INFINITY).0 + padding * 2.0;
    let height = size + 5.0;
    let tag = Rect::new(
        rect.center_x() - width / 2.0,
        rect.y - height / 2.0,
        width,
        height,
    );
    let (fill, ink) = if lit {
        (palette.accent, palette.accent_content)
    } else {
        (palette.base_200, palette.content)
    };
    ui.fill(tag, fill);
    ui.stroke(tag, theme::BORDER * 0.75, palette.base_300);
    ui.text(tag, shortcut, style, ink);
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

        // O primeiro nó é a sombra dura; a barra vem logo depois.
        let bar = ui.frame().nodes[1].rect;
        assert!(
            (bar.center_x() - AREA.center_x()).abs() < 0.5,
            "centralizada"
        );
        assert_eq!(bar.bottom(), AREA.bottom() - BOTTOM_MARGIN);
        assert!(
            bar.bottom() + theme::SHADOW_SM <= AREA.bottom(),
            "a sombra cabe na janela"
        );
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
        let palette = theme::palette();
        assert!(
            ui.frame().nodes.iter().any(|node| matches!(
                &node.visual,
                Visual::Text { text, color, .. } if text == "F1" && *color == palette.accent_content
            )),
            "o slot disparado acende a etiqueta em accent"
        );
    }
}
