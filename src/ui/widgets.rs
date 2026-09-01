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

use crate::data::{Dir, Stratagem, SupportStrat};
use crate::shared::FlashKind;
use crate::ui::theme::{self, font, Color};
use crate::ui::toolkit::{id_at, Align, Id, ImageStyle, Measure, Rect, TextStyle, Ui, Weight};

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

/// Altura que o card consome fora do conteúdo — o que uma tela precisa somar
/// para saber de quanto card ela precisa.
pub fn card_chrome(header: bool) -> f32 {
    CARD_PADDING * 2.0
        + if header {
            CARD_HEADER_HEIGHT + CARD_HEADER_GAP
        } else {
            0.0
        }
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

/// Borda dos botões de escolha e de captura (`border-2` do legado).
const CHOICE_BORDER: f32 = 2.0;
/// Período do `animate-pulse-hd`, usado pelo botão que espera uma tecla.
const PULSE_MS: u32 = 1_400;

/// Botão de uma escolha exclusiva (modificador in-game, velocidade, idioma):
/// amarelo sólido quando é o escolhido, escuro com borda quando não.
pub fn choice_button(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    let rect = if ui.is_pressed(id) {
        rect.inset(BUTTON_PRESS_INSET)
    } else {
        rect
    };
    let style = TextStyle::new(font::SIZE_LABEL, Weight::Black)
        .tracking(font::TRACKING_LABEL)
        .align(Align::Center)
        .middle();

    if selected {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::YELLOW);
        ui.stroke(
            rect,
            theme::RADIUS_BUTTON,
            CHOICE_BORDER,
            theme::YELLOW.mix(theme::BG_DEEP, 0.25),
        );
        ui.glow(rect, theme::RADIUS_BUTTON, theme::YELLOW);
        ui.text(rect, label.to_uppercase(), style, theme::TEXT_ON_ACCENT);
    } else {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
        if hover > 0.0 {
            ui.fill(
                rect,
                theme::RADIUS_BUTTON,
                theme::YELLOW.alpha(0.05 * hover),
            );
        }
        ui.stroke(
            rect,
            theme::RADIUS_BUTTON,
            CHOICE_BORDER,
            theme::BORDER.mix(theme::YELLOW.alpha(0.5), hover),
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

/// Botão de atalho: mostra a tecla ligada e, enquanto espera uma nova, pulsa em
/// amarelo com o "OUVINDO..." do legado.
pub fn key_button(ui: &mut Ui, id: Id, rect: Rect, label: &str, capturing: bool) {
    let style = TextStyle::new(font::SIZE_BODY, Weight::Black)
        .tracking(font::TRACKING_LABEL)
        .align(Align::Center)
        .middle();

    if capturing {
        // O pulso nunca chega a apagar de vez: a borda continua legível no vale.
        let intensity = 0.45 + 0.55 * ui.pulse(PULSE_MS);
        ui.fill(
            rect,
            theme::RADIUS_BUTTON,
            theme::YELLOW.alpha(0.10 * intensity),
        );
        ui.stroke(
            rect,
            theme::RADIUS_BUTTON,
            CHOICE_BORDER,
            theme::YELLOW.alpha(intensity),
        );
        ui.text(rect, label.to_uppercase(), style, theme::YELLOW);
    } else {
        let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
        ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
        ui.stroke(
            rect,
            theme::RADIUS_BUTTON,
            CHOICE_BORDER,
            theme::BORDER.mix(theme::YELLOW.alpha(0.5), hover),
        );
        ui.text(
            rect,
            label.to_uppercase(),
            style,
            theme::TEXT_HOVER.mix(theme::TEXT, hover),
        );
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

/// Filete de acento na lateral esquerda de um card (`border-l-4` das seções da
/// grade). Desenhado como o card inteiro dentro de um recorte de 4 DIP, o que o
/// faz acompanhar o canto arredondado sem precisar de outra geometria.
pub fn card_left_accent(ui: &mut Ui, rect: Rect, color: Color) {
    ui.push_clip(Rect::new(rect.x, rect.y, LEFT_ACCENT_WIDTH, rect.h));
    ui.fill(rect, theme::RADIUS_CARD, color);
    ui.pop_clip();
}

// --- Setas do codex ---

/// A partir de sete passos as setas encolhem para caber na largura do card
/// (`strat.codex.length > 6` do legado).
const CODEX_LONG: usize = 6;
const CODEX_SIZE: f32 = 16.0;
const CODEX_SIZE_LONG: f32 = 13.0;
const CODEX_GAP: f32 = 6.0;
const CODEX_GAP_LONG: f32 = 4.0;
/// Espessura do traço no quadrado de 24 unidades do ícone original.
const ARROW_STROKE_UNITS: f32 = 2.0;

pub fn codex_size(len: usize) -> f32 {
    if len > CODEX_LONG {
        CODEX_SIZE_LONG
    } else {
        CODEX_SIZE
    }
}

fn codex_gap(len: usize) -> f32 {
    if len > CODEX_LONG {
        CODEX_GAP_LONG
    } else {
        CODEX_GAP
    }
}

/// Largura que a fileira de setas ocupa.
pub fn codex_width(len: usize) -> f32 {
    if len == 0 {
        return 0.0;
    }
    codex_size(len) * len as f32 + codex_gap(len) * (len - 1) as f32
}

/// Uma seta, no traço do ícone da v1 (lucide `ArrowUp` e companhia): haste mais
/// chevron, desenhados num quadrado de 24 unidades com ponta arredondada.
pub fn arrow(ui: &mut Ui, rect: Rect, dir: Dir, color: Color) {
    let point = |u: f32, v: f32| (rect.x + rect.w * u / 24.0, rect.y + rect.h * v / 24.0);
    let width = (rect.w * ARROW_STROKE_UNITS / 24.0).max(1.0);

    // Haste (a, b) e chevron (c → d → e), nas coordenadas do ícone.
    let (a, b, c, d, e) = match dir {
        Dir::Up => (
            (12.0, 20.0),
            (12.0, 4.0),
            (5.0, 11.0),
            (12.0, 4.0),
            (19.0, 11.0),
        ),
        Dir::Down => (
            (12.0, 4.0),
            (12.0, 20.0),
            (5.0, 13.0),
            (12.0, 20.0),
            (19.0, 13.0),
        ),
        Dir::Left => (
            (20.0, 12.0),
            (4.0, 12.0),
            (11.0, 5.0),
            (4.0, 12.0),
            (11.0, 19.0),
        ),
        Dir::Right => (
            (4.0, 12.0),
            (20.0, 12.0),
            (13.0, 5.0),
            (20.0, 12.0),
            (13.0, 19.0),
        ),
    };
    ui.line(point(a.0, a.1), point(b.0, b.1), width, color);
    ui.line(point(c.0, c.1), point(d.0, d.1), width, color);
    ui.line(point(d.0, d.1), point(e.0, e.1), width, color);
}

/// Fileira de setas do codex, centralizada em `rect`.
pub fn arrow_row(ui: &mut Ui, rect: Rect, codex: &[Dir], color: Color) {
    if codex.is_empty() {
        return;
    }
    let size = codex_size(codex.len());
    let gap = codex_gap(codex.len());
    let mut x = rect.x + (rect.w - codex_width(codex.len())) / 2.0;
    let y = rect.y + (rect.h - size) / 2.0;
    for dir in codex {
        arrow(ui, Rect::new(x, y, size, size), *dir, color);
        x += size + gap;
    }
}

// --- Card de estratagema ---

/// Faixa escura do topo, onde o nome é lido sobre o ícone (`p-3 pb-8`).
const CARD_NAME_FADE: f32 = 52.0;
/// Faixa escura do rodapé, atrás das setas (`p-4 pt-12`).
const CARD_CODEX_FADE: f32 = 72.0;
const CARD_BORDER: f32 = 2.0;
/// Ampliação do ícone sob o mouse (`group-hover:scale-110`).
const CARD_ZOOM: f32 = 0.10;
/// Opacidade do ícone parado e sob o mouse (`opacity-70` → `opacity-100`).
const CARD_IMAGE_ALPHA: f32 = 0.70;
/// Tudo no card desabilitado (`opacity-20`).
const CARD_DISABLED_ALPHA: f32 = 0.20;
/// `transition-all duration-500` do ícone — o resto do card usa [`HOVER_MS`].
const CARD_HOVER_MS: u32 = 260;

/// O que muda a leitura de um card na grade (`StratagemCard.jsx`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardState {
    /// Equipado noutro slot ou em conflito de exclusividade: não responde.
    pub disabled: bool,
    /// É o estratagema do slot em edição — clicar de novo o remove.
    pub in_active_slot: bool,
    /// Cor da categoria (primeira tag do estratagema).
    pub accent: Color,
}

/// Card quadrado da grade: ícone sangrando até a borda, nome no topo e o codex
/// no rodapé. Card desabilitado não registra área clicável — sem clique, sem
/// hover e sem cursor de mão, como o `cursor-not-allowed` do legado.
pub fn stratagem_card(ui: &mut Ui, id: Id, rect: Rect, strat: &Stratagem, state: CardState) {
    let hover = if state.disabled {
        0.0
    } else {
        ui.fade(id, ui.is_hot(id), CARD_HOVER_MS)
    };
    let dim = if state.disabled {
        CARD_DISABLED_ALPHA
    } else {
        1.0
    };
    let image_alpha = (CARD_IMAGE_ALPHA + (1.0 - CARD_IMAGE_ALPHA) * hover) * dim;
    icon_card_body(
        ui,
        rect,
        &strat.imagem,
        &strat.nome,
        &strat.codex,
        image_alpha,
        1.0 + CARD_ZOOM * hover,
        dim,
    );

    let border = if state.disabled {
        theme::BORDER.alpha(CARD_DISABLED_ALPHA)
    } else if state.in_active_slot {
        theme::YELLOW.alpha(0.6)
    } else {
        theme::BORDER.alpha(0.5).mix(state.accent.alpha(0.5), hover)
    };
    ui.stroke(rect, theme::RADIUS_CARD, CARD_BORDER, border);
    if state.in_active_slot {
        ui.glow(rect, theme::RADIUS_CARD, theme::YELLOW);
    } else if hover > 0.0 {
        ui.glow(rect, theme::RADIUS_CARD, state.accent.alpha(hover));
    }

    if !state.disabled {
        ui.hit(id, rect);
    }
}

/// Corpo comum dos cards quadrados com ícone: o ícone sangrando até a borda, o
/// nome sobre o gradiente do topo e o codex sobre o do rodapé. `dim` apaga o
/// card inteiro (estado desabilitado da grade); a borda fica com quem chamou.
#[allow(clippy::too_many_arguments)]
fn icon_card_body(
    ui: &mut Ui,
    rect: Rect,
    image: &str,
    name: &str,
    codex: &[Dir],
    image_alpha: f32,
    zoom: f32,
    dim: f32,
) {
    let fade = |color: Color| color.alpha(color.a * dim);

    ui.fill(rect, theme::RADIUS_CARD, theme::CARD_BG);
    ui.image_styled(
        rect,
        format!("icons/{image}"),
        ImageStyle::FILL
            .opacity(image_alpha)
            .rounded(theme::RADIUS_CARD)
            .zoom(zoom),
    );

    // Os dois gradientes terminam na cor do fundo da página, então o que passa
    // dos cantos arredondados se confunde com ela — não precisam de recorte.
    let name_area = rect.with_h(CARD_NAME_FADE);
    ui.gradient(
        name_area,
        0.0,
        fade(theme::BG_DEEP),
        theme::BG_DEEP.alpha(0.0),
    );
    ui.text(
        name_area.inset_xy(10.0, 12.0),
        name.to_uppercase(),
        TextStyle::new(font::SIZE_LABEL, Weight::Black)
            .align(Align::Center)
            .wrap(),
        fade(theme::TEXT),
    );

    let codex_area = Rect::new(
        rect.x,
        rect.bottom() - CARD_CODEX_FADE,
        rect.w,
        CARD_CODEX_FADE,
    );
    ui.gradient(
        codex_area,
        0.0,
        theme::BG_DEEP.alpha(0.0),
        fade(theme::BG_DEEP),
    );
    let size = codex_size(codex.len());
    arrow_row(
        ui,
        Rect::new(rect.x, rect.bottom() - 16.0 - size, rect.w, size),
        codex,
        fade(theme::CYAN),
    );
}

// --- Card de apoio fixo ---

/// Opacidade do ícone de apoio parado (`opacity-80` do legado).
const SUPPORT_IMAGE_ALPHA: f32 = 0.80;

/// Card de um apoio fixo (Reforço, Ressuprimento, Rearme da Águia). Não é
/// clicável — quem responde é o botão de atalho embaixo dele —, mas recebe as
/// mesmas piscadas de disparo e bloqueio dos slots de macro.
pub fn support_card(ui: &mut Ui, index: usize, rect: Rect, support: &SupportStrat) {
    let triggered = ui.anim(
        flash_id(index, true, FlashKind::Triggered),
        FLASH_TRIGGERED_MS,
    );
    let blocked = ui.anim(flash_id(index, true, FlashKind::Blocked), FLASH_BLOCKED_MS);

    icon_card_body(
        ui,
        rect,
        support.imagem,
        support.nome,
        support.codex,
        SUPPORT_IMAGE_ALPHA + (1.0 - SUPPORT_IMAGE_ALPHA) * triggered,
        1.0 + CARD_ZOOM * triggered,
        1.0,
    );

    let (border, glow) = if triggered > 0.0 {
        (theme::YELLOW, Some(theme::YELLOW.alpha(triggered)))
    } else if blocked > 0.0 {
        (theme::RED.alpha(0.7), Some(theme::RED.alpha(0.7 * blocked)))
    } else {
        (theme::BORDER.alpha(0.5), None)
    };
    ui.stroke(rect, theme::RADIUS_CARD, CARD_BORDER, border);
    if let Some(glow) = glow {
        ui.glow(rect, theme::RADIUS_CARD, glow);
    }
}

// --- Slot de macro ---

/// Lado do quadrado de slot (`w-16 h-16`).
pub const SLOT_SIZE: f32 = 64.0;
/// Botão de limpar que aparece no canto sob o mouse (`w-5 h-5`).
const SLOT_CLEAR_SIZE: f32 = 20.0;
/// Traço do quadrado (`border-2`).
const SLOT_BORDER: f32 = 2.0;
/// Faixa do atalho no topo do slot.
const SLOT_TAG_HEIGHT: f32 = 14.0;
const SLOT_TAG_PADDING: f32 = 6.0;
/// Barra que marca o slot em edição (`w-6 h-1`).
const SLOT_ACTIVE_BAR: (f32, f32) = (24.0, 4.0);
/// Filete lateral dos cards de seção.
const LEFT_ACCENT_WIDTH: f32 = 4.0;

/// Duração das piscadas, iguais às da v1 (`Slot.jsx` e `App.jsx`).
pub const FLASH_TRIGGERED_MS: u32 = 500;
pub const FLASH_BLOCKED_MS: u32 = 400;

/// Id do quadrado do slot `index`.
pub fn slot_id(index: usize) -> Id {
    id_at("ui.slot", index)
}

/// Id do × que esvazia o slot `index`.
pub fn slot_clear_id(index: usize) -> Id {
    id_at("ui.slot.clear", index)
}

/// Chave da animação de piscada. Vale para os slots de macro e para os cards de
/// apoio fixo (Fase 6), que recebem os mesmos eventos do engine.
pub fn flash_id(index: usize, support: bool, kind: FlashKind) -> Id {
    let name = match (support, kind) {
        (false, FlashKind::Triggered) => "ui.flash.slot",
        (false, FlashKind::Blocked) => "ui.flash.slot.blocked",
        (true, FlashKind::Triggered) => "ui.flash.support",
        (true, FlashKind::Blocked) => "ui.flash.support.blocked",
    };
    id_at(name, index)
}

/// Quadrado de um slot de macro: ícone do estratagema equipado, atalho no topo,
/// destaque do slot em edição e as duas piscadas (disparo e bloqueio).
pub fn slot_square(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    index: usize,
    rect: Rect,
    strat: Option<&Stratagem>,
    shortcut: &str,
    active: bool,
) {
    let id = slot_id(index);
    let clear = slot_clear_id(index);
    let triggered = ui.anim(
        flash_id(index, false, FlashKind::Triggered),
        FLASH_TRIGGERED_MS,
    );
    let blocked = ui.anim(flash_id(index, false, FlashKind::Blocked), FLASH_BLOCKED_MS);
    let hover = ui.fade(id, ui.is_hot(id) || ui.is_hot(clear), HOVER_MS);

    // Bloqueio: mancha vermelha em volta do quadrado (`-inset-1` da v1).
    if blocked > 0.0 {
        ui.fill(
            rect.inset(-4.0),
            theme::RADIUS_SLOT + 4.0,
            theme::RED.alpha(0.3 * blocked),
        );
    }

    let background = if triggered > 0.0 {
        theme::YELLOW
            .alpha(0.4 * triggered)
            .over(theme::SURFACE_ACTIVE)
    } else if active {
        theme::SURFACE_ACTIVE
    } else {
        theme::CARD_BG
    };
    ui.fill(rect, theme::RADIUS_SLOT, background);

    match strat {
        Some(strat) => {
            ui.image_styled(
                rect,
                format!("icons/{}", strat.imagem),
                ImageStyle::FILL
                    .opacity(0.8)
                    .rounded(theme::RADIUS_SLOT)
                    .zoom(1.0 + CARD_ZOOM * triggered),
            );
            // Escurece o topo para o atalho continuar legível sobre o ícone.
            ui.gradient(
                rect.with_h(40.0),
                0.0,
                theme::BG_DEEP.alpha(0.6),
                theme::BG_DEEP.alpha(0.0),
            );
        }
        None => empty_slot(ui, rect),
    }

    shortcut_tag(ui, measure, rect, shortcut, active);

    let border = if active || triggered > 0.0 {
        theme::YELLOW
    } else {
        theme::BORDER.mix(theme::TEXT_DIM, hover)
    };
    ui.stroke(rect, theme::RADIUS_SLOT, SLOT_BORDER, border);
    if active || triggered > 0.0 {
        ui.glow(rect, theme::RADIUS_SLOT, theme::YELLOW);
        let (width, height) = SLOT_ACTIVE_BAR;
        ui.fill(
            Rect::new(
                rect.center_x() - width / 2.0,
                rect.bottom() - height / 2.0,
                width,
                height,
            ),
            height / 2.0,
            theme::YELLOW,
        );
    }
    ui.hit(id, rect);

    // O × sai por cima e é registrado depois, então ganha o clique na sobreposição.
    if strat.is_some() && hover > 0.0 {
        let button = Rect::new(
            rect.right() - SLOT_CLEAR_SIZE / 2.0,
            rect.y - SLOT_CLEAR_SIZE / 2.0,
            SLOT_CLEAR_SIZE,
            SLOT_CLEAR_SIZE,
        );
        let strong = ui.is_hot(clear);
        ui.ellipse(
            button,
            theme::RED.alpha(if strong { 1.0 } else { 0.8 * hover }),
        );
        ui.text(
            button,
            "×",
            TextStyle::new(font::SIZE_BODY, Weight::Black)
                .align(Align::Center)
                .middle(),
            theme::TEXT.alpha(hover),
        );
        ui.hit(clear, button);
    }
}

/// Slot vazio: o quadrado tracejado e o "OPEN" apagados da v1.
fn empty_slot(ui: &mut Ui, rect: Rect) {
    let mut box_rect = rect.centered(20.0, 32.0);
    let mark = box_rect.cut_top(20.0);
    ui.stroke(
        mark,
        6.0,
        SLOT_BORDER,
        theme::TEXT_DIM.alpha(CARD_DISABLED_ALPHA),
    );
    ui.text(
        Rect::new(rect.x, box_rect.y + 2.0, rect.w, 10.0),
        "OPEN",
        TextStyle::new(7.0, Weight::Black)
            .tracking(font::TRACKING_LABEL)
            .align(Align::Center),
        theme::TEXT_DIM.alpha(CARD_DISABLED_ALPHA),
    );
}

/// Etiqueta do atalho, presa no topo do slot.
fn shortcut_tag(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, shortcut: &str, active: bool) {
    let style = TextStyle::new(font::SIZE_TINY, Weight::Black)
        .align(Align::Center)
        .middle();
    let width = measure.text_size(shortcut, style, f32::INFINITY).0 + SLOT_TAG_PADDING * 2.0;
    let tag = Rect::new(
        rect.center_x() - width / 2.0,
        rect.y + 4.0,
        width,
        SLOT_TAG_HEIGHT,
    );
    ui.fill(
        tag,
        4.0,
        if active {
            theme::YELLOW
        } else {
            theme::BG_DEEP
        },
    );
    ui.stroke(
        tag,
        4.0,
        theme::HAIRLINE_WIDTH,
        if active { theme::YELLOW } else { theme::BORDER },
    );
    ui.text(
        tag,
        shortcut,
        style,
        if active {
            theme::TEXT_ON_ACCENT
        } else {
            theme::TEXT_DIM
        },
    );
}

// --- Dropdown ---

/// Altura de uma linha da lista aberta.
pub const DROPDOWN_ROW: f32 = 28.0;
/// A lista para de crescer aqui e passa a rolar.
pub const DROPDOWN_MAX_HEIGHT: f32 = 240.0;
/// Lado do triângulo que indica o dropdown fechado.
const CHEVRON_SIZE: f32 = 10.0;

/// Campo fechado do dropdown: o valor atual e a seta. É o `<select>` do legado,
/// que aqui não pode ser um controle nativo — a lista precisa do mesmo tema.
pub fn dropdown_field(ui: &mut Ui, id: Id, rect: Rect, value: &str, open: bool) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
    ui.stroke(
        rect,
        theme::RADIUS_BUTTON,
        CHOICE_BORDER,
        if open {
            theme::CYAN.alpha(0.5)
        } else {
            theme::BORDER.mix(theme::CYAN.alpha(0.5), hover)
        },
    );

    let mut inner = rect.inset_xy(12.0, 0.0);
    let chevron = inner
        .cut_right(CHEVRON_SIZE + 4.0)
        .middle_row(CHEVRON_SIZE)
        .with_w(CHEVRON_SIZE);
    chevron_down(ui, chevron, theme::TEXT_DIM.mix(theme::TEXT, hover));
    ui.text(
        inner,
        value.to_uppercase(),
        TextStyle::new(font::SIZE_TINY, Weight::Black).middle(),
        theme::TEXT_DIM.mix(theme::TEXT, hover),
    );
    ui.hit(id, rect);
}

/// Só a ponta da seta, sem haste: o `<select>` do sistema não desenha uma seta
/// inteira, e a do codex ficaria pesada demais neste tamanho.
fn chevron_down(ui: &mut Ui, rect: Rect, color: Color) {
    let width = (rect.w / 6.0).max(1.0);
    let top = rect.y + rect.h * 0.35;
    let bottom = rect.y + rect.h * 0.65;
    ui.line((rect.x, top), (rect.center_x(), bottom), width, color);
    ui.line((rect.center_x(), bottom), (rect.right(), top), width, color);
}

/// Fundo da lista aberta. As linhas e a rolagem ficam com quem chama — só ela
/// sabe quantos itens a categoria tem.
pub fn dropdown_panel(ui: &mut Ui, rect: Rect) {
    ui.fill(rect, theme::RADIUS_BUTTON, theme::BG_DEEP);
    ui.stroke(rect, theme::RADIUS_BUTTON, CHOICE_BORDER, theme::BORDER);
}

/// Uma linha da lista: acesa sob o mouse, amarela quando é a escolhida.
pub fn dropdown_row(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    if hover > 0.0 {
        ui.fill(rect, 6.0, theme::SURFACE_HOVER.alpha(0.8 * hover));
    }
    ui.text(
        rect.inset_xy(10.0, 0.0),
        label.to_uppercase(),
        TextStyle::new(font::SIZE_TINY, Weight::Black).middle(),
        if selected {
            theme::YELLOW
        } else {
            theme::TEXT_DIM.mix(theme::TEXT, hover)
        },
    );
    ui.hit(id, rect);
}

// --- Chips das builds salvas ---

/// Chip de build salva (`py-2.5 px-4`).
pub const CHIP_HEIGHT: f32 = 36.0;
pub const CHIP_GAP: f32 = 8.0;
const CHIP_PADDING: f32 = 16.0;
/// × que exclui a build, no canto do chip.
const CHIP_DELETE_SIZE: f32 = 20.0;

/// Posição de um chip dentro da área que o recebe, em coordenadas relativas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chip {
    pub index: usize,
    x: f32,
    y: f32,
    width: f32,
}

impl Chip {
    /// Retângulo do chip dentro de `area`.
    pub fn rect(self, area: Rect) -> Rect {
        Rect::new(area.x + self.x, area.y + self.y, self.width, CHIP_HEIGHT)
    }
}

/// As fileiras de chips já resolvidas, e a altura que elas ocupam.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChipLayout {
    pub chips: Vec<Chip>,
    pub height: f32,
}

pub fn chip_style() -> TextStyle {
    TextStyle::new(font::SIZE_LABEL, Weight::Black)
        .tracking(font::TRACKING_LABEL)
        .align(Align::Center)
        .middle()
}

/// Onde cada chip cai numa faixa de `width` de largura: os nomes têm tamanhos
/// diferentes e a fileira quebra como o `flex-wrap` do legado.
pub fn chip_layout<'a>(
    measure: &mut dyn Measure,
    names: impl Iterator<Item = &'a str>,
    width: f32,
) -> ChipLayout {
    let style = chip_style();
    let mut chips = Vec::new();
    let (mut x, mut y) = (0.0f32, 0.0f32);

    for (index, name) in names.enumerate() {
        let text = measure
            .text_size(&name.to_uppercase(), style, f32::INFINITY)
            .0;
        let chip_width = (text + CHIP_PADDING * 2.0).min(width.max(1.0));
        if x > 0.0 && x + chip_width > width {
            x = 0.0;
            y += CHIP_HEIGHT + CHIP_GAP;
        }
        chips.push(Chip {
            index,
            x,
            y,
            width: chip_width,
        });
        x += chip_width + CHIP_GAP;
    }

    ChipLayout {
        chips,
        height: y + CHIP_HEIGHT,
    }
}

/// Chip de uma build salva: aplica no clique e, com `delete`, ganha o × que
/// aparece sob o mouse. O overlay usa a mesma peça sem o × — lá a build é só
/// aplicada (a gestão fica na janela principal).
pub fn loadout_chip(ui: &mut Ui, id: Id, delete: Option<Id>, rect: Rect, name: &str, active: bool) {
    let hovered = ui.is_hot(id) || delete.is_some_and(|delete| ui.is_hot(delete));
    let hover = ui.fade(id, hovered, HOVER_MS);
    let name = name.to_uppercase();

    if active {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::YELLOW);
        ui.glow(rect, theme::RADIUS_BUTTON, theme::YELLOW);
        ui.text(rect, name, chip_style(), theme::TEXT_ON_ACCENT);
    } else {
        ui.fill(rect, theme::RADIUS_BUTTON, theme::SURFACE);
        ui.stroke(
            rect,
            theme::RADIUS_BUTTON,
            2.0,
            theme::BORDER.mix(theme::YELLOW.alpha(0.5), hover),
        );
        ui.text(
            rect,
            name,
            chip_style(),
            theme::TEXT_DIM.mix(theme::TEXT, hover),
        );
    }
    ui.hit(id, rect);

    let Some(delete) = delete else {
        return;
    };
    if hover <= 0.0 {
        return;
    }
    let button = Rect::new(
        rect.right() - CHIP_DELETE_SIZE / 2.0,
        rect.y - CHIP_DELETE_SIZE / 2.0,
        CHIP_DELETE_SIZE,
        CHIP_DELETE_SIZE,
    );
    let strong = ui.is_hot(delete);
    ui.ellipse(
        button,
        theme::RED.alpha(if strong { 1.0 } else { 0.8 * hover }),
    );
    ui.text(
        button,
        "×",
        TextStyle::new(font::SIZE_BODY, Weight::Black)
            .align(Align::Center)
            .middle(),
        theme::TEXT.alpha(hover),
    );
    ui.hit(delete, button);
}

// --- Card de item de build ---

/// `p-3` do bloco de texto.
const ITEM_TEXT_PADDING: f32 = 12.0;
/// `min-h-[52px]` do bloco de texto.
const ITEM_TEXT_MIN_HEIGHT: f32 = 52.0;
/// `mt-1` entre nome, subtítulo e descrição.
const ITEM_TEXT_GAP: f32 = 4.0;
/// `w-7 h-7` do botão de cadeado, encostado no canto (`top-1.5 right-1.5`).
const LOCK_SIZE: f32 = 28.0;
const LOCK_INSET: f32 = 6.0;
/// `p-4 pt-8` em volta da imagem: a folga maior no topo é do rótulo.
const ITEM_IMAGE_PADDING: f32 = 16.0;
const ITEM_IMAGE_TOP: f32 = 32.0;

/// Um item da build exibida — estratagema ou equipamento.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemCard<'a> {
    /// Categoria ("ESTRATAGEMA 1", "ARMADURA").
    pub label: &'a str,
    pub name: &'a str,
    /// Caminho relativo a `assets/icons/`; `None` desenha o marcador vazio.
    pub image: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub description: Option<&'a str>,
    /// Etiqueta do canto ("SET").
    pub badge: Option<&'a str>,
    pub locked: bool,
}

fn item_name_style() -> TextStyle {
    TextStyle::new(font::SIZE_LABEL, Weight::Black)
        .align(Align::Center)
        .wrap()
}

fn item_subtitle_style() -> TextStyle {
    TextStyle::new(font::SIZE_TINY, Weight::Black)
        .align(Align::Center)
        .wrap()
}

fn item_description_style() -> TextStyle {
    TextStyle::new(font::SIZE_TINY, Weight::Regular)
        .align(Align::Center)
        .wrap()
}

/// Altura do bloco de texto do card, que depende do que o item tem a dizer
/// (a ficha da armadura e a descrição do booster ocupam várias linhas).
pub fn item_card_text_height(measure: &mut dyn Measure, card: &ItemCard, width: f32) -> f32 {
    let inner = (width - ITEM_TEXT_PADDING * 2.0).max(1.0);
    let mut height = measure
        .text_size(&card.name.to_uppercase(), item_name_style(), inner)
        .1;
    if let Some(subtitle) = card.subtitle {
        height += ITEM_TEXT_GAP
            + measure
                .text_size(&subtitle.to_uppercase(), item_subtitle_style(), inner)
                .1;
    }
    if let Some(description) = card.description {
        height += ITEM_TEXT_GAP
            + measure
                .text_size(description, item_description_style(), inner)
                .1;
    }
    (height + ITEM_TEXT_PADDING).max(ITEM_TEXT_MIN_HEIGHT)
}

/// Altura total do card: a imagem é quadrada e o texto vem embaixo.
pub fn item_card_height(measure: &mut dyn Measure, card: &ItemCard, width: f32) -> f32 {
    width + item_card_text_height(measure, card, width)
}

/// Card de um item da build, com o cadeado que o mantém no próximo sorteio.
pub fn build_item_card(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    lock: Id,
    rect: Rect,
    card: &ItemCard,
) {
    ui.fill(rect, theme::RADIUS_CARD, theme::CARD_BG);

    // Imagem quadrada no topo, com proporção preservada: os renders de arma são
    // bem mais largos que altos e esticá-los deformaria a silhueta.
    let square = Rect::new(rect.x, rect.y, rect.w, rect.w.min(rect.h));
    let picture = Rect::new(
        square.x + ITEM_IMAGE_PADDING,
        square.y + ITEM_IMAGE_TOP,
        (square.w - ITEM_IMAGE_PADDING * 2.0).max(0.0),
        (square.h - ITEM_IMAGE_TOP - ITEM_IMAGE_PADDING).max(0.0),
    );
    match card.image {
        Some(path) => ui.image_styled(picture, format!("icons/{path}"), ImageStyle::FILL.contain()),
        // O mesmo marcador da v1 para item vazio (e para os ícones vetoriais,
        // que o decodificador não lê).
        None => ui.text(
            picture,
            "\u{25A3}",
            TextStyle::new(28.0, Weight::Regular)
                .align(Align::Center)
                .middle(),
            theme::BORDER,
        ),
    }

    item_card_text(ui, measure, rect, square.bottom(), card);
    item_card_header(ui, rect, card);

    let border = if card.locked {
        theme::YELLOW.alpha(0.6)
    } else {
        theme::BORDER.alpha(0.5)
    };
    ui.stroke(rect, theme::RADIUS_CARD, CARD_BORDER, border);
    if card.locked {
        ui.glow(rect, theme::RADIUS_CARD, theme::YELLOW);
    }
    lock_button(ui, lock, rect, card.locked);
}

fn item_card_text(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, top: f32, card: &ItemCard) {
    let mut text = Rect::new(
        rect.x + ITEM_TEXT_PADDING,
        top,
        (rect.w - ITEM_TEXT_PADDING * 2.0).max(0.0),
        (rect.bottom() - top).max(0.0),
    );
    let name = card.name.to_uppercase();
    let height = measure.text_size(&name, item_name_style(), text.w).1;
    ui.text(text.cut_top(height), name, item_name_style(), theme::TEXT);

    if let Some(subtitle) = card.subtitle {
        text.skip_top(ITEM_TEXT_GAP);
        let subtitle = subtitle.to_uppercase();
        let height = measure
            .text_size(&subtitle, item_subtitle_style(), text.w)
            .1;
        ui.text(
            text.cut_top(height),
            subtitle,
            item_subtitle_style(),
            theme::CYAN.alpha(0.8),
        );
    }
    if let Some(description) = card.description {
        text.skip_top(ITEM_TEXT_GAP);
        let height = measure
            .text_size(description, item_description_style(), text.w)
            .1;
        ui.text(
            text.cut_top(height),
            description,
            item_description_style(),
            theme::TEXT_DIM,
        );
    }
}

/// Faixa do topo: categoria em ciano e, quando o item fecha o set, a etiqueta.
fn item_card_header(ui: &mut Ui, rect: Rect, card: &ItemCard) {
    let row = Rect::new(rect.x + 10.0, rect.y + 8.0, rect.w - 20.0, 12.0);
    let style = TextStyle::new(8.0, Weight::Black)
        .tracking(font::TRACKING_WIDE)
        .middle();
    ui.text(
        row,
        card.label.to_uppercase(),
        style,
        theme::CYAN.alpha(0.9),
    );

    let Some(badge) = card.badge else {
        return;
    };
    // A etiqueta fica à direita, entre o texto da categoria e o cadeado.
    let chip = Rect::new(
        rect.right() - LOCK_SIZE - LOCK_INSET - 34.0,
        rect.y + 8.0,
        30.0,
        12.0,
    );
    ui.fill(chip, 3.0, theme::YELLOW);
    ui.text(
        chip,
        badge.to_uppercase(),
        TextStyle::new(7.0, Weight::Black)
            .align(Align::Center)
            .middle(),
        theme::TEXT_ON_ACCENT,
    );
}

/// Cadeado do canto: amarelo travado, escuro destravado.
fn lock_button(ui: &mut Ui, id: Id, rect: Rect, locked: bool) {
    let hover = ui.fade(id, ui.is_hot(id), HOVER_MS);
    let button = Rect::new(
        rect.right() - LOCK_SIZE - LOCK_INSET,
        rect.y + LOCK_INSET,
        LOCK_SIZE,
        LOCK_SIZE,
    );
    ui.fill(
        button,
        8.0,
        if locked {
            theme::YELLOW
        } else {
            theme::SURFACE.over(theme::CARD_BG)
        },
    );
    ui.stroke(
        button,
        8.0,
        theme::HAIRLINE_WIDTH,
        if locked {
            theme::YELLOW.mix(theme::BG_DEEP, 0.25)
        } else {
            theme::BORDER.mix(theme::TEXT_DIM, hover)
        },
    );
    ui.text(
        button,
        if locked { "\u{1F512}" } else { "\u{1F513}" },
        TextStyle::new(font::SIZE_BODY, Weight::Regular)
            .align(Align::Center)
            .middle(),
        if locked {
            theme::TEXT_ON_ACCENT
        } else {
            theme::TEXT_DIM.mix(theme::TEXT, hover)
        },
    );
    ui.hit(id, button);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::{id, Input, Visual};

    /// Medidor de largura fixa, com quebra de linha grosseira: o suficiente para
    /// conferir o layout das abas e a altura dos cards de item.
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

    fn stratagem(codex: &[Dir]) -> Stratagem {
        Stratagem {
            id: 7,
            nome: "Orbital Precision Strike".into(),
            imagem: "stratagems/Orbital.webp".into(),
            tag: vec!["Offensive".into()],
            codex: codex.to_vec(),
        }
    }

    fn lines(ui: &Ui) -> Vec<&Visual> {
        ui.frame()
            .nodes
            .iter()
            .filter(|node| matches!(node.visual, Visual::Line { .. }))
            .map(|node| &node.visual)
            .collect()
    }

    #[test]
    fn every_arrow_is_a_stem_plus_a_chevron() {
        let mut ui = Ui::new();
        ui.begin(0);
        arrow(
            &mut ui,
            Rect::new(0.0, 0.0, 24.0, 24.0),
            Dir::Up,
            theme::CYAN,
        );
        ui.end();

        // Haste de baixo para cima e as duas metades do chevron, nas mesmas
        // coordenadas do ícone da v1.
        assert_eq!(lines(&ui).len(), 3);
        assert_eq!(
            ui.frame().nodes[0].visual,
            Visual::Line {
                from: (12.0, 20.0),
                to: (12.0, 4.0),
                width: 2.0,
                color: theme::CYAN
            }
        );
    }

    #[test]
    fn a_long_codex_uses_smaller_arrows_and_stays_centered() {
        let short = [Dir::Up, Dir::Down];
        let long = [Dir::Up; 9];
        assert_eq!(codex_size(short.len()), CODEX_SIZE);
        assert_eq!(codex_size(long.len()), CODEX_SIZE_LONG);
        assert!(codex_width(long.len()) > codex_width(short.len()));

        let row = Rect::new(0.0, 0.0, 200.0, 16.0);
        let mut ui = Ui::new();
        ui.begin(0);
        arrow_row(&mut ui, row, &short, theme::CYAN);
        ui.end();

        assert_eq!(lines(&ui).len(), 6, "duas setas");
        let left = ui
            .frame()
            .nodes
            .iter()
            .fold(f32::MAX, |acc, node| acc.min(node.rect.x));
        let right = ui
            .frame()
            .nodes
            .iter()
            .fold(f32::MIN, |acc, node| acc.max(node.rect.right()));
        assert!(
            ((left + right) / 2.0 - row.center_x()).abs() <= 1.0,
            "a fileira fica centralizada no retângulo"
        );
    }

    #[test]
    fn a_disabled_card_answers_neither_click_nor_hover() {
        let rect = Rect::new(0.0, 0.0, 174.0, 174.0);
        let strat = stratagem(&[Dir::Up, Dir::Down]);
        let mut ui = Ui::new();

        ui.begin(0);
        stratagem_card(
            &mut ui,
            id("card"),
            rect,
            &strat,
            CardState {
                disabled: true,
                in_active_slot: false,
                accent: theme::RED,
            },
        );
        ui.end();
        assert_eq!(ui.frame().hit_at(10.0, 10.0), None);

        ui.begin(0);
        stratagem_card(
            &mut ui,
            id("card"),
            rect,
            &strat,
            CardState {
                disabled: false,
                in_active_slot: false,
                accent: theme::RED,
            },
        );
        ui.end();
        assert_eq!(ui.frame().hit_at(10.0, 10.0), Some(id("card")));
    }

    #[test]
    fn the_card_of_the_slot_being_edited_is_outlined_in_yellow() {
        let rect = Rect::new(0.0, 0.0, 174.0, 174.0);
        let strat = stratagem(&[Dir::Up]);
        let mut ui = Ui::new();

        ui.begin(0);
        stratagem_card(
            &mut ui,
            id("card"),
            rect,
            &strat,
            CardState {
                disabled: false,
                in_active_slot: true,
                accent: theme::RED,
            },
        );
        ui.end();

        assert!(ui.frame().nodes.iter().any(|node| matches!(
            node.visual,
            Visual::Stroke { width, color, .. } if width == CARD_BORDER && color == theme::YELLOW.alpha(0.6)
        )));
    }

    fn slot(ui: &mut Ui, now: u64, strat: Option<&Stratagem>, active: bool) {
        ui.begin(now);
        slot_square(
            ui,
            &mut Fixed,
            0,
            Rect::new(0.0, 0.0, SLOT_SIZE, SLOT_SIZE),
            strat,
            "F1",
            active,
        );
        ui.end();
    }

    #[test]
    fn a_filled_slot_offers_the_clear_button_only_under_the_mouse() {
        let strat = stratagem(&[Dir::Up]);
        let mut ui = Ui::new();

        slot(&mut ui, 0, Some(&strat), false);
        // Ponto no canto superior direito, dentro do quadrado e do × que o cobre.
        let corner = (SLOT_SIZE - 4.0, 4.0);
        assert_eq!(
            ui.frame().hit_at(corner.0, corner.1),
            Some(slot_id(0)),
            "sem o mouse em cima o × não existe"
        );

        // O fade de hover precisa de tempo para abrir.
        ui.input(Input::Move { x: 30.0, y: 30.0 });
        slot(&mut ui, 0, Some(&strat), false);
        slot(&mut ui, 1_000, Some(&strat), false);
        assert_eq!(
            ui.frame().hit_at(corner.0, corner.1),
            Some(slot_clear_id(0))
        );

        // Slot vazio nunca mostra o botão.
        slot(&mut ui, 2_000, None, false);
        assert_eq!(ui.frame().hit_at(corner.0, corner.1), Some(slot_id(0)));
    }

    #[test]
    fn a_triggered_slot_flashes_and_goes_back_to_rest() {
        let strat = stratagem(&[Dir::Up]);
        let mut ui = Ui::new();
        slot(&mut ui, 0, Some(&strat), false);

        ui.flash(flash_id(0, false, FlashKind::Triggered), FLASH_TRIGGERED_MS);
        slot(&mut ui, 0, Some(&strat), false);
        assert!(ui.animating(), "a piscada mantém o timer vivo");
        let flashing = ui.frame().nodes.len();

        slot(&mut ui, FLASH_TRIGGERED_MS as u64 * 2, Some(&strat), false);
        assert!(!ui.animating());
        assert!(
            ui.frame().nodes.len() < flashing,
            "sem a piscada some a barra amarela do destaque"
        );
    }

    #[test]
    fn flash_keys_never_collide_between_slots_supports_and_kinds() {
        let keys = [
            flash_id(0, false, FlashKind::Triggered),
            flash_id(0, false, FlashKind::Blocked),
            flash_id(0, true, FlashKind::Triggered),
            flash_id(1, false, FlashKind::Triggered),
            slot_id(0),
            slot_clear_id(0),
        ];
        for (i, a) in keys.iter().enumerate() {
            for b in &keys[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn a_dropdown_field_and_its_rows_answer_the_mouse() {
        let mut ui = Ui::new();
        let field = Rect::new(0.0, 0.0, 200.0, CONTROL_HEIGHT);
        let list = Rect::new(0.0, 40.0, 200.0, DROPDOWN_ROW * 3.0);

        ui.begin(0);
        dropdown_field(&mut ui, id("armor"), field, "A-35 Recon", true);
        dropdown_panel(&mut ui, list);
        for index in 0..3 {
            let row = Rect::new(
                list.x,
                list.y + DROPDOWN_ROW * index as f32,
                list.w,
                DROPDOWN_ROW,
            );
            dropdown_row(&mut ui, id_at("armor.row", index), row, "Item", index == 1);
        }
        ui.end();

        assert_eq!(ui.frame().hit_at(10.0, 10.0), Some(id("armor")));
        assert_eq!(
            ui.frame().hit_at(10.0, 40.0 + DROPDOWN_ROW * 1.5),
            Some(id_at("armor.row", 1))
        );
        // A escolhida sai em amarelo, e só ela.
        let yellow = ui
            .frame()
            .nodes
            .iter()
            .filter(|node| {
                matches!(&node.visual, Visual::Text { color, .. } if *color == theme::YELLOW)
            })
            .count();
        assert_eq!(yellow, 1);
    }

    #[test]
    fn an_item_card_grows_with_what_it_has_to_say() {
        let plain = ItemCard {
            label: "Estratagema 1",
            name: "Orbital Precision Strike",
            image: None,
            subtitle: None,
            description: None,
            badge: None,
            locked: false,
        };
        let detailed = ItemCard {
            subtitle: Some("Média · ARM 100 · VEL 500 · STA 100"),
            description: Some(
                "Acclimated: Provides 50% resistance to fire, gas, acid, and electrical damage.",
            ),
            ..plain
        };

        let width = 180.0;
        let short = item_card_height(&mut Fixed, &plain, width);
        let tall = item_card_height(&mut Fixed, &detailed, width);
        assert!(
            short >= width + ITEM_TEXT_MIN_HEIGHT,
            "a altura mínima vale"
        );
        assert!(tall > short, "a ficha da armadura empurra o card");
    }

    #[test]
    fn a_locked_item_card_is_outlined_and_the_lock_takes_the_click() {
        let card = ItemCard {
            label: "Armadura",
            name: "A-35 Recon",
            image: Some("equipment/armor-a-35-recon.webp"),
            subtitle: None,
            description: None,
            badge: Some("SET"),
            locked: true,
        };
        let rect = Rect::new(0.0, 0.0, 180.0, 240.0);
        let mut ui = Ui::new();

        ui.begin(0);
        build_item_card(&mut ui, &mut Fixed, id("lock"), rect, &card);
        ui.end();

        // O cadeado fica no canto superior direito e é a única área clicável.
        assert_eq!(
            ui.frame().hit_at(rect.right() - 20.0, 20.0),
            Some(id("lock"))
        );
        assert_eq!(ui.frame().hit_at(rect.center_x(), rect.center_y()), None);
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            node.visual,
            Visual::Stroke { width, color, .. } if width == CARD_BORDER && color == theme::YELLOW.alpha(0.6)
        )));
        // A etiqueta de set aparece com o texto do tema.
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            &node.visual,
            Visual::Text { text, .. } if text == "SET"
        )));
    }

    #[test]
    fn an_item_card_without_an_image_draws_the_empty_marker() {
        let card = ItemCard {
            label: "Booster",
            name: "—",
            image: None,
            subtitle: None,
            description: None,
            badge: None,
            locked: false,
        };
        let mut ui = Ui::new();
        ui.begin(0);
        build_item_card(
            &mut ui,
            &mut Fixed,
            id("lock"),
            Rect::new(0.0, 0.0, 180.0, 240.0),
            &card,
        );
        ui.end();

        assert!(!ui
            .frame()
            .nodes
            .iter()
            .any(|node| matches!(node.visual, Visual::Image { .. })));
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            &node.visual,
            Visual::Text { text, .. } if text == "\u{25A3}"
        )));
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
