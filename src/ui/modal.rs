//! Diálogo genérico em D2D (styleguide §6.5): véu sobre a tela inteira e um
//! `dialog-box` no meio (moldura, sombra dura, barra de título com os
//! quadrados de janela, título grande, texto e dois botões).
//!
//! Hoje quem o usa é o aviso de atualização pronta. O véu não é só decoração:
//! ele registra uma área clicável do tamanho da tela, e como é desenhado por
//! último ele fica por cima de tudo no hit-test; nenhum clique vaza para a
//! aba de trás enquanto o diálogo está aberto.

use crate::ui::theme::{self, font, motion};
use crate::ui::toolkit::{id, Align, Id, Measure, Rect, TextStyle, Ui, Weight};
use crate::ui::widgets::{self, styles, ButtonVariant};

/// Largura do diálogo.
pub const WIDTH: f32 = 440.0;
const PADDING: f32 = 24.0;
/// Barra de título.
const BAR_HEIGHT: f32 = 32.0;
const BAR_PADDING: f32 = 12.0;
/// Kicker e título.
const KICKER_H: f32 = 16.0;
const TITLE_H: f32 = 30.0;
/// Espaço entre cabeçalho, texto e botões.
const SECTION_GAP: f32 = 20.0;
const BUTTON_H: f32 = 48.0;
const BUTTON_GAP: f32 = 12.0;
/// Largura do botão secundário; o primário fica com o resto.
const SECONDARY_WIDTH: f32 = 130.0;
/// Opacidade do véu (`bg-base-100/80`, sem blur).
const SCRIM_ALPHA: f32 = 0.8;

/// Conteúdo do diálogo.
#[derive(Debug, Clone, Copy)]
pub struct Modal<'a> {
    /// Nome de "arquivo" da barra (`UPDATE.EXE`).
    pub file: &'a str,
    /// Linha pequena sobre o título (a versão, no aviso de atualização).
    pub kicker: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub primary: &'a str,
    pub secondary: &'a str,
}

/// Véu de fundo. Clicar nele não faz nada, só impede que o clique chegue na
/// tela de trás.
pub fn scrim_id() -> Id {
    id("modal.scrim")
}

/// Botão de ação (o "Reiniciar Agora").
pub fn primary_id() -> Id {
    id("modal.primary")
}

/// Botão de recusa (o "Depois").
pub fn secondary_id() -> Id {
    id("modal.secondary")
}

/// Chave da animação de entrada.
fn enter_id() -> Id {
    id("modal.enter")
}

fn body_style() -> TextStyle {
    styles::body().wrap()
}

/// Altura do diálogo com este conteúdo.
pub fn height(measure: &mut dyn Measure, modal: &Modal) -> f32 {
    let inner = WIDTH - PADDING * 2.0;
    let body = measure.text_size(modal.body, body_style(), inner).1;
    BAR_HEIGHT + PADDING * 2.0 + KICKER_H + TITLE_H + SECTION_GAP + body + SECTION_GAP + BUTTON_H
}

/// Desenha o diálogo cobrindo `area`. Chamar por último na construção da tela.
pub fn show(ui: &mut Ui, measure: &mut dyn Measure, area: Rect, modal: &Modal) {
    let palette = theme::palette();
    ui.fill(area, palette.base_100.faded(SCRIM_ALPHA));
    ui.hit(scrim_id(), area);

    // `animate-enter`: sobe 8 DIP até o lugar, sem overshoot.
    let enter = ui.fade(enter_id(), true, motion::ENTER_MS);
    let rise = motion::ENTER_RISE * (1.0 - motion::ease_out(enter));
    let card = area
        .centered(WIDTH, height(measure, modal))
        .translate(0.0, rise);

    ui.fill(card.translate(theme::SHADOW, theme::SHADOW), palette.shadow);
    ui.fill(card, palette.base_200);

    let mut content = card;
    let bar = content.cut_top(BAR_HEIGHT);
    ui.fill(bar, palette.base_100);
    ui.fill(
        Rect::new(bar.x, bar.bottom() - theme::BORDER, bar.w, theme::BORDER),
        palette.base_300,
    );
    let row = widgets::window_dots(ui, bar.inset_xy(BAR_PADDING, 0.0), palette.accent);
    ui.text(
        row,
        modal.file.to_uppercase(),
        styles::micro().middle(),
        palette.muted,
    );

    let mut content = content.inset(PADDING);
    ui.text(
        content.cut_top(KICKER_H),
        widgets::sigil(modal.kicker),
        styles::micro(),
        palette.accent_text,
    );
    ui.text(
        content.cut_top(TITLE_H),
        modal.title.to_uppercase(),
        TextStyle::new(font::SIZE_TITLE, Weight::Black)
            .italic()
            .tracking(font::TRACKING_TIGHTER)
            .align(Align::Start),
        palette.content,
    );
    content.skip_top(SECTION_GAP);

    let body_h = measure.text_size(modal.body, body_style(), content.w).1;
    ui.text(
        content.cut_top(body_h),
        modal.body,
        body_style(),
        palette.content,
    );
    content.skip_top(SECTION_GAP);

    let mut buttons = content.cut_top(BUTTON_H);
    let secondary = buttons.cut_left(SECONDARY_WIDTH);
    buttons.cut_left(BUTTON_GAP);
    // Descartar à esquerda, a ação à direita: o rodapé de formulário do guia.
    widgets::button(
        ui,
        secondary_id(),
        secondary,
        modal.secondary,
        ButtonVariant::Secondary,
    );
    widgets::button(
        ui,
        primary_id(),
        buttons,
        modal.primary,
        ButtonVariant::Primary,
    );
    ui.stroke(card, theme::BORDER, palette.base_300);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::Visual;

    /// Medidor de largura fixa, como o das outras telas.
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

    const AREA: Rect = Rect::new(0.0, 0.0, 820.0, 640.0);

    const UPDATE: Modal<'static> = Modal {
        file: "update.exe",
        kicker: "v2.1.0",
        title: "Atualização Disponível",
        body: "Uma nova versão foi baixada e está pronta para ser instalada.",
        primary: "Reiniciar Agora",
        secondary: "Depois",
    };

    fn build_at(ui: &mut Ui, modal: &Modal, now: u64) {
        ui.begin(now);
        show(ui, &mut Fixed, AREA, modal);
        ui.end();
    }

    fn build(ui: &mut Ui, modal: &Modal) {
        build_at(ui, modal, 0);
    }

    /// Duas passagens com folga: a entrada termina e o diálogo para no lugar.
    fn build_settled(ui: &mut Ui, modal: &Modal) {
        build_at(ui, modal, 0);
        build_at(ui, modal, 1_000);
    }

    #[test]
    fn both_buttons_answer_the_mouse() {
        let mut ui = Ui::new();
        build(&mut ui, &UPDATE);

        assert!(ui.frame().has_hit(primary_id()));
        assert!(ui.frame().has_hit(secondary_id()));
        assert!(ui.frame().has_hit(scrim_id()));
    }

    #[test]
    fn the_scrim_swallows_every_click_outside_the_card() {
        let mut ui = Ui::new();
        // Um botão da tela de trás, desenhado antes do diálogo.
        let draw = |ui: &mut Ui, now: u64| {
            ui.begin(now);
            ui.hit(id("tela.botao"), AREA);
            show(ui, &mut Fixed, AREA, &UPDATE);
            ui.end();
        };
        draw(&mut ui, 0);
        draw(&mut ui, 1_000);

        // Canto da tela: o véu ganha do que está embaixo.
        assert_eq!(ui.frame().hit_at(4.0, 4.0), Some(scrim_id()));
        // E o diálogo continua clicável no meio.
        let card = AREA.centered(WIDTH, height(&mut Fixed, &UPDATE));
        let button_y = card.bottom() - PADDING - BUTTON_H / 2.0;
        assert_eq!(
            ui.frame()
                .hit_at(card.x + PADDING + SECONDARY_WIDTH / 2.0, button_y),
            Some(secondary_id())
        );
        assert_eq!(
            ui.frame().hit_at(card.right() - PADDING - 10.0, button_y),
            Some(primary_id())
        );
    }

    #[test]
    fn the_card_grows_with_the_text() {
        let short = Modal {
            body: "Curto.",
            ..UPDATE
        };
        let long = Modal {
            body: "Um texto bem mais longo, que quebra em várias linhas dentro do cartão e \
                   empurra os botões para baixo sem transbordar.",
            ..UPDATE
        };
        assert!(height(&mut Fixed, &long) > height(&mut Fixed, &short));
    }

    #[test]
    fn the_content_shows_up_on_screen() {
        let mut ui = Ui::new();
        build(&mut ui, &UPDATE);

        let texts: Vec<String> = ui
            .frame()
            .nodes
            .iter()
            .filter_map(|node| match &node.visual {
                Visual::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"UPDATE.EXE".to_string()));
        assert!(texts.contains(&"> V2.1.0".to_string()));
        assert!(texts.iter().any(|text| text.contains("ATUALIZAÇÃO")));
        assert!(texts.iter().any(|text| text == UPDATE.body));
        assert!(texts.iter().any(|text| text.contains("REINICIAR")));
        assert!(texts.iter().any(|text| text.contains("DEPOIS")));
    }

    #[test]
    fn the_dialog_rises_into_place_and_then_stops() {
        let mut ui = Ui::new();
        build(&mut ui, &UPDATE);
        assert!(ui.animating(), "a entrada pede quadros");

        build_settled(&mut ui, &UPDATE);
        assert!(
            !ui.animating(),
            "parado no lugar, o diálogo não acorda a CPU"
        );
    }
}
