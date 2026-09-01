//! Primeira leva de widgets: barra de abas, card, botão, linha de toggle e o
//! lugar reservado para um `EDIT` nativo.
//!
//! Cada função só empurra nós e áreas clicáveis no [`Ui`]; quem interpreta o
//! clique é a tela, comparando o id devolvido por
//! [`Ui::input`](crate::ui::toolkit::Ui::input) com os ids que estas funções
//! expõem. Assim nenhum widget precisa de callback, e a construção continua
//! sendo uma função pura do estado.
//!
//! As medidas vêm das classes Tailwind da v1 (`legacy/src/renderer/index.css` e
//! o JSX), convertidas para DIP.

use crate::ui::theme::{self, font, Color};
use crate::ui::toolkit::{id_at, Align, Id, Measure, Rect, TextStyle, Ui, Weight};

/// Altura do header com as abas (`py-2` em volta de botões `py-5`).
pub const TAB_BAR_HEIGHT: f32 = 68.0;
/// Altura do botão de aba (`px-12 py-5` com texto de 10px).
const TAB_HEIGHT: f32 = 52.0;
/// `px-12`.
const TAB_PADDING_X: f32 = 48.0;
/// Barra de acento no topo da aba ativa (`h-[3px]`).
const TAB_ACCENT_HEIGHT: f32 = 3.0;
/// Marca decorativa de canto do HUD (`w-8 h-8` com borda cyan/20).
const CORNER_SIZE: f32 = 32.0;

/// `p-6` dos cards.
pub const CARD_PADDING: f32 = 24.0;
/// Altura da faixa do header do card (indicador + título + divisor).
const CARD_HEADER_HEIGHT: f32 = 30.0;
/// `mb-6` entre o divisor do header e o conteúdo.
const CARD_HEADER_GAP: f32 = 24.0;
/// `w-2 h-2` do `hd-indicator`.
const INDICATOR_SIZE: f32 = 8.0;

/// Altura padrão de botão e campo de texto.
pub const CONTROL_HEIGHT: f32 = 40.0;
/// `border-b-4` do botão primário.
const BUTTON_UNDERLINE: f32 = 4.0;
/// Encolhimento do `active:scale-95` — em DIP, não em transformação.
const BUTTON_PRESS_INSET: f32 = 2.0;

/// Trilho do switch (`w-11 h-6`).
const SWITCH_WIDTH: f32 = 44.0;
const SWITCH_HEIGHT: f32 = 24.0;
const SWITCH_KNOB: f32 = 18.0;

/// Duração dos fades de hover (`transition-all duration-300` do CSS).
const HOVER_MS: u32 = 180;
/// Duração do deslizar do switch.
const SWITCH_MS: u32 = 160;

/// Aba do header.
#[derive(Debug, Clone, Copy)]
pub struct Tab<'a> {
    pub label: &'a str,
    /// Cor do estado ativo: ciano nas duas primeiras, amarelo em Configurações.
    pub accent: Color,
}

/// Id da aba `index` — o que a tela compara com o clique.
pub fn tab_id(index: usize) -> Id {
    id_at("ui.tab", index)
}

/// Header com as abas centralizadas, divisores de 1px e as marcas de canto.
pub fn tab_bar(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, tabs: &[Tab], active: usize) {
    ui.fill(rect, 0.0, theme::BG_DEEP);
    ui.fill(
        Rect::new(rect.x, rect.bottom() - 1.0, rect.w, theme::HAIRLINE_WIDTH),
        0.0,
        theme::HAIRLINE,
    );
    hud_corners(ui, rect);

    let style = tab_style();
    let labels: Vec<String> = tabs.iter().map(|tab| tab.label.to_uppercase()).collect();
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| measure.text_size(label, style, f32::INFINITY).0 + TAB_PADDING_X * 2.0)
        .collect();

    // Um divisor antes de cada aba e um depois da última.
    let total: f32 = widths.iter().sum::<f32>() + (tabs.len() + 1) as f32 * theme::HAIRLINE_WIDTH;
    let row = rect.middle_row(TAB_HEIGHT);
    let mut cursor = Rect::new(row.x + (row.w - total) / 2.0, row.y, total, row.h);

    for (index, tab) in tabs.iter().enumerate() {
        divider(ui, cursor.cut_left(theme::HAIRLINE_WIDTH));
        let slot = cursor.cut_left(widths[index]);
        let id = tab_id(index);
        if index == active {
            active_tab(ui, slot, &labels[index], style, tab.accent);
        } else {
            inactive_tab(ui, id, slot, &labels[index], style);
        }
        ui.hit(id, slot);
    }
    divider(ui, cursor.cut_left(theme::HAIRLINE_WIDTH));
}

fn tab_style() -> TextStyle {
    TextStyle::new(font::SIZE_LABEL, Weight::Black)
        .tracking(font::TRACKING_WIDE)
        .align(Align::Center)
        .middle()
}

fn divider(ui: &mut Ui, rect: Rect) {
    ui.fill(rect.middle_row(24.0), 0.0, theme::BORDER);
}

fn active_tab(ui: &mut Ui, rect: Rect, label: &str, style: TextStyle, accent: Color) {
    ui.fill(rect, 0.0, accent.alpha(0.1));
    let bar = Rect::new(rect.x, rect.y, rect.w, TAB_ACCENT_HEIGHT);
    ui.fill(bar, 0.0, accent);
    ui.glow(bar, 0.0, accent);
    ui.text(rect, label, style, accent);
}

fn inactive_tab(ui: &mut Ui, id: Id, rect: Rect, label: &str, style: TextStyle) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    if hover > 0.0 {
        ui.fill(rect, 0.0, theme::SURFACE_HOVER.alpha(0.5 * hover));
        // `hover::after`: risco amarelo que cresce do centro para as laterais.
        let inset = 16.0 + (rect.w / 2.0 - 16.0) * (1.0 - hover);
        ui.fill(
            Rect::new(
                rect.x + inset,
                rect.bottom() - theme::HAIRLINE_WIDTH,
                (rect.w - inset * 2.0).max(0.0),
                theme::HAIRLINE_WIDTH,
            ),
            0.0,
            theme::YELLOW.alpha(0.3 * hover),
        );
    }
    ui.text(
        rect,
        label,
        style,
        theme::TEXT_DIM.mix(theme::TEXT_HOVER, hover),
    );
}

/// Cantos do HUD: dois "L" finos em ciano translúcido, como no header da v1.
fn hud_corners(ui: &mut Ui, rect: Rect) {
    let color = theme::CYAN.alpha(0.2);
    let thickness = theme::HAIRLINE_WIDTH;
    for left in [true, false] {
        let x = if left {
            rect.x
        } else {
            rect.right() - CORNER_SIZE
        };
        ui.fill(Rect::new(x, rect.y, CORNER_SIZE, thickness), 0.0, color);
        let vertical_x = if left {
            rect.x
        } else {
            rect.right() - thickness
        };
        ui.fill(
            Rect::new(vertical_x, rect.y, thickness, CORNER_SIZE),
            0.0,
            color,
        );
    }
}

/// Header opcional de um card.
#[derive(Debug, Clone, Copy)]
pub struct CardHeader<'a> {
    pub title: &'a str,
    pub accent: Color,
}

/// Card do tema (`hd-card`): fundo, borda, raio 16 e um filete de acento no
/// topo. Devolve o retângulo interno, já descontados padding e header.
pub fn card(ui: &mut Ui, rect: Rect, header: Option<CardHeader<'_>>) -> Rect {
    ui.fill(rect, theme::RADIUS_CARD, theme::CARD_BG);
    ui.stroke(
        rect,
        theme::RADIUS_CARD,
        theme::HAIRLINE_WIDTH,
        theme::BORDER,
    );

    let mut content = rect.inset(CARD_PADDING);
    let Some(header) = header else {
        return content;
    };

    // `border-t-2` do card: some nos cantos arredondados, então começa depois deles.
    ui.fill(
        Rect::new(
            rect.x + theme::RADIUS_CARD,
            rect.y,
            (rect.w - theme::RADIUS_CARD * 2.0).max(0.0),
            2.0,
        ),
        0.0,
        header.accent.alpha(0.2),
    );

    let mut row = content.cut_top(CARD_HEADER_HEIGHT);
    let indicator = row
        .cut_left(INDICATOR_SIZE + 12.0)
        .middle_row(INDICATOR_SIZE)
        .with_w(INDICATOR_SIZE);
    ui.ellipse(indicator, header.accent);
    ui.glow(indicator, INDICATOR_SIZE / 2.0, header.accent);
    ui.text(
        row,
        header.title.to_uppercase(),
        TextStyle::new(font::SIZE_CARD_HEADER, Weight::Black)
            .tracking(font::TRACKING_WIDE)
            .middle(),
        header.accent,
    );
    ui.fill(
        Rect::new(content.x, content.y, content.w, theme::HAIRLINE_WIDTH),
        0.0,
        theme::HAIRLINE,
    );
    content.skip_top(CARD_HEADER_GAP);
    content
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// `hd-btn-primary`: fundo de acento, texto escuro, filete inferior.
    Primary,
    /// Fundo escuro com borda; a borda acende no hover.
    Secondary,
    /// Só texto, para ações discretas.
    Ghost,
}

/// Botão do tema. `accent` vale para Primary (fundo) e Secondary (borda no hover).
pub fn button(ui: &mut Ui, id: Id, rect: Rect, label: &str, variant: ButtonVariant, accent: Color) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    // `active:scale-95` sem transformação: o botão encolhe um fio ao ser
    // pressionado, o que dá a mesma leitura e não mexe no layout.
    let rect = if ui.is_pressed(id) {
        rect.inset(BUTTON_PRESS_INSET)
    } else {
        rect
    };
    let style = TextStyle::new(font::SIZE_LABEL, Weight::Black)
        .tracking(font::TRACKING_WIDE)
        .align(Align::Center)
        .middle();

    match variant {
        ButtonVariant::Primary => {
            let fill = accent.mix(theme::TEXT, 0.15 * hover);
            ui.fill(rect, theme::RADIUS_BUTTON, fill);
            // Filete inferior escuro (`border-b-4`), dentro do raio.
            ui.fill(
                Rect::new(
                    rect.x + theme::RADIUS_BUTTON / 2.0,
                    rect.bottom() - BUTTON_UNDERLINE,
                    (rect.w - theme::RADIUS_BUTTON).max(0.0),
                    BUTTON_UNDERLINE,
                ),
                0.0,
                accent.mix(theme::BG_DEEP, 0.45),
            );
            ui.text(rect, label.to_uppercase(), style, theme::TEXT_ON_ACCENT);
        }
        ButtonVariant::Secondary => {
            ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
            ui.stroke(
                rect,
                theme::RADIUS_BUTTON,
                theme::HAIRLINE_WIDTH,
                theme::BORDER.mix(accent.alpha(0.5), hover),
            );
            ui.text(
                rect,
                label.to_uppercase(),
                style,
                theme::TEXT_DIM.mix(theme::TEXT, hover),
            );
        }
        ButtonVariant::Ghost => {
            if hover > 0.0 {
                ui.fill(
                    rect,
                    theme::RADIUS_BUTTON,
                    theme::SURFACE.alpha(0.6 * hover),
                );
            }
            ui.text(
                rect,
                label.to_uppercase(),
                style,
                theme::TEXT_DIM.mix(theme::TEXT, hover),
            );
        }
    }
    ui.hit(id, rect);
}

/// Linha "label + descrição + switch". `on` é o estado atual; o clique chega
/// pelo id, e quem muda o settings é a tela.
pub fn toggle_row(ui: &mut Ui, id: Id, rect: Rect, label: &str, desc: &str, on: bool) {
    let mut row = rect;
    let switch = row.cut_right(SWITCH_WIDTH).middle_row(SWITCH_HEIGHT);
    row.cut_right(16.0);

    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    let mut text_area = row;
    let label_row = text_area.cut_top(16.0);
    ui.text(
        label_row,
        label.to_uppercase(),
        TextStyle::new(font::SIZE_LABEL, Weight::Black).tracking(font::TRACKING_LABEL),
        theme::TEXT,
    );
    if !desc.is_empty() {
        ui.text(
            text_area,
            desc,
            TextStyle::new(font::SIZE_TINY, Weight::Regular).wrap(),
            theme::TEXT_DIM,
        );
    }

    switch_track(ui, id, switch, on, hover);
    ui.hit(id, rect);
}

fn switch_track(ui: &mut Ui, id: Id, rect: Rect, on: bool, hover: f32) {
    // O mesmo id serve aos dois fades: o do trilho anda com o estado, o do
    // hover com o mouse, e cada um tem sua chave.
    let travel = ui.fade(id ^ 0x5357, on, SWITCH_MS);
    let radius = rect.h / 2.0;
    let track = theme::SURFACE.mix(theme::YELLOW.alpha(0.35), travel);
    ui.fill(rect, radius, track);
    ui.stroke(
        rect,
        radius,
        theme::HAIRLINE_WIDTH,
        theme::BORDER.mix(theme::YELLOW, travel.max(hover * 0.3)),
    );

    let padding = (rect.h - SWITCH_KNOB) / 2.0;
    let knob_x = rect.x + padding + (rect.w - SWITCH_KNOB - padding * 2.0) * travel;
    let knob = Rect::new(knob_x, rect.y + padding, SWITCH_KNOB, SWITCH_KNOB);
    ui.ellipse(knob, theme::TEXT_DIM.mix(theme::YELLOW, travel));
    if travel > 0.0 {
        ui.glow(knob, SWITCH_KNOB / 2.0, theme::YELLOW.alpha(travel));
    }
}

/// Moldura de um campo de texto. O texto em si é de um `EDIT` nativo, que a
/// janela cria e posiciona a partir de [`Ui::edit`] (R14).
///
/// Com `placeholder`, o campo está vazio e sem foco: aí nem existe filho nativo
/// (ele pintaria o próprio fundo por cima da dica) e o texto de apoio é
/// desenhado em D2D como no legado. O clique na moldura é que traz o `EDIT`.
pub fn edit_host(ui: &mut Ui, id: Id, rect: Rect, focused: bool, placeholder: Option<&str>) {
    ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
    ui.stroke(
        rect,
        theme::RADIUS_BUTTON,
        2.0,
        if focused {
            theme::CYAN.alpha(0.5)
        } else {
            theme::BORDER
        },
    );

    let inner = rect.inset_xy(14.0, 10.0);
    match placeholder {
        Some(hint) => ui.text(
            inner,
            hint.to_uppercase(),
            TextStyle::new(font::SIZE_BODY, Weight::Black)
                .tracking(font::TRACKING_LABEL)
                .middle(),
            theme::TEXT_DIM,
        ),
        None => ui.edit(id, inner),
    }
    ui.hit(id, rect);
}

/// Bolinha de status com brilho — verde quando ativo, vermelha quando não.
pub fn status_dot(ui: &mut Ui, rect: Rect, on: bool) {
    let color = if on { theme::GREEN } else { theme::RED };
    ui.ellipse(rect, color);
    ui.glow(rect, rect.w / 2.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::{id, Input, Visual};

    /// Medidor de largura fixa: o suficiente para conferir o layout das abas.
    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, _max: f32) -> (f32, f32) {
            (text.chars().count() as f32 * style.size * 0.6, style.size)
        }
    }

    const TABS: [Tab<'static>; 3] = [
        Tab {
            label: "Macros",
            accent: theme::CYAN,
        },
        Tab {
            label: "Builds",
            accent: theme::CYAN,
        },
        Tab {
            label: "Config",
            accent: theme::YELLOW,
        },
    ];

    fn header(ui: &mut Ui, active: usize) {
        ui.begin(0);
        tab_bar(
            ui,
            &mut Fixed,
            Rect::new(0.0, 0.0, 820.0, TAB_BAR_HEIGHT),
            &TABS,
            active,
        );
        ui.end();
    }

    #[test]
    fn tabs_are_centered_and_clickable() {
        let mut ui = Ui::new();
        header(&mut ui, 0);

        // Cada aba responde no seu pedaço, e a barra fica centralizada.
        let first = ui.frame().hit_at(300.0, TAB_BAR_HEIGHT / 2.0);
        assert_eq!(first, Some(tab_id(0)));
        let last = ui.frame().hit_at(520.0, TAB_BAR_HEIGHT / 2.0);
        assert_eq!(last, Some(tab_id(2)));

        // Fora da faixa das abas não há clique.
        assert_eq!(ui.frame().hit_at(10.0, TAB_BAR_HEIGHT / 2.0), None);
        assert_eq!(ui.frame().hit_at(300.0, TAB_BAR_HEIGHT + 5.0), None);
    }

    #[test]
    fn the_active_tab_paints_the_accent_bar() {
        let mut ui = Ui::new();
        header(&mut ui, 2);

        let bars: Vec<&Visual> = ui
            .frame()
            .nodes
            .iter()
            .filter(|node| node.rect.h == TAB_ACCENT_HEIGHT)
            .map(|node| &node.visual)
            .collect();
        assert_eq!(bars.len(), 1, "só a aba ativa tem barra de acento");
        assert_eq!(
            bars[0],
            &Visual::Fill {
                radius: 0.0,
                color: theme::YELLOW
            },
            "a aba de configurações usa o acento amarelo"
        );
    }

    #[test]
    fn hovering_an_inactive_tab_starts_a_fade() {
        let mut ui = Ui::new();
        header(&mut ui, 0);
        assert!(!ui.animating());

        let response = ui.input(Input::Move {
            x: 400.0,
            y: TAB_BAR_HEIGHT / 2.0,
        });
        assert!(response.redraw);

        header(&mut ui, 0);
        assert!(ui.animating(), "o fade de hover pede o timer de 16ms");
    }

    #[test]
    fn a_card_reserves_padding_and_header() {
        let mut ui = Ui::new();
        ui.begin(0);
        let plain = card(&mut ui, Rect::new(0.0, 0.0, 400.0, 200.0), None);
        assert_eq!(plain, Rect::new(24.0, 24.0, 352.0, 152.0));

        let with_header = card(
            &mut ui,
            Rect::new(0.0, 0.0, 400.0, 200.0),
            Some(CardHeader {
                title: "Atalhos",
                accent: theme::YELLOW,
            }),
        );
        assert!(with_header.y > plain.y + CARD_HEADER_HEIGHT);
        assert_eq!(with_header.x, plain.x);
        ui.end();
    }

    #[test]
    fn a_pressed_button_shrinks_and_still_answers() {
        let mut ui = Ui::new();
        let rect = Rect::new(0.0, 0.0, 200.0, CONTROL_HEIGHT);

        ui.begin(0);
        button(
            &mut ui,
            id("go"),
            rect,
            "Gerar",
            ButtonVariant::Primary,
            theme::CYAN,
        );
        ui.end();
        assert_eq!(ui.frame().nodes[0].rect, rect);

        ui.input(Input::Down { x: 10.0, y: 10.0 });
        ui.begin(0);
        button(
            &mut ui,
            id("go"),
            rect,
            "Gerar",
            ButtonVariant::Primary,
            theme::CYAN,
        );
        ui.end();
        assert_eq!(ui.frame().nodes[0].rect, rect.inset(BUTTON_PRESS_INSET));
        assert_eq!(ui.frame().hit_at(10.0, 10.0), Some(id("go")));

        assert_eq!(
            ui.input(Input::Up { x: 10.0, y: 10.0 }).clicked,
            Some(id("go"))
        );
    }

    #[test]
    fn the_switch_knob_slides_from_one_end_to_the_other() {
        let mut ui = Ui::new();
        let rect = Rect::new(0.0, 0.0, 300.0, 40.0);

        let knob_x = |ui: &Ui| {
            ui.frame()
                .nodes
                .iter()
                .find(|node| matches!(node.visual, Visual::Ellipse { .. }))
                .expect("knob")
                .rect
                .x
        };

        // Com o tempo parado o fade não anda: o estado inicial é o "desligado".
        ui.begin(0);
        toggle_row(&mut ui, id("hud"), rect, "HUD", "desc", false);
        ui.end();
        let off = knob_x(&ui);

        // Ligado e com tempo suficiente para o fade terminar.
        ui.begin(0);
        toggle_row(&mut ui, id("hud"), rect, "HUD", "desc", true);
        ui.end();
        ui.begin(1_000);
        toggle_row(&mut ui, id("hud"), rect, "HUD", "desc", true);
        ui.end();
        assert!(knob_x(&ui) > off);
        assert!(!ui.animating(), "chegou na ponta e parou");
    }

    #[test]
    fn an_edit_host_publishes_its_rect_for_the_native_child() {
        let mut ui = Ui::new();
        let rect = Rect::new(10.0, 10.0, 300.0, CONTROL_HEIGHT);

        ui.begin(0);
        edit_host(&mut ui, id("search"), rect, true, None);
        ui.end();

        let edits = ui.frame().edits();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].id, id("search"));
        assert!(edits[0].rect.w < 300.0, "o texto entra com folga da borda");
        assert_eq!(ui.frame().hit_at(20.0, 20.0), Some(id("search")));
    }

    #[test]
    fn an_empty_unfocused_field_shows_the_hint_and_no_native_child() {
        let mut ui = Ui::new();
        ui.begin(0);
        edit_host(
            &mut ui,
            id("search"),
            Rect::new(0.0, 0.0, 300.0, CONTROL_HEIGHT),
            false,
            Some("Buscar estratagema..."),
        );
        ui.end();

        assert!(ui.frame().edits().is_empty());
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            &node.visual,
            Visual::Text { text, .. } if text.contains("BUSCAR")
        )));
    }
}
