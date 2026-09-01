//! Modal genérico em D2D: véu escuro sobre a tela inteira e um cartão no meio
//! com título, texto e dois botões.
//!
//! Hoje quem o usa é o aviso de atualização pronta (porte do modal da v1 em
//! `legacy/src/renderer/App.jsx` ~937-980). O véu não é só decoração: ele
//! registra uma área clicável do tamanho da tela, e como é desenhado por último
//! ele fica por cima de tudo no hit-test — nenhum clique vaza para a aba de
//! trás enquanto o modal está aberto.

use crate::ui::theme::{self, font, Color};
use crate::ui::toolkit::{id, Align, Id, Measure, Rect, TextStyle, Ui, Weight};
use crate::ui::widgets::{self, ButtonVariant};

/// Largura do cartão (`max-w-md` da v1).
pub const WIDTH: f32 = 420.0;
/// `p-8`.
const PADDING: f32 = 32.0;
/// `rounded-3xl`.
const RADIUS: f32 = 24.0;
/// Borda de acento (`border-2`).
const BORDER_WIDTH: f32 = 2.0;
/// Quadrado do ícone (`w-12 h-12`) e a bolinha que pulsa dentro dele.
const ICON: f32 = 48.0;
const ICON_DOT: f32 = 12.0;
const ICON_GAP: f32 = 16.0;
/// Altura das duas linhas do cabeçalho.
const TITLE_H: f32 = 20.0;
const SUBTITLE_H: f32 = 14.0;
/// `space-y-6` entre cabeçalho, texto e botões.
const SECTION_GAP: f32 = 24.0;
/// `py-4` dos botões.
const BUTTON_H: f32 = 48.0;
const BUTTON_GAP: f32 = 12.0;
/// Largura do botão secundário; o primário fica com o resto.
const SECONDARY_WIDTH: f32 = 130.0;
/// Período do pulso da bolinha (`animate-pulse` do legado).
const PULSE_MS: u32 = 1_400;

/// Conteúdo do modal.
#[derive(Debug, Clone, Copy)]
pub struct Modal<'a> {
    pub title: &'a str,
    /// Linha pequena sob o título (a versão, no aviso de atualização).
    pub subtitle: &'a str,
    pub body: &'a str,
    pub primary: &'a str,
    pub secondary: &'a str,
    pub accent: Color,
}

/// Véu de fundo. Clicar nele não faz nada — só impede que o clique chegue na
/// tela de trás.
pub fn scrim_id() -> Id {
    id("modal.scrim")
}

/// Botão de acento (o "Reiniciar Agora").
pub fn primary_id() -> Id {
    id("modal.primary")
}

/// Botão de recusa (o "Depois").
pub fn secondary_id() -> Id {
    id("modal.secondary")
}

fn body_style() -> TextStyle {
    TextStyle::new(font::SIZE_BODY, Weight::Regular).wrap()
}

/// Altura do cartão com este conteúdo.
pub fn height(measure: &mut dyn Measure, modal: &Modal) -> f32 {
    let inner = WIDTH - PADDING * 2.0;
    let body = measure.text_size(modal.body, body_style(), inner).1;
    PADDING * 2.0 + ICON + SECTION_GAP + body + SECTION_GAP + BUTTON_H
}

/// Desenha o modal cobrindo `area`. Chamar por último na construção da tela.
pub fn show(ui: &mut Ui, measure: &mut dyn Measure, area: Rect, modal: &Modal) {
    ui.fill(area, 0.0, theme::BG_DEEP.alpha(0.8));
    ui.hit(scrim_id(), area);

    let card = area.centered(WIDTH, height(measure, modal));
    ui.fill(card, RADIUS, theme::CARD_BG);
    ui.stroke(card, RADIUS, BORDER_WIDTH, modal.accent.alpha(0.5));

    let mut content = card.inset(PADDING);
    header(ui, content.cut_top(ICON), modal);
    content.skip_top(SECTION_GAP);

    let body_h = measure.text_size(modal.body, body_style(), content.w).1;
    ui.text(
        content.cut_top(body_h),
        modal.body,
        body_style(),
        theme::TEXT,
    );
    content.skip_top(SECTION_GAP);

    let mut buttons = content.cut_top(BUTTON_H);
    let secondary = buttons.cut_right(SECONDARY_WIDTH);
    buttons.cut_right(BUTTON_GAP);
    widgets::button(
        ui,
        primary_id(),
        buttons,
        modal.primary,
        ButtonVariant::Primary,
        modal.accent,
    );
    widgets::button(
        ui,
        secondary_id(),
        secondary,
        modal.secondary,
        ButtonVariant::Secondary,
        modal.accent,
    );
}

/// Quadrado do ícone com a bolinha pulsando, título e subtítulo.
fn header(ui: &mut Ui, rect: Rect, modal: &Modal) {
    let mut row = rect;
    let icon = row.cut_left(ICON);
    ui.fill(icon, theme::RADIUS_CARD, modal.accent.alpha(0.2));
    ui.stroke(
        icon,
        theme::RADIUS_CARD,
        theme::HAIRLINE_WIDTH,
        modal.accent.alpha(0.3),
    );
    let pulse = ui.pulse(PULSE_MS);
    ui.ellipse(
        icon.centered(ICON_DOT, ICON_DOT),
        modal.accent.alpha(0.6 + 0.4 * pulse),
    );
    row.cut_left(ICON_GAP);

    let mut text = row.middle_row(TITLE_H + SUBTITLE_H);
    ui.text(
        text.cut_top(TITLE_H),
        modal.title.to_uppercase(),
        TextStyle::new(font::SIZE_TITLE, Weight::Black).tracking(font::TRACKING_WIDE),
        modal.accent,
    );
    ui.text(
        text.cut_top(SUBTITLE_H),
        modal.subtitle.to_uppercase(),
        TextStyle::new(font::SIZE_LABEL, Weight::Black)
            .tracking(font::TRACKING_LABEL)
            .align(Align::Start),
        theme::TEXT_DIM,
    );
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
        title: "Atualização Disponível",
        subtitle: "v2.1.0",
        body: "Uma nova versão foi baixada e está pronta para ser instalada.",
        primary: "Reiniciar Agora",
        secondary: "Depois",
        accent: theme::YELLOW,
    };

    fn build(ui: &mut Ui, modal: &Modal) {
        ui.begin(0);
        show(ui, &mut Fixed, AREA, modal);
        ui.end();
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
        // Um botão da tela de trás, desenhado antes do modal.
        ui.begin(0);
        ui.hit(id("tela.botao"), AREA);
        show(&mut ui, &mut Fixed, AREA, &UPDATE);
        ui.end();

        // Canto da tela: o véu ganha do que está embaixo.
        assert_eq!(ui.frame().hit_at(4.0, 4.0), Some(scrim_id()));
        // E o cartão continua clicável no meio.
        let card = AREA.centered(WIDTH, height(&mut Fixed, &UPDATE));
        let button_y = card.bottom() - PADDING - BUTTON_H / 2.0;
        assert_eq!(
            ui.frame().hit_at(card.x + PADDING + 10.0, button_y),
            Some(primary_id())
        );
        assert_eq!(
            ui.frame()
                .hit_at(card.right() - PADDING - SECONDARY_WIDTH / 2.0, button_y),
            Some(secondary_id())
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
        assert!(texts.iter().any(|text| text.contains("ATUALIZAÇÃO")));
        assert!(texts.iter().any(|text| text.contains("V2.1.0")));
        assert!(texts.iter().any(|text| text == UPDATE.body));
        assert!(texts.iter().any(|text| text.contains("REINICIAR")));
        assert!(texts.iter().any(|text| text.contains("DEPOIS")));
    }

    #[test]
    fn the_dot_keeps_pulsing_while_the_modal_is_open() {
        let mut ui = Ui::new();
        build(&mut ui, &UPDATE);
        assert!(ui.animating());
    }
}
