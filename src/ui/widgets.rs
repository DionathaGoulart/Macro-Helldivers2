//! As peças da interface na skin `retro` do styleguide (§6): topbar, painel com
//! barra de título, botões, campos, toggle, tile de estratagema, slot, chips,
//! dropdown, banner e toast.
//!
//! Cada função só empurra nós e áreas clicáveis no [`Ui`]; quem interpreta o
//! clique é a tela, comparando o id devolvido por
//! [`Ui::input`](crate::ui::toolkit::Ui::input) com os ids que estas funções
//! expõem. Assim nenhum widget precisa de callback, e a construção continua
//! sendo uma função pura do estado.
//!
//! Regra de ouro herdada do guia: **nenhum widget ramifica por tema**. Toda cor
//! sai de [`theme::palette()`] pelo papel (`base_300` é moldura, `accent` é
//! fill de ênfase), e a geometria é a mesma nos dois temas: moldura de 2px,
//! sombra dura deslocada, zero raio.

use crate::data::{Dir, Stratagem, SupportStrat};
use crate::shared::FlashKind;
use crate::ui::theme::{self, font, motion, Color, Status};
use crate::ui::toolkit::{id, id_at, Align, Id, ImageStyle, Measure, Rect, TextStyle, Ui, Weight};

/// Estilos de texto do guia (§3). Uma família só; o que muda é peso, tamanho,
/// caixa e espaçamento.
pub mod styles {
    use crate::ui::theme::font;
    use crate::ui::toolkit::{Align, TextStyle, Weight};

    /// Micro-texto de máquina: status, meta, rótulo de lista (`text-[10px]
    /// uppercase tracking-[0.2em]`).
    pub fn micro() -> TextStyle {
        TextStyle::new(font::SIZE_MICRO, Weight::Bold).tracking(font::TRACKING_MICRO)
    }

    /// Micro-texto pesado: cabeçalho de tabela, título de toast.
    pub fn micro_black() -> TextStyle {
        TextStyle::new(font::SIZE_MICRO, Weight::Black).tracking(font::TRACKING_MICRO)
    }

    /// Rótulo de formulário (`text-xs font-bold uppercase tracking-widest`).
    pub fn label() -> TextStyle {
        TextStyle::new(font::SIZE_LABEL, Weight::Bold).tracking(font::TRACKING_WIDEST)
    }

    /// Corpo de texto.
    pub fn body() -> TextStyle {
        TextStyle::new(font::SIZE_BODY, Weight::Regular)
    }

    /// Descrição de ajuda sob um rótulo (`text-xs`, apagada).
    pub fn hint() -> TextStyle {
        TextStyle::new(font::SIZE_LABEL, Weight::Regular).wrap()
    }

    /// Texto de botão grande: peso 900, caixa alta, `0.05em`.
    pub fn button() -> TextStyle {
        TextStyle::new(font::SIZE_BODY, Weight::Black)
            .tracking(font::TRACKING_BUTTON)
            .align(Align::Center)
            .middle()
    }

    /// Texto de `icon-btn`: micro-texto preto, `tracking-widest`.
    pub fn icon_button() -> TextStyle {
        TextStyle::new(font::SIZE_MICRO, Weight::Black)
            .tracking(font::TRACKING_WIDEST)
            .align(Align::Center)
            .middle()
    }

    /// Título de tela (`screen-title`): preto, itálico, `tracking-tighter`.
    pub fn title() -> TextStyle {
        TextStyle::new(font::SIZE_TITLE, Weight::Black)
            .italic()
            .tracking(font::TRACKING_TIGHTER)
    }
}

// --- Texto de máquina (§4.5, §4.6, §4.8) ---

/// Kicker e rótulo de seção com o `>` literal: `> ATALHOS`.
pub fn sigil(text: &str) -> String {
    format!("> {}", text.to_uppercase())
}

/// Valor entre colchetes: `[VAZIO]`.
pub fn bracketed(text: &str) -> String {
    format!("[{}]", text.to_uppercase())
}

/// Nome de "arquivo" da barra de título de um painel: `ATALHOS_DE_COMBATE.CFG`.
/// Pontuação some, espaço vira `_`, e a extensão diz o que o painel guarda.
pub fn file_name(title: &str, ext: &str) -> String {
    let mut name = String::new();
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            name.extend(ch.to_uppercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_')
            && !name.is_empty()
            && !name.ends_with('_')
        {
            name.push('_');
        }
    }
    format!("{}.{}", name.trim_end_matches('_'), ext.to_uppercase())
}

/// Texto com o caret `_` piscando no fim (§4.5) — o "carregando" do guia, no
/// lugar de spinner. Reticências do texto original saem: o caret já diz que
/// algo está em curso.
///
/// O caret é um nó à parte, na posição que ele ocupa aceso: apagado, o texto
/// não sai do lugar (um `_` que vira espaço deslocaria o texto centralizado).
pub fn caret_text(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    rect: Rect,
    text: &str,
    style: TextStyle,
    color: Color,
) {
    let on = ui.blink(motion::BLINK_MS);
    let base = text
        .trim_end_matches(['.', '\u{2026}'])
        .trim_end()
        .to_uppercase();
    let full = measure
        .text_size(&format!("{base}_"), style, f32::INFINITY)
        .0;
    let head = measure.text_size(&base, style, f32::INFINITY).0;
    let x = match style.align {
        Align::Start => rect.x,
        Align::Center => rect.center_x() - full / 2.0,
        Align::End => rect.right() - full,
    };
    let start = style.align(Align::Start);
    ui.text(
        Rect::new(x, rect.y, (rect.right() - x).max(full), rect.h),
        base,
        start,
        color,
    );
    if on {
        ui.text(
            Rect::new(x + head, rect.y, full - head + 1.0, rect.h),
            "_",
            start,
            color,
        );
    }
}

// --- Primitivas de caixa (§4.1, §4.2, §4.9) ---

/// Caixa com moldura grossa: fill e borda de 2px na cor da moldura.
fn framed(ui: &mut Ui, rect: Rect, fill: Color) {
    let palette = theme::palette();
    ui.fill(rect, fill);
    ui.stroke(rect, theme::BORDER, palette.base_300);
}

/// Sombra dura deslocada (§4.1): o próprio retângulo, empurrado para a direita
/// e para baixo, sem blur.
fn shadow(ui: &mut Ui, rect: Rect, offset: f32) {
    if offset > 0.0 {
        ui.fill(rect.translate(offset, offset), theme::palette().shadow);
    }
}

/// Hover e clique de um controle.
#[derive(Debug, Clone, Copy)]
struct Touch {
    /// Fade do hover, 0..1.
    hover: f32,
    pressed: bool,
}

fn touch(ui: &mut Ui, id: Id) -> Touch {
    Touch {
        hover: ui.fade(id, ui.is_hot(id), motion::HOVER_MS),
        pressed: ui.is_pressed(id),
    }
}

/// Levantar no hover (§4.9): o clicável sobe 4 DIP e a sombra cresce de `sm`
/// para a padrão; pressionado, ele volta ao lugar. Desenha a sombra e devolve
/// onde a face vai. A área clicável continua no retângulo original — é ela
/// que decide o hover, e ela não pode fugir do mouse.
fn raised(ui: &mut Ui, rect: Rect, touch: Touch, rest: f32) -> Rect {
    if touch.pressed {
        shadow(ui, rect, rest.min(theme::SHADOW_SM));
        return rect;
    }
    let face = rect.translate(0.0, -theme::LIFT * touch.hover);
    shadow(ui, face, rest + (theme::SHADOW - rest) * touch.hover);
    face
}

/// Anel de foco (§4.12): 2px sólidos em accent, afastados 2px do controle.
fn focus_ring(ui: &mut Ui, rect: Rect) {
    ui.stroke(
        rect.inset(-(theme::FOCUS_OFFSET + theme::FOCUS_RING)),
        theme::FOCUS_RING,
        theme::palette().accent,
    );
}

/// Quadrado de status (presença, categoria): fill da cor com a moldura em
/// volta — a moldura carrega o contraste, a cor carrega o significado.
pub fn status_square(ui: &mut Ui, rect: Rect, color: Color) {
    framed(ui, rect, color);
}

/// Os três quadrados de janela (§4.7), alinhados à direita de `row`. Devolve
/// o que sobra de `row` à esquerda deles.
pub fn window_dots(ui: &mut Ui, row: Rect, first: Color) -> Rect {
    let palette = theme::palette();
    let mut row = row;
    let dots = row.cut_right(DOT * 3.0 + DOT_GAP * 2.0).middle_row(DOT);
    for (index, color) in [first, palette.base_300, palette.base_300]
        .into_iter()
        .enumerate()
    {
        ui.fill(
            Rect::new(dots.x + (DOT + DOT_GAP) * index as f32, dots.y, DOT, DOT),
            color,
        );
    }
    row
}

/// Lado de cada quadrado de janela e o espaço entre eles.
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 4.0;

// --- Topbar ---

/// Altura da topbar (marca, abas e o toggle de tema).
pub const TAB_BAR_HEIGHT: f32 = 60.0;
const TOPBAR_PADDING: f32 = 20.0;
/// Quadrado do ícone do app (`avatar-sq`).
const BRAND_ICON: f32 = 32.0;
const BRAND_GAP: f32 = 10.0;
/// Espaço entre a marca e as abas.
const BRAND_TO_NAV: f32 = 28.0;
/// Item de navegação: `px-3 py-2 text-sm font-bold`.
const NAV_HEIGHT: f32 = 32.0;
const NAV_PADDING_X: f32 = 12.0;
const NAV_GAP: f32 = 6.0;
/// O `icon-btn` do toggle de tema.
const TOGGLE_SIZE: f32 = 32.0;

/// Id da aba `index` — o que a tela compara com o clique.
pub fn tab_id(index: usize) -> Id {
    id_at("ui.tab", index)
}

/// Id do toggle de tema da topbar.
pub fn theme_toggle_id() -> Id {
    id("ui.theme")
}

fn nav_style() -> TextStyle {
    TextStyle::new(font::SIZE_BODY, Weight::Bold)
        .tracking(font::TRACKING_WIDE)
        .align(Align::Center)
        .middle()
}

/// Topbar (§6.9): marca à esquerda (ícone, kicker e título pequeno), as abas
/// como itens de navegação e o toggle de tema na ponta direita. Fundo de
/// página com a linha inferior de 2px.
pub fn tab_bar(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, tabs: &[&str], active: usize) {
    let palette = theme::palette();
    ui.fill(rect, palette.base_100);
    ui.fill(
        Rect::new(rect.x, rect.bottom() - theme::BORDER, rect.w, theme::BORDER),
        palette.base_300,
    );

    let mut row = rect.inset_xy(TOPBAR_PADDING, 0.0);
    row.h -= theme::BORDER;

    // Toggle de tema, na ponta direita.
    let toggle = row.cut_right(TOGGLE_SIZE).middle_row(TOGGLE_SIZE);
    icon_btn(ui, theme_toggle_id(), toggle, Glyph::Contrast, Tone::Plain);

    // Marca: o ícone do app como `avatar-sq` e o par kicker + título.
    let icon = row.cut_left(BRAND_ICON).middle_row(BRAND_ICON);
    ui.image(icon, "icons/tray.png", 1.0);
    ui.stroke(icon, theme::BORDER, palette.base_300);
    row.cut_left(BRAND_GAP);
    let title_style = TextStyle::new(font::SIZE_BRAND, Weight::Black)
        .italic()
        .tracking(font::TRACKING_TIGHTER);
    let kicker = sigil("Helldivers 2");
    let brand_w = measure
        .text_size(&kicker, styles::micro(), f32::INFINITY)
        .0
        .max(measure.text_size("MACRO", title_style, f32::INFINITY).0);
    let mut brand = row.cut_left(brand_w).middle_row(38.0);
    ui.text(
        brand.cut_top(14.0),
        kicker,
        styles::micro(),
        palette.accent_text,
    );
    ui.text(brand, "MACRO", title_style, palette.content);
    row.cut_left(BRAND_TO_NAV);

    // Abas.
    let style = nav_style();
    for (index, label) in tabs.iter().enumerate() {
        let label = label.to_uppercase();
        let width = measure.text_size(&label, style, f32::INFINITY).0 + NAV_PADDING_X * 2.0;
        let slot = row.cut_left(width).middle_row(NAV_HEIGHT);
        row.cut_left(NAV_GAP);
        nav_item(ui, tab_id(index), slot, &label, index == active);
    }
}

/// Item de navegação: o ativo inunda de accent com sombra `sm`; os outros
/// ganham o fill de 8% no hover (§6.9).
fn nav_item(ui: &mut Ui, id: Id, rect: Rect, label: &str, active: bool) {
    let palette = theme::palette();
    let hover = ui.fade(id, ui.is_hot(id), motion::HOVER_MS);
    if active {
        shadow(ui, rect, theme::SHADOW_SM);
        framed(ui, rect, palette.accent);
        ui.text(rect, label, nav_style(), palette.accent_content);
    } else {
        if hover > 0.0 {
            ui.fill(rect, palette.hover_fill().faded(hover));
        }
        ui.text(rect, label, nav_style(), palette.content);
    }
    ui.hit(id, rect);
}

// --- Painel (§6.2) ---

/// `panel-body`: padding interno do painel.
pub const CARD_PADDING: f32 = 20.0;
/// Barra de título do painel (`window-bar`, §4.8).
pub const CARD_BAR_HEIGHT: f32 = 36.0;
const CARD_BAR_PADDING: f32 = 12.0;
/// Quadrado de categoria na barra de título.
const MARKER: f32 = 10.0;

/// Barra de título de um painel.
#[derive(Debug, Clone, Copy)]
pub struct CardHeader<'a> {
    /// Título; vira o nome de arquivo da barra (ver [`file_name`]).
    pub title: &'a str,
    /// Extensão do "arquivo": `CFG`, `DB`, `LOG`…
    pub ext: &'a str,
    /// Quadrado de cor antes do nome — a categoria da seção.
    pub marker: Option<Color>,
}

impl<'a> CardHeader<'a> {
    pub fn new(title: &'a str, ext: &'a str) -> CardHeader<'a> {
        CardHeader {
            title,
            ext,
            marker: None,
        }
    }

    pub fn marker(self, color: Color) -> CardHeader<'a> {
        CardHeader {
            marker: Some(color),
            ..self
        }
    }
}

/// Altura que o painel consome fora do conteúdo — o que uma tela precisa somar
/// para saber de quanto painel ela precisa. A sombra fica de fora: ela sai do
/// retângulo, e quem empilha painéis deixa o espaço dela no vão.
pub fn card_chrome(header: bool) -> f32 {
    CARD_PADDING * 2.0 + if header { CARD_BAR_HEIGHT } else { 0.0 }
}

/// Painel (`retro-border bg-base-200 retro-shadow`) com a barra de título
/// opcional. Devolve o retângulo interno, já descontados barra e padding.
pub fn card(ui: &mut Ui, rect: Rect, header: Option<CardHeader<'_>>) -> Rect {
    let palette = theme::palette();
    shadow(ui, rect, theme::SHADOW);
    framed(ui, rect, palette.base_200);

    let mut body = rect;
    if let Some(header) = header {
        let bar = body.cut_top(CARD_BAR_HEIGHT);
        window_bar(ui, bar, header);
    }
    body.inset(CARD_PADDING)
}

/// A barra: fundo de página, linha inferior de 2px, o nome em micro-texto
/// apagado (só o nome — a barra também hospeda botões) e os quadrados de
/// janela à direita.
fn window_bar(ui: &mut Ui, bar: Rect, header: CardHeader<'_>) {
    let palette = theme::palette();
    let inner = bar.inset(theme::BORDER);
    ui.fill(inner, palette.base_100);
    ui.fill(
        Rect::new(bar.x, bar.bottom() - theme::BORDER, bar.w, theme::BORDER),
        palette.base_300,
    );

    let mut row = inner.inset_xy(CARD_BAR_PADDING, 0.0);
    row = window_dots(ui, row, palette.accent);
    if let Some(color) = header.marker {
        let marker = row.cut_left(MARKER).middle_row(MARKER);
        status_square(ui, marker, color);
        row.cut_left(8.0);
    }
    ui.text(
        row,
        file_name(header.title, header.ext),
        styles::micro().middle(),
        palette.muted,
    );
}

/// Faixa livre da barra de título de `card_rect`, à esquerda dos quadrados de
/// janela: é onde a tela encosta os `icon-btn` de ação, da direita para a
/// esquerda.
pub fn card_actions(card_rect: Rect) -> Rect {
    let bar = card_rect.with_h(CARD_BAR_HEIGHT).inset(theme::BORDER);
    let mut row = bar.inset_xy(CARD_BAR_PADDING, 0.0);
    row.cut_right(DOT * 3.0 + DOT_GAP * 2.0 + CARD_BAR_PADDING);
    row.middle_row(ICON_BTN_HEIGHT)
}

/// Caixa aninhada num painel (§6.9, card do índice): moldura sobre o fundo de
/// página, e o fill da superfície no hover quando ela é clicável.
pub fn inset_box(ui: &mut Ui, rect: Rect, hover: f32) {
    let palette = theme::palette();
    framed(ui, rect, palette.base_100.mix(palette.base_200, hover));
}

/// Rótulo de seção (`section-label`): `> NOME` em micro-texto apagado.
pub fn section_label(ui: &mut Ui, rect: Rect, text: &str) {
    ui.text(rect, sigil(text), styles::micro(), theme::palette().muted);
}

/// Cabeçalho de tela (§6.9): `screen-kicker` em accent sobre o
/// `screen-title` grande.
pub fn page_header(ui: &mut Ui, rect: Rect, kicker: &str, title: &str) {
    let palette = theme::palette();
    let mut rect = rect;
    ui.text(
        rect.cut_top(PAGE_KICKER_HEIGHT),
        sigil(kicker),
        styles::micro(),
        palette.accent_text,
    );
    ui.text(rect, title.to_uppercase(), styles::title(), palette.content);
}

/// Altura do cabeçalho de tela: kicker e título.
pub const PAGE_HEADER_HEIGHT: f32 = PAGE_KICKER_HEIGHT + 30.0;
const PAGE_KICKER_HEIGHT: f32 = 16.0;

// --- Botões (§6.1) ---

/// Altura padrão de botão e campo.
pub const CONTROL_HEIGHT: f32 = 40.0;
/// `icon-btn` compacto: `px-3 py-2` em volta do micro-texto.
pub const ICON_BTN_HEIGHT: f32 = 26.0;
const ICON_BTN_PADDING: f32 = 12.0;
/// Botão de fechar/limpar que sai no canto de um slot ou chip.
const CORNER_BUTTON: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// `btn-goodchat`: o CTA, fill accent.
    Primary,
    /// `btn-goodchat-outline`: moldura sobre a superfície, texto de conteúdo.
    Secondary,
    /// `btn-goodchat-danger`: fill de erro. Ação destrutiva.
    Danger,
    /// `:disabled`: o outline a 40%, sem sombra, sem hover e sem clique.
    Disabled,
}

/// Botão grande. Levanta no hover e volta ao lugar quando pressionado.
pub fn button(ui: &mut Ui, id: Id, rect: Rect, label: &str, variant: ButtonVariant) {
    let palette = theme::palette();
    let label = label.to_uppercase();
    if variant == ButtonVariant::Disabled {
        let dim = theme::DISABLED_ALPHA;
        ui.fill(rect, palette.base_200.faded(dim));
        ui.stroke(rect, theme::BORDER, palette.base_300.faded(dim));
        ui.text(rect, label, styles::button(), palette.content.faded(dim));
        return;
    }

    let face = {
        let touched = touch(ui, id);
        raised(ui, rect, touched, theme::SHADOW_SM)
    };
    let (fill, text) = match variant {
        ButtonVariant::Primary => (palette.accent, palette.accent_content),
        ButtonVariant::Danger => (palette.error.fill, palette.error.content),
        _ => (palette.base_200, palette.content),
    };
    framed(ui, face, fill);
    ui.text(face, label, styles::button(), text);
    ui.hit(id, rect);
}

/// O que vai dentro de um `icon-btn`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Glyph<'a> {
    /// Rótulo em micro-texto.
    Text(&'a str),
    /// `×`.
    Close,
    /// Quadrado meio cheio: o toggle de tema.
    Contrast,
    /// Cadeado desenhado, fechado ou aberto.
    Lock(bool),
}

/// Cor de um `icon-btn` em repouso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Superfície; o hover inunda de accent.
    Plain,
    /// Já em accent (estado ligado, ação principal da barra).
    Accent,
    /// Fill de erro (limpar, excluir).
    Danger,
}

/// Largura de um `icon-btn` de texto.
pub fn icon_btn_width(measure: &mut dyn Measure, label: &str) -> f32 {
    measure
        .text_size(&label.to_uppercase(), styles::icon_button(), f32::INFINITY)
        .0
        + ICON_BTN_PADDING * 2.0
}

/// `icon-btn` (§6.1): o botão compacto das barras. Moldura sobre a
/// superfície; no hover ele levanta e inunda de accent.
pub fn icon_btn(ui: &mut Ui, id: Id, rect: Rect, glyph: Glyph<'_>, tone: Tone) {
    let palette = theme::palette();
    let touch = touch(ui, id);
    let face = raised(ui, rect, touch, 0.0);
    let (fill, ink) = match tone {
        Tone::Plain => (
            palette.base_200.mix(palette.accent, touch.hover),
            palette.content.mix(palette.accent_content, touch.hover),
        ),
        Tone::Accent => (palette.accent, palette.accent_content),
        Tone::Danger => (palette.error.fill, palette.error.content),
    };
    framed(ui, face, fill);
    draw_glyph(ui, face, glyph, ink, fill);
    ui.hit(id, rect);
}

fn draw_glyph(ui: &mut Ui, rect: Rect, glyph: Glyph<'_>, ink: Color, fill: Color) {
    match glyph {
        Glyph::Text(label) => ui.text(rect, label.to_uppercase(), styles::icon_button(), ink),
        Glyph::Close => {
            let mark = rect.centered(rect.w * 0.36, rect.h * 0.36);
            let width = (rect.w / 9.0).clamp(1.5, 2.5);
            ui.line((mark.x, mark.y), (mark.right(), mark.bottom()), width, ink);
            ui.line((mark.right(), mark.y), (mark.x, mark.bottom()), width, ink);
        }
        Glyph::Contrast => {
            let mark = rect.centered(rect.w * 0.44, rect.w * 0.44);
            ui.stroke(mark, theme::BORDER, ink);
            ui.fill(mark.with_w(mark.w / 2.0), ink);
        }
        Glyph::Lock(locked) => lock_icon(ui, rect, locked, ink, fill),
    }
}

/// Cadeado em traço reto: corpo cheio e a alça por cima. Aberto, a alça sobe
/// e solta uma das pernas.
fn lock_icon(ui: &mut Ui, rect: Rect, locked: bool, ink: Color, fill: Color) {
    let size = rect.w.min(rect.h) * 0.5;
    let area = rect.centered(size, size);
    let body = Rect::new(area.x, area.y + size * 0.45, size, size * 0.55);
    let shackle_w = size * 0.62;
    let lift = if locked { 0.0 } else { size * 0.18 };
    let shackle = Rect::new(
        area.center_x() - shackle_w / 2.0,
        area.y - lift,
        shackle_w,
        size * 0.5,
    );
    let stroke = (size / 6.0).max(1.5);
    ui.stroke(shackle, stroke, ink);
    if !locked {
        // Perna direita solta: o fill do botão apaga o pé dela.
        ui.fill(
            Rect::new(
                shackle.right() - stroke,
                shackle.y + shackle.h * 0.55,
                stroke,
                shackle.h,
            ),
            fill,
        );
    }
    ui.fill(body, ink);
    // A fechadura: um furo na cor do botão.
    ui.fill(
        Rect::new(
            body.center_x() - stroke / 2.0,
            body.y + body.h * 0.3,
            stroke,
            body.h * 0.4,
        ),
        fill,
    );
}

/// Botão de uma escolha exclusiva (modificador, velocidade, idioma, facção):
/// o escolhido inunda de accent e fica erguido; os outros são outline e
/// levantam no hover.
///
/// Um rótulo `NOME · DETALHE` ("Padrão · 30 fps") vira duas linhas: o nome em
/// cima e o detalhe em micro-texto embaixo — numa linha só, três escolhas
/// lado a lado não cabem na coluna.
pub fn choice_button(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let palette = theme::palette();
    let face = if selected {
        shadow(ui, rect, theme::SHADOW_SM);
        rect
    } else {
        let touched = touch(ui, id);
        raised(ui, rect, touched, 0.0)
    };
    let (fill, ink) = if selected {
        (palette.accent, palette.accent_content)
    } else {
        (palette.base_200, palette.content)
    };
    framed(ui, face, fill);

    let label = label.to_uppercase();
    match label.split_once(" \u{00B7} ") {
        Some((head, tail)) => {
            let mut lines = face.middle_row(CHOICE_SPLIT_TEXT);
            ui.text(
                lines.cut_top(16.0),
                head,
                styles::label().align(Align::Center).middle(),
                ink,
            );
            ui.text(
                lines,
                tail,
                styles::micro().align(Align::Center).middle(),
                if selected { ink } else { palette.muted },
            );
        }
        None => ui.text(
            face,
            label,
            styles::label().align(Align::Center).middle(),
            ink,
        ),
    }
    ui.hit(id, rect);
}

/// Altura do bloco de duas linhas de um rótulo `NOME · DETALHE`, e a do botão
/// que o comporta.
const CHOICE_SPLIT_TEXT: f32 = 30.0;
pub const CHOICE_SPLIT_HEIGHT: f32 = 46.0;

/// Botão de atalho: a tecla ligada em destaque e, enquanto espera uma nova,
/// inunda de accent com o `ESCUTANDO_` do caret piscando.
pub fn key_button(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    id: Id,
    rect: Rect,
    label: &str,
    capturing: bool,
) {
    let palette = theme::palette();
    let style = TextStyle::new(font::SIZE_BODY, Weight::Black)
        .tracking(font::TRACKING_WIDEST)
        .align(Align::Center)
        .middle();
    if capturing {
        shadow(ui, rect, theme::SHADOW_SM);
        framed(ui, rect, palette.accent);
        caret_text(ui, measure, rect, label, style, palette.accent_content);
    } else {
        let face = {
            let touched = touch(ui, id);
            raised(ui, rect, touched, theme::SHADOW_SM)
        };
        framed(ui, face, palette.base_200);
        ui.text(face, label.to_uppercase(), style, palette.content);
    }
    ui.hit(id, rect);
}

// --- Toggle (§6.4) ---

/// Trilho do switch: retângulo de 2.5rem × 1.25rem, nunca pílula.
const SWITCH_WIDTH: f32 = 40.0;
const SWITCH_HEIGHT: f32 = 20.0;
const SWITCH_THUMB: f32 = 12.0;
/// Duração do deslizar do switch.
const SWITCH_MS: u32 = 160;
/// Padding das linhas de toggle.
const TOGGLE_PADDING: f32 = 12.0;

/// Linha "rótulo + descrição + switch", como um card do índice de
/// configuração: caixa aninhada que ganha a superfície no hover. `on` é o
/// estado atual; o clique chega pelo id, e quem muda o settings é a tela.
pub fn toggle_row(ui: &mut Ui, id: Id, rect: Rect, label: &str, desc: &str, on: bool) {
    let palette = theme::palette();
    let hover = ui.fade(id, ui.is_hot(id), motion::HOVER_MS);
    inset_box(ui, rect, hover);

    let mut row = rect.inset_xy(TOGGLE_PADDING, 0.0);
    let switch = row.cut_right(SWITCH_WIDTH).middle_row(SWITCH_HEIGHT);
    row.cut_right(TOGGLE_PADDING);

    let mut text = row.inset_xy(0.0, 8.0);
    let label_row = text.cut_top(16.0);
    ui.text(
        label_row,
        label.to_uppercase(),
        styles::label().middle(),
        palette.content,
    );
    if !desc.is_empty() {
        ui.text(text, desc, styles::hint(), palette.muted);
    }

    switch_track(ui, id, switch, on);
    ui.hit(id, rect);
}

/// Trilho retangular com o polegar quadrado (§6.4): desligado, trilho de
/// página e polegar na cor da moldura; ligado, trilho accent e polegar no
/// conteúdo dele.
fn switch_track(ui: &mut Ui, id: Id, rect: Rect, on: bool) {
    let palette = theme::palette();
    // Chave própria: o fade do hover usa o id puro.
    let travel = ui.fade(id ^ 0x5357, on, SWITCH_MS);
    framed(ui, rect, palette.base_100.mix(palette.accent, travel));

    let padding = (rect.h - SWITCH_THUMB) / 2.0;
    let x = rect.x + padding + (rect.w - SWITCH_THUMB - padding * 2.0) * travel;
    ui.fill(
        Rect::new(x, rect.y + padding, SWITCH_THUMB, SWITCH_THUMB),
        palette.base_300.mix(palette.accent_content, travel),
    );
}

// --- Campo de texto (§6.4) ---

const FIELD_PADDING_X: f32 = 12.0;

/// Moldura de um campo de texto. O texto em si é de um `EDIT` nativo, que a
/// janela cria e posiciona a partir de [`Ui::edit`] (R14).
///
/// Com `placeholder`, o campo está vazio e sem foco: aí nem existe filho nativo
/// (ele pintaria o próprio fundo por cima da dica) e o texto de apoio é
/// desenhado em D2D. O clique na moldura é que traz o `EDIT`.
///
/// `reserve` tira DIP da direita do texto: é onde a tela põe um botão dentro
/// do campo (limpar a busca). O filho nativo é uma janela de verdade e
/// cobriria qualquer coisa desenhada debaixo dele.
pub fn edit_host(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    focused: bool,
    placeholder: Option<&str>,
    reserve: f32,
) {
    let palette = theme::palette();
    framed(ui, rect, palette.base_200);
    if focused {
        focus_ring(ui, rect);
    }

    let mut inner = rect.inset_xy(FIELD_PADDING_X, 10.0);
    inner.w = (inner.w - reserve).max(0.0);
    match placeholder {
        Some(hint) => ui.text(
            inner,
            hint.to_uppercase(),
            styles::label().middle(),
            palette.muted,
        ),
        None => ui.edit(id, inner),
    }
    ui.hit(id, rect);
}

// --- Banner e toast (§6.8) ---

/// Banner de página (`alert`): moldura na cor do status sobre a superfície,
/// título com o `!` na frente e a mensagem embaixo.
pub fn alert(ui: &mut Ui, rect: Rect, status: Status, title: &str, message: &str) {
    let palette = theme::palette();
    ui.fill(rect, palette.base_200);
    ui.stroke(rect, theme::BORDER, status.fill);

    let mut content = rect.inset_xy(14.0, 8.0);
    let title = format!("! {}", title.to_uppercase());
    if message.is_empty() {
        ui.text(content, title, styles::micro_black().middle(), status.text);
        return;
    }
    ui.text(
        content.cut_top(14.0),
        title,
        styles::micro_black(),
        status.text,
    );
    content.skip_top(2.0);
    ui.text(
        content,
        message,
        TextStyle::new(font::SIZE_LABEL, Weight::Regular).wrap(),
        palette.content,
    );
}

/// Largura e altura do toast.
pub const TOAST_WIDTH: f32 = 280.0;
pub const TOAST_HEIGHT: f32 = 74.0;
const TOAST_BAR: f32 = 26.0;

/// Toast (§6.8): painel com a barra de título — o primeiro quadrado de janela
/// na cor do status —, o título em micro-texto e o corpo. `visible` vai de 1 a
/// 0 na saída, e o bloco inteiro apaga junto.
pub fn toast(ui: &mut Ui, rect: Rect, status: Status, title: &str, message: &str, visible: f32) {
    let palette = theme::palette();
    let fade = |color: Color| color.faded(visible);
    ui.fill(
        rect.translate(theme::SHADOW, theme::SHADOW),
        fade(palette.shadow),
    );
    ui.fill(rect, fade(palette.base_200));

    let mut body = rect;
    let bar = body.cut_top(TOAST_BAR);
    ui.fill(bar, fade(palette.base_100));
    ui.fill(
        Rect::new(bar.x, bar.bottom() - theme::BORDER, bar.w, theme::BORDER),
        fade(palette.base_300),
    );
    let mut row = bar.inset_xy(CARD_BAR_PADDING, 0.0);
    let dots = row.cut_right(DOT * 3.0 + DOT_GAP * 2.0).middle_row(DOT);
    for (index, color) in [status.fill, palette.base_300, palette.base_300]
        .into_iter()
        .enumerate()
    {
        ui.fill(
            Rect::new(dots.x + (DOT + DOT_GAP) * index as f32, dots.y, DOT, DOT),
            fade(color),
        );
    }
    ui.text(
        row,
        title.to_uppercase(),
        styles::micro_black().middle(),
        fade(status.text),
    );
    ui.text(
        body.inset_xy(CARD_BAR_PADDING, 10.0),
        message.to_uppercase(),
        styles::label().wrap(),
        fade(palette.content),
    );
    ui.stroke(rect, theme::BORDER, fade(palette.base_300));
}

// --- Setas do codex ---

/// A partir de sete passos as setas encolhem para caber na largura do card.
const CODEX_LONG: usize = 6;
const CODEX_SIZE: f32 = 16.0;
const CODEX_SIZE_LONG: f32 = 13.0;
const CODEX_GAP: f32 = 5.0;
const CODEX_GAP_LONG: f32 = 3.0;
/// Espessura do traço no quadrado de 24 unidades do ícone.
const ARROW_STROKE_UNITS: f32 = 3.0;

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

/// Uma seta: haste mais chevron, desenhados num quadrado de 24 unidades com
/// ponta reta.
pub fn arrow(ui: &mut Ui, rect: Rect, dir: Dir, color: Color) {
    let point = |u: f32, v: f32| (rect.x + rect.w * u / 24.0, rect.y + rect.h * v / 24.0);
    let width = (rect.w * ARROW_STROKE_UNITS / 24.0).max(1.0);

    // Haste (a, b) e chevron (c → d → e), nas coordenadas do ícone.
    let (a, b, c, d, e) = match dir {
        Dir::Up => (
            (12.0, 20.0),
            (12.0, 5.0),
            (5.0, 12.0),
            (12.0, 5.0),
            (19.0, 12.0),
        ),
        Dir::Down => (
            (12.0, 4.0),
            (12.0, 19.0),
            (5.0, 12.0),
            (12.0, 19.0),
            (19.0, 12.0),
        ),
        Dir::Left => (
            (20.0, 12.0),
            (5.0, 12.0),
            (12.0, 5.0),
            (5.0, 12.0),
            (12.0, 19.0),
        ),
        Dir::Right => (
            (4.0, 12.0),
            (19.0, 12.0),
            (12.0, 5.0),
            (19.0, 12.0),
            (12.0, 19.0),
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

// --- Tile de estratagema ---

/// Legenda do tile, embaixo do ícone: nome (até duas linhas) e o codex.
pub const TILE_CAPTION: f32 = 62.0;
const TILE_NAME_HEIGHT: f32 = 28.0;
const TILE_PADDING: f32 = 8.0;

/// Altura de um tile de `width` de largura: o ícone é quadrado e a legenda
/// vem embaixo dele.
pub fn tile_height(width: f32) -> f32 {
    width + TILE_CAPTION
}

/// O que muda a leitura de um tile na grade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardState {
    /// Equipado noutro slot ou em conflito de exclusividade: não responde.
    pub disabled: bool,
    /// É o estratagema do slot em edição — clicar de novo o remove.
    pub in_active_slot: bool,
}

/// Como o corpo de um tile é pintado.
#[derive(Debug, Clone, Copy)]
struct TileLook {
    /// A legenda inunda de accent (selecionado, disparado).
    filled: f32,
    /// Opacidade do bloco inteiro (`:disabled` a 40%).
    alpha: f32,
}

/// Tile da grade: o ícone do jogo, que já é um quadro, em cima; nome e codex
/// na legenda embaixo. Clicável, ele descansa com a sombra `sm` e levanta no
/// hover. Desabilitado, apaga a 40% e não registra área clicável — sem clique,
/// sem hover e sem cursor de mão.
pub fn stratagem_card(ui: &mut Ui, id: Id, rect: Rect, strat: &Stratagem, state: CardState) {
    let look = TileLook {
        filled: if state.in_active_slot { 1.0 } else { 0.0 },
        alpha: if state.disabled {
            theme::DISABLED_ALPHA
        } else {
            1.0
        },
    };
    let face = if state.disabled {
        rect
    } else {
        let rest = if state.in_active_slot {
            theme::SHADOW
        } else {
            theme::SHADOW_SM
        };
        {
            let touched = touch(ui, id);
            raised(ui, rect, touched, rest)
        }
    };
    tile_body(ui, face, &strat.imagem, &strat.nome, &strat.codex, look);
    if !state.disabled {
        ui.hit(id, rect);
    }
}

fn tile_body(ui: &mut Ui, rect: Rect, image: &str, name: &str, codex: &[Dir], look: TileLook) {
    let palette = theme::palette();
    let fade = |color: Color| color.faded(look.alpha);
    let side = (rect.w - theme::BORDER * 2.0).max(0.0);

    ui.fill(rect, fade(palette.base_100));
    let icon = Rect::new(rect.x + theme::BORDER, rect.y + theme::BORDER, side, side);
    ui.image_styled(
        icon,
        format!("icons/{image}"),
        ImageStyle::FILL.opacity(look.alpha),
    );

    // Legenda: separada do ícone pela mesma linha grossa da moldura.
    let divider = Rect::new(rect.x, icon.bottom(), rect.w, theme::BORDER);
    let caption = Rect::new(
        icon.x,
        divider.bottom(),
        side,
        (rect.bottom() - theme::BORDER - divider.bottom()).max(0.0),
    );
    ui.fill(divider, fade(palette.base_300));
    let ink = palette.content.mix(palette.accent_content, look.filled);
    if look.filled > 0.0 {
        ui.fill(caption, fade(palette.accent.faded(look.filled)));
    }

    let mut text = caption.inset_xy(TILE_PADDING, 6.0);
    let name_area = text.cut_top(TILE_NAME_HEIGHT);
    ui.text(
        name_area,
        name.to_uppercase(),
        TextStyle::new(font::SIZE_MICRO, Weight::Black)
            .align(Align::Center)
            .wrap(),
        fade(ink),
    );
    let size = codex_size(codex.len());
    let codex_row = text.middle_row(size);
    arrow_row(ui, codex_row, codex, fade(ink));
    ui.stroke(rect, theme::BORDER, fade(palette.base_300));
}

// --- Card de apoio fixo ---

/// Card de um apoio fixo (Reforço, Ressuprimento, Rearme da Águia). Não é
/// clicável — quem responde é o botão de atalho embaixo dele —, mas recebe as
/// mesmas piscadas de disparo e bloqueio dos slots de macro.
pub fn support_card(ui: &mut Ui, index: usize, rect: Rect, support: &SupportStrat) {
    let triggered = ui.anim(
        flash_id(index, true, FlashKind::Triggered),
        FLASH_TRIGGERED_MS,
    );
    let blocked = ui.anim(flash_id(index, true, FlashKind::Blocked), FLASH_BLOCKED_MS);

    blocked_frame(ui, rect, blocked);
    tile_body(
        ui,
        rect,
        support.imagem,
        support.nome,
        support.codex,
        TileLook {
            filled: triggered,
            alpha: 1.0,
        },
    );
}

/// Moldura de erro em volta de um slot recusado pelo engine, apagando com o
/// pulso.
fn blocked_frame(ui: &mut Ui, rect: Rect, blocked: f32) {
    if blocked > 0.0 {
        ui.stroke(
            rect.inset(-5.0),
            theme::BORDER,
            theme::palette().error.fill.faded(blocked),
        );
    }
}

// --- Slot de macro ---

/// Lado do quadrado de slot.
pub const SLOT_SIZE: f32 = 64.0;
/// Etiqueta do atalho, colada na borda de cima do slot.
const SLOT_TAG_HEIGHT: f32 = 16.0;
const SLOT_TAG_PADDING: f32 = 6.0;

/// Duração das piscadas de disparo e de bloqueio.
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
/// apoio fixo, que recebem os mesmos eventos do engine.
pub fn flash_id(index: usize, support: bool, kind: FlashKind) -> Id {
    let name = match (support, kind) {
        (false, FlashKind::Triggered) => "ui.flash.slot",
        (false, FlashKind::Blocked) => "ui.flash.slot.blocked",
        (true, FlashKind::Triggered) => "ui.flash.support",
        (true, FlashKind::Blocked) => "ui.flash.support.blocked",
    };
    id_at(name, index)
}

/// Quadrado de um slot de macro: o ícone equipado (ou `[VAZIO]`), o atalho
/// como etiqueta colada na borda de cima, o slot em edição erguido com a
/// etiqueta em accent e as duas piscadas — o disparo inunda de accent, o
/// bloqueio acende uma moldura de erro em volta.
#[allow(clippy::too_many_arguments)]
pub fn slot_square(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    index: usize,
    rect: Rect,
    strat: Option<&Stratagem>,
    shortcut: &str,
    active: bool,
    empty_label: &str,
) {
    let palette = theme::palette();
    let id = slot_id(index);
    let clear = slot_clear_id(index);
    let triggered = ui.anim(
        flash_id(index, false, FlashKind::Triggered),
        FLASH_TRIGGERED_MS,
    );
    let blocked = ui.anim(flash_id(index, false, FlashKind::Blocked), FLASH_BLOCKED_MS);
    let hover = ui.fade(id, ui.is_hot(id) || ui.is_hot(clear), motion::HOVER_MS);

    // O slot em edição fica erguido, como um cartão pego da mesa.
    let rest = if active {
        theme::SHADOW
    } else {
        theme::SHADOW_SM
    };
    let lifted = if active { 1.0 } else { hover };
    let face = raised(
        ui,
        rect,
        Touch {
            hover: lifted,
            pressed: ui.is_pressed(id),
        },
        rest,
    );
    blocked_frame(ui, face, blocked);

    ui.fill(face, palette.base_100);
    match strat {
        Some(strat) => ui.image(
            face.inset(theme::BORDER),
            format!("icons/{}", strat.imagem),
            1.0,
        ),
        None => ui.text(
            face,
            bracketed(empty_label),
            TextStyle::new(font::SIZE_TINY, Weight::Bold)
                .align(Align::Center)
                .middle(),
            palette.muted,
        ),
    }
    if triggered > 0.0 {
        ui.fill(
            face.inset(theme::BORDER),
            palette.accent.faded(0.6 * triggered),
        );
    }
    ui.stroke(face, theme::BORDER, palette.base_300);
    shortcut_tag(ui, measure, face, shortcut, active || triggered > 0.0);
    ui.hit(id, rect);

    // O × sai por cima e é registrado depois, então ganha o clique na sobreposição.
    if strat.is_some() && hover > 0.0 {
        let button = Rect::new(
            face.right() - CORNER_BUTTON + 4.0,
            face.y - 4.0,
            CORNER_BUTTON,
            CORNER_BUTTON,
        );
        icon_btn(ui, clear, button, Glyph::Close, Tone::Danger);
    }
}

/// Etiqueta do atalho (`tag`), meio para fora da borda de cima do slot.
fn shortcut_tag(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, shortcut: &str, active: bool) {
    let palette = theme::palette();
    let style = TextStyle::new(font::SIZE_TINY, Weight::Black)
        .align(Align::Center)
        .middle();
    let width = measure.text_size(shortcut, style, f32::INFINITY).0 + SLOT_TAG_PADDING * 2.0;
    let tag = Rect::new(
        rect.center_x() - width / 2.0,
        rect.y - SLOT_TAG_HEIGHT / 2.0,
        width,
        SLOT_TAG_HEIGHT,
    );
    let (fill, ink) = if active {
        (palette.accent, palette.accent_content)
    } else {
        (palette.base_200, palette.content)
    };
    framed(ui, tag, fill);
    ui.text(tag, shortcut, style, ink);
}

// --- Dropdown (§6.4, §6.5) ---

/// Altura de uma linha da lista aberta (`px-3 py-2`).
pub const DROPDOWN_ROW: f32 = 28.0;
/// A lista para de crescer aqui e passa a rolar.
pub const DROPDOWN_MAX_HEIGHT: f32 = 240.0;

/// Campo fechado do dropdown: o valor atual e o `▼` literal. É o `<select>`,
/// que aqui não pode ser um controle nativo — a lista precisa do mesmo tema.
pub fn dropdown_field(ui: &mut Ui, id: Id, rect: Rect, value: &str, open: bool) {
    let palette = theme::palette();
    let hover = ui.fade(id, ui.is_hot(id), motion::HOVER_MS);
    framed(ui, rect, palette.base_200.mix(palette.base_100, hover));
    if open {
        focus_ring(ui, rect);
    }

    let mut inner = rect.inset_xy(FIELD_PADDING_X, 0.0);
    let chevron = inner.cut_right(14.0);
    ui.text(
        chevron,
        if open { "\u{25B2}" } else { "\u{25BC}" },
        TextStyle::new(font::SIZE_MICRO, Weight::Regular)
            .align(Align::End)
            .middle(),
        palette.content,
    );
    ui.text(
        inner,
        value.to_uppercase(),
        styles::label().middle(),
        palette.content,
    );
    ui.hit(id, rect);
}

/// Fundo da lista aberta: popover com moldura e sombra `sm`. As linhas e a
/// rolagem ficam com quem chama — só ela sabe quantos itens a categoria tem.
pub fn dropdown_panel(ui: &mut Ui, rect: Rect) {
    shadow(ui, rect, theme::SHADOW_SM);
    framed(ui, rect, theme::palette().base_200);
}

/// Uma linha da lista. O item sob o mouse inunda de accent (item ativo do
/// popover); o escolhido leva o `✓` e o texto em accent.
pub fn dropdown_row(ui: &mut Ui, id: Id, rect: Rect, label: &str, selected: bool) {
    let palette = theme::palette();
    let hover = ui.fade(id, ui.is_hot(id), motion::HOVER_MS);
    if hover > 0.0 {
        ui.fill(rect, palette.accent.faded(hover));
    }
    let resting = if selected {
        palette.accent_text
    } else {
        palette.content
    };
    let ink = resting.mix(palette.accent_content, hover);
    let mut inner = rect.inset_xy(FIELD_PADDING_X, 0.0);
    if selected {
        ui.text(
            inner.cut_right(14.0),
            "\u{2713}",
            TextStyle::new(font::SIZE_BODY, Weight::Black)
                .align(Align::End)
                .middle(),
            ink,
        );
    }
    ui.text(inner, label.to_uppercase(), styles::label().middle(), ink);
    ui.hit(id, rect);
}

// --- Chips das builds salvas ---

/// Chip de build salva.
pub const CHIP_HEIGHT: f32 = 32.0;
pub const CHIP_GAP: f32 = 10.0;
const CHIP_PADDING: f32 = 14.0;

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
    styles::micro_black().align(Align::Center).middle()
}

/// Onde cada chip cai numa faixa de `width` de largura: os nomes têm tamanhos
/// diferentes e a fileira quebra como um `flex-wrap`. O vão entre fileiras
/// deixa espaço para a sombra e o × do canto.
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

/// Chip de uma build salva — a `tag` clicável: levanta e inunda de accent no
/// hover, e fica em accent quando é a build aplicada. Com `delete`, ganha o ×
/// que aparece sob o mouse. O overlay usa a mesma peça sem o × — lá a build é
/// só aplicada (a gestão fica na janela principal).
pub fn loadout_chip(ui: &mut Ui, id: Id, delete: Option<Id>, rect: Rect, name: &str, active: bool) {
    let palette = theme::palette();
    let hovered = ui.is_hot(id) || delete.is_some_and(|delete| ui.is_hot(delete));
    let hover = ui.fade(id, hovered, motion::HOVER_MS);
    let name = name.to_uppercase();

    let fill_t = if active { 1.0 } else { hover };
    let face = raised(
        ui,
        rect,
        Touch {
            hover,
            pressed: ui.is_pressed(id),
        },
        if active { theme::SHADOW_SM } else { 0.0 },
    );
    framed(ui, face, palette.base_200.mix(palette.accent, fill_t));
    ui.text(
        face,
        name,
        chip_style(),
        palette.content.mix(palette.accent_content, fill_t),
    );
    ui.hit(id, rect);

    let Some(delete) = delete else {
        return;
    };
    if hover <= 0.0 {
        return;
    }
    let button = Rect::new(
        face.right() - CORNER_BUTTON + 4.0,
        face.y - 6.0,
        CORNER_BUTTON,
        CORNER_BUTTON,
    );
    icon_btn(ui, delete, button, Glyph::Close, Tone::Danger);
}

// --- Card de item de build ---

/// Barra do topo do card, com a categoria e o cadeado.
const ITEM_BAR: f32 = 30.0;
/// Padding do bloco de texto.
const ITEM_TEXT_PADDING: f32 = 10.0;
const ITEM_TEXT_MIN_HEIGHT: f32 = 44.0;
const ITEM_TEXT_GAP: f32 = 4.0;
/// Botão do cadeado.
const LOCK_SIZE: f32 = 22.0;
/// Respiro em volta da imagem.
const ITEM_IMAGE_PADDING: f32 = 12.0;

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
    TextStyle::new(font::SIZE_MICRO, Weight::Bold)
        .align(Align::Center)
        .wrap()
}

fn item_description_style() -> TextStyle {
    TextStyle::new(font::SIZE_MICRO, Weight::Regular)
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
    (height + ITEM_TEXT_PADDING * 2.0).max(ITEM_TEXT_MIN_HEIGHT)
}

/// Altura total do card: barra, imagem quadrada e o texto embaixo.
pub fn item_card_height(measure: &mut dyn Measure, card: &ItemCard, width: f32) -> f32 {
    ITEM_BAR + width + item_card_text_height(measure, card, width)
}

/// Card de um item da build: caixa aninhada com a barra da categoria, a imagem
/// e o texto. O cadeado mantém o item no próximo sorteio; travado, a barra
/// inunda de accent.
pub fn build_item_card(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    lock: Id,
    rect: Rect,
    card: &ItemCard,
) {
    let palette = theme::palette();
    ui.fill(rect, palette.base_100);

    // Barra: categoria, etiqueta de set e o cadeado.
    let mut body = rect;
    let bar = body.cut_top(ITEM_BAR);
    if card.locked {
        ui.fill(bar, palette.accent);
    }
    ui.fill(
        Rect::new(bar.x, bar.bottom() - theme::BORDER, bar.w, theme::BORDER),
        palette.base_300,
    );
    let mut row = bar.inset_xy(8.0, 0.0);
    row.h -= theme::BORDER;
    let button = row.cut_right(LOCK_SIZE).middle_row(LOCK_SIZE);
    icon_btn(
        ui,
        lock,
        button,
        Glyph::Lock(card.locked),
        if card.locked {
            Tone::Accent
        } else {
            Tone::Plain
        },
    );
    row.cut_right(6.0);
    if let Some(badge) = card.badge {
        let tag = row.cut_right(34.0).middle_row(16.0);
        let (fill, ink) = if card.locked {
            (palette.accent_content, palette.accent)
        } else {
            (palette.accent, palette.accent_content)
        };
        framed(ui, tag, fill);
        ui.text(
            tag,
            badge.to_uppercase(),
            TextStyle::new(font::SIZE_TINY, Weight::Black)
                .align(Align::Center)
                .middle(),
            ink,
        );
        row.cut_right(6.0);
    }
    ui.text(
        row,
        card.label.to_uppercase(),
        styles::micro().middle(),
        if card.locked {
            palette.accent_content
        } else {
            palette.muted
        },
    );

    // Imagem quadrada, com proporção preservada: os renders de arma são bem
    // mais largos que altos e esticá-los deformaria a silhueta.
    let square = body.cut_top(rect.w.min(body.h));
    let picture = square.inset(ITEM_IMAGE_PADDING);
    match card.image {
        Some(path) => ui.image_styled(picture, format!("icons/{path}"), ImageStyle::FILL.contain()),
        // Item vazio (e ícone vetorial, que o decodificador não lê).
        None => ui.text(
            picture,
            "\u{25A1}",
            TextStyle::new(28.0, Weight::Regular)
                .align(Align::Center)
                .middle(),
            palette.muted,
        ),
    }

    item_card_text(ui, measure, body, card);
    ui.stroke(rect, theme::BORDER, palette.base_300);
}

fn item_card_text(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, card: &ItemCard) {
    let palette = theme::palette();
    let mut text = rect.inset(ITEM_TEXT_PADDING);
    let name = card.name.to_uppercase();
    let height = measure.text_size(&name, item_name_style(), text.w).1;
    ui.text(
        text.cut_top(height),
        name,
        item_name_style(),
        palette.content,
    );

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
            palette.accent_text,
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
            palette.muted,
        );
    }
}

/// Trilho de uso (`usage-track`/`usage-fill`, §6.7): moldura sobre o fundo de
/// página, fill sólido de accent proporcional a `share`.
pub fn usage_bar(ui: &mut Ui, rect: Rect, share: f32) {
    let palette = theme::palette();
    framed(ui, rect, palette.base_100);
    let inner = rect.inset(theme::BORDER);
    let share = share.clamp(0.0, 1.0);
    if share > 0.0 {
        ui.fill(inner.with_w(inner.w * share), palette.accent);
    }
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

    const TABS: [&str; 3] = ["Macros", "Builds", "Config"];

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

    fn tab_rect(ui: &Ui, index: usize) -> Rect {
        ui.frame()
            .nodes
            .iter()
            .find(|node| {
                matches!(&node.visual, Visual::Text { text, .. } if *text == TABS[index].to_uppercase())
            })
            .expect("rótulo da aba")
            .rect
    }

    #[test]
    fn every_tab_answers_in_its_own_slot_and_the_toggle_sits_apart() {
        let mut ui = Ui::new();
        header(&mut ui, 0);

        for index in 0..TABS.len() {
            let rect = tab_rect(&ui, index);
            assert_eq!(
                ui.frame().hit_at(rect.center_x(), rect.center_y()),
                Some(tab_id(index))
            );
        }
        // A marca não é clicável, e o toggle fica na ponta direita.
        assert_eq!(ui.frame().hit_at(30.0, TAB_BAR_HEIGHT / 2.0), None);
        assert_eq!(
            ui.frame().hit_at(820.0 - 30.0, TAB_BAR_HEIGHT / 2.0),
            Some(theme_toggle_id())
        );
        // Abaixo da topbar não há clique.
        assert_eq!(
            ui.frame()
                .hit_at(tab_rect(&ui, 1).center_x(), TAB_BAR_HEIGHT + 5.0),
            None
        );
    }

    #[test]
    fn the_active_tab_floods_with_accent_and_the_others_do_not() {
        let mut ui = Ui::new();
        header(&mut ui, 2);
        let palette = theme::palette();

        for (index, tab) in TABS.iter().enumerate() {
            let rect = tab_rect(&ui, index);
            let flooded = ui.frame().nodes.iter().any(|node| {
                node.rect == rect
                    && matches!(node.visual, Visual::Fill { color } if color == palette.accent)
            });
            assert_eq!(flooded, index == 2, "aba {index}");
            let label = ui
                .frame()
                .nodes
                .iter()
                .find(|node| {
                    matches!(&node.visual, Visual::Text { text, .. } if *text == tab.to_uppercase())
                })
                .unwrap();
            let expected = if index == 2 {
                palette.accent_content
            } else {
                palette.content
            };
            assert!(matches!(label.visual, Visual::Text { color, .. } if color == expected));
        }
    }

    #[test]
    fn hovering_an_inactive_tab_starts_a_fade() {
        let mut ui = Ui::new();
        header(&mut ui, 0);
        assert!(!ui.animating());

        let target = tab_rect(&ui, 1);
        let response = ui.input(Input::Move {
            x: target.center_x(),
            y: target.center_y(),
        });
        assert!(response.redraw);

        header(&mut ui, 0);
        assert!(ui.animating(), "o fade de hover pede o timer de 16ms");
    }

    #[test]
    fn the_brand_names_the_app() {
        let mut ui = Ui::new();
        header(&mut ui, 0);
        let texts = texts(&ui);
        assert!(texts.iter().any(|text| text == "> HELLDIVERS 2"));
        assert!(texts.iter().any(|text| text == "MACRO"));
    }

    #[test]
    fn a_card_reserves_padding_and_the_title_bar() {
        let mut ui = Ui::new();
        ui.begin(0);
        let plain = card(&mut ui, Rect::new(0.0, 0.0, 400.0, 200.0), None);
        assert_eq!(
            plain,
            Rect::new(
                CARD_PADDING,
                CARD_PADDING,
                400.0 - CARD_PADDING * 2.0,
                200.0 - CARD_PADDING * 2.0
            )
        );

        let with_header = card(
            &mut ui,
            Rect::new(0.0, 0.0, 400.0, 200.0),
            Some(CardHeader::new("Atalhos de Combate", "cfg")),
        );
        assert_eq!(with_header.y, plain.y + CARD_BAR_HEIGHT);
        assert_eq!(with_header.x, plain.x);
        assert_eq!(
            card_chrome(true) - card_chrome(false),
            CARD_BAR_HEIGHT,
            "a conta de altura casa com o desenho"
        );
        ui.end();

        assert!(texts(&ui).contains(&"ATALHOS_DE_COMBATE.CFG".to_string()));
    }

    #[test]
    fn a_panel_casts_a_hard_shadow_down_and_to_the_right() {
        let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
        let mut ui = Ui::new();
        ui.begin(0);
        card(&mut ui, rect, None);
        ui.end();

        let first = &ui.frame().nodes[0];
        assert_eq!(first.rect, rect.translate(theme::SHADOW, theme::SHADOW));
        assert_eq!(
            first.visual,
            Visual::Fill {
                color: theme::palette().shadow
            }
        );
    }

    #[test]
    fn file_names_are_upper_snake_with_the_extension() {
        assert_eq!(
            file_name("Atalhos de Combate", "cfg"),
            "ATALHOS_DE_COMBATE.CFG"
        );
        assert_eq!(
            file_name("Config. de Controles", "cfg"),
            "CONFIG_DE_CONTROLES.CFG"
        );
        assert_eq!(file_name("Builds Salvas", "db"), "BUILDS_SALVAS.DB");
        assert_eq!(
            file_name("  Meta — por Facção ", "log"),
            "META_POR_FACÇÃO.LOG"
        );
    }

    #[test]
    fn machine_text_helpers_add_the_sigil_and_the_brackets() {
        assert_eq!(sigil("Ofensivo"), "> OFENSIVO");
        assert_eq!(bracketed("vazio"), "[VAZIO]");
    }

    #[test]
    fn the_caret_blinks_after_the_text_without_moving_it() {
        let rect = Rect::new(0.0, 0.0, 200.0, 20.0);
        let style = styles::label().align(Align::Center).middle();
        let mut ui = Ui::new();

        ui.begin(0);
        caret_text(
            &mut ui,
            &mut Fixed,
            rect,
            "Ouvindo...",
            style,
            theme::ROSE.content,
        );
        ui.end();
        let lit = ui.frame().nodes.clone();
        assert_eq!(
            texts(&ui),
            vec!["OUVINDO", "_"],
            "as reticências dão lugar ao caret"
        );
        assert!(ui.animating());

        ui.begin(600);
        caret_text(
            &mut ui,
            &mut Fixed,
            rect,
            "Ouvindo...",
            style,
            theme::ROSE.content,
        );
        ui.end();
        assert_eq!(texts(&ui), vec!["OUVINDO"], "segunda metade: caret apagado");
        assert_eq!(ui.frame().nodes[0].rect, lit[0].rect, "o texto não dança");
    }

    #[test]
    fn a_hovered_button_lifts_but_keeps_its_hit_area() {
        let mut ui = Ui::new();
        let rect = Rect::new(0.0, 20.0, 200.0, CONTROL_HEIGHT);
        let draw = |ui: &mut Ui, now: u64| {
            ui.begin(now);
            button(ui, id("go"), rect, "Gerar", ButtonVariant::Primary);
            ui.end();
        };

        draw(&mut ui, 0);
        // A moldura é desenhada sobre a face, com o mesmo retângulo.
        let face = |ui: &Ui| {
            ui.frame()
                .nodes
                .iter()
                .find(|node| matches!(node.visual, Visual::Stroke { .. }))
                .expect("moldura do botão")
                .rect
        };
        assert_eq!(face(&ui), rect, "em repouso");

        ui.input(Input::Move { x: 10.0, y: 30.0 });
        draw(&mut ui, 0);
        draw(&mut ui, 1_000);
        assert_eq!(face(&ui), rect.translate(0.0, -theme::LIFT), "levantou");
        assert_eq!(ui.frame().hit_at(10.0, rect.bottom() - 1.0), Some(id("go")));

        // Pressionado, volta ao lugar — e o clique conta.
        ui.input(Input::Down { x: 10.0, y: 30.0 });
        draw(&mut ui, 1_000);
        assert_eq!(face(&ui), rect);
        assert_eq!(
            ui.input(Input::Up { x: 10.0, y: 30.0 }).clicked,
            Some(id("go"))
        );
    }

    #[test]
    fn a_disabled_button_neither_lifts_nor_answers() {
        let mut ui = Ui::new();
        ui.begin(0);
        button(
            &mut ui,
            id("save"),
            Rect::new(0.0, 0.0, 120.0, CONTROL_HEIGHT),
            "Salvar",
            ButtonVariant::Disabled,
        );
        ui.end();
        assert_eq!(ui.frame().hit_at(10.0, 10.0), None);
        assert!(ui.frame().nodes.iter().all(|node| match node.visual {
            Visual::Fill { color } | Visual::Stroke { color, .. } | Visual::Text { color, .. } =>
                (color.a - theme::DISABLED_ALPHA).abs() < 1e-4,
            _ => true,
        }));
    }

    #[test]
    fn the_switch_thumb_slides_from_one_end_to_the_other() {
        let mut ui = Ui::new();
        let rect = Rect::new(0.0, 0.0, 300.0, 56.0);

        let thumb_x = |ui: &Ui| {
            ui.frame()
                .nodes
                .iter()
                .find(|node| node.rect.w == SWITCH_THUMB && node.rect.h == SWITCH_THUMB)
                .expect("polegar")
                .rect
                .x
        };

        // Com o tempo parado o fade não anda: o estado inicial é o "desligado".
        ui.begin(0);
        toggle_row(&mut ui, id("hud"), rect, "HUD", "desc", false);
        ui.end();
        let off = thumb_x(&ui);

        // Ligado e com tempo suficiente para o fade terminar.
        ui.begin(0);
        toggle_row(&mut ui, id("hud"), rect, "HUD", "desc", true);
        ui.end();
        ui.begin(1_000);
        toggle_row(&mut ui, id("hud"), rect, "HUD", "desc", true);
        ui.end();
        assert!(thumb_x(&ui) > off);
        assert!(!ui.animating(), "chegou na ponta e parou");
    }

    #[test]
    fn an_edit_host_publishes_its_rect_for_the_native_child() {
        let mut ui = Ui::new();
        let rect = Rect::new(10.0, 10.0, 300.0, CONTROL_HEIGHT);

        ui.begin(0);
        edit_host(&mut ui, id("search"), rect, true, None, 0.0);
        ui.end();

        let edits = ui.frame().edits();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].id, id("search"));
        assert!(edits[0].rect.w < 300.0, "o texto entra com folga da borda");
        assert_eq!(ui.frame().hit_at(20.0, 20.0), Some(id("search")));
        // Com foco, o anel de 2px fica 2px para fora do campo.
        let ring = rect.inset(-(theme::FOCUS_OFFSET + theme::FOCUS_RING));
        assert!(ui.frame().nodes.iter().any(|node| node.rect == ring));
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
            theme::ROSE.content,
        );
        ui.end();

        assert_eq!(lines(&ui).len(), 3);
        assert_eq!(
            ui.frame().nodes[0].visual,
            Visual::Line {
                from: (12.0, 20.0),
                to: (12.0, 5.0),
                width: ARROW_STROKE_UNITS,
                color: theme::ROSE.content
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
        arrow_row(&mut ui, row, &short, theme::ROSE.content);
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

    fn tile(ui: &mut Ui, now: u64, state: CardState) {
        let rect = Rect::new(0.0, 0.0, 174.0, tile_height(174.0));
        ui.begin(now);
        stratagem_card(
            ui,
            id("card"),
            rect,
            &stratagem(&[Dir::Up, Dir::Down]),
            state,
        );
        ui.end();
    }

    const IDLE: CardState = CardState {
        disabled: false,
        in_active_slot: false,
    };

    #[test]
    fn a_disabled_tile_answers_neither_click_nor_hover_and_fades() {
        let mut ui = Ui::new();
        tile(
            &mut ui,
            0,
            CardState {
                disabled: true,
                ..IDLE
            },
        );
        assert_eq!(ui.frame().hit_at(10.0, 10.0), None);
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            node.visual,
            Visual::Image { style, .. } if style.opacity == theme::DISABLED_ALPHA
        )));

        tile(&mut ui, 0, IDLE);
        assert_eq!(ui.frame().hit_at(10.0, 10.0), Some(id("card")));
    }

    #[test]
    fn the_tile_of_the_slot_being_edited_floods_its_caption() {
        let mut ui = Ui::new();
        tile(
            &mut ui,
            0,
            CardState {
                in_active_slot: true,
                ..IDLE
            },
        );
        let palette = theme::palette();
        // A legenda fica embaixo do ícone; a sombra (que no escuro também é
        // accent) fica colada no topo.
        assert!(ui.frame().nodes.iter().any(|node| node.rect.y > 150.0
            && matches!(node.visual, Visual::Fill { color } if color == palette.accent)));
        assert!(ui.frame().nodes.iter().any(|node| matches!(
            &node.visual,
            Visual::Text { text, color, .. }
                if text == "ORBITAL PRECISION STRIKE" && *color == palette.accent_content
        )));
    }

    #[test]
    fn a_tile_shows_the_whole_icon_above_its_caption() {
        let mut ui = Ui::new();
        tile(&mut ui, 0, IDLE);
        let icon = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(node.visual, Visual::Image { .. }))
            .unwrap()
            .rect;
        assert_eq!(icon.w, icon.h, "o ícone do jogo fica quadrado");
        let name = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(&node.visual, Visual::Text { .. }))
            .unwrap()
            .rect;
        assert!(name.y >= icon.bottom(), "a legenda não cobre o ícone");
    }

    fn slot(ui: &mut Ui, now: u64, strat: Option<&Stratagem>, active: bool) {
        ui.begin(now);
        slot_square(
            ui,
            &mut Fixed,
            0,
            Rect::new(0.0, 20.0, SLOT_SIZE, SLOT_SIZE),
            strat,
            "F1",
            active,
            "Vazio",
        );
        ui.end();
    }

    #[test]
    fn a_filled_slot_offers_the_clear_button_only_under_the_mouse() {
        let strat = stratagem(&[Dir::Up]);
        let mut ui = Ui::new();

        slot(&mut ui, 0, Some(&strat), false);
        // Ponto no canto superior direito, dentro do quadrado e do × que o cobre.
        let corner = (SLOT_SIZE - 5.0, 25.0);
        assert_eq!(
            ui.frame().hit_at(corner.0, corner.1),
            Some(slot_id(0)),
            "sem o mouse em cima o × não existe"
        );

        // O fade de hover precisa de tempo para abrir.
        ui.input(Input::Move { x: 30.0, y: 50.0 });
        slot(&mut ui, 0, Some(&strat), false);
        slot(&mut ui, 1_000, Some(&strat), false);
        assert_eq!(
            ui.frame().hit_at(corner.0, corner.1 - 4.0),
            Some(slot_clear_id(0))
        );

        // Slot vazio nunca mostra o botão, e diz que está vazio.
        slot(&mut ui, 2_000, None, false);
        assert_eq!(ui.frame().hit_at(corner.0, corner.1), Some(slot_id(0)));
        assert!(texts(&ui).contains(&"[VAZIO]".to_string()));
    }

    #[test]
    fn the_active_slot_is_raised_with_an_accent_tag() {
        let strat = stratagem(&[Dir::Up]);
        let mut ui = Ui::new();
        slot(&mut ui, 0, Some(&strat), true);
        let palette = theme::palette();

        let icon = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(node.visual, Visual::Image { .. }))
            .unwrap()
            .rect;
        assert!(icon.y < 20.0, "erguido acima do lugar de repouso");
        let tag = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(&node.visual, Visual::Text { text, .. } if text == "F1"))
            .unwrap();
        assert!(
            matches!(tag.visual, Visual::Text { color, .. } if color == palette.accent_content)
        );
    }

    #[test]
    fn a_triggered_slot_flashes_and_goes_back_to_rest() {
        let strat = stratagem(&[Dir::Up]);
        let mut ui = Ui::new();
        slot(&mut ui, 0, Some(&strat), false);
        let resting = ui.frame().nodes.len();

        ui.flash(flash_id(0, false, FlashKind::Triggered), FLASH_TRIGGERED_MS);
        slot(&mut ui, 0, Some(&strat), false);
        assert!(ui.animating(), "a piscada mantém o timer vivo");
        assert!(ui.frame().nodes.len() > resting, "o fill de accent entra");

        slot(&mut ui, FLASH_TRIGGERED_MS as u64 * 2, Some(&strat), false);
        assert!(!ui.animating());
        assert_eq!(ui.frame().nodes.len(), resting);
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
        let list = Rect::new(0.0, 44.0, 200.0, DROPDOWN_ROW * 3.0);

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
            ui.frame().hit_at(10.0, 44.0 + DROPDOWN_ROW * 1.5),
            Some(id_at("armor.row", 1))
        );
        // A escolhida leva o ✓, e só ela.
        assert_eq!(
            texts(&ui).iter().filter(|text| *text == "\u{2713}").count(),
            1
        );
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
            short >= ITEM_BAR + width + ITEM_TEXT_MIN_HEIGHT,
            "a altura mínima vale"
        );
        assert!(tall > short, "a ficha da armadura empurra o card");
    }

    #[test]
    fn a_locked_item_card_floods_its_bar_and_the_lock_takes_the_click() {
        let card = ItemCard {
            label: "Armadura",
            name: "A-35 Recon",
            image: Some("equipment/armor-a-35-recon.webp"),
            subtitle: None,
            description: None,
            badge: Some("SET"),
            locked: true,
        };
        let rect = Rect::new(0.0, 0.0, 180.0, 280.0);
        let mut ui = Ui::new();

        ui.begin(0);
        build_item_card(&mut ui, &mut Fixed, id("lock"), rect, &card);
        ui.end();

        // O cadeado fica na barra, no canto direito, e é a única área clicável.
        assert_eq!(
            ui.frame().hit_at(rect.right() - 16.0, ITEM_BAR / 2.0),
            Some(id("lock"))
        );
        assert_eq!(ui.frame().hit_at(rect.center_x(), rect.center_y()), None);
        let palette = theme::palette();
        assert!(ui.frame().nodes.iter().any(|node| node.rect.h == ITEM_BAR
            && matches!(node.visual, Visual::Fill { color } if color == palette.accent)));
        assert!(texts(&ui).contains(&"SET".to_string()));
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
            Rect::new(0.0, 0.0, 180.0, 280.0),
            &card,
        );
        ui.end();

        assert!(!ui
            .frame()
            .nodes
            .iter()
            .any(|node| matches!(node.visual, Visual::Image { .. })));
        assert!(texts(&ui).contains(&"\u{25A1}".to_string()));
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
            0.0,
        );
        ui.end();

        assert!(ui.frame().edits().is_empty());
        assert!(texts(&ui).iter().any(|text| text.contains("BUSCAR")));
    }

    #[test]
    fn a_toast_fades_as_one_block() {
        let mut ui = Ui::new();
        ui.begin(0);
        toast(
            &mut ui,
            Rect::new(0.0, 0.0, TOAST_WIDTH, TOAST_HEIGHT),
            theme::palette().success,
            "Concluído",
            "Backup exportado!",
            0.5,
        );
        ui.end();
        assert!(ui.frame().nodes.iter().all(|node| match node.visual {
            Visual::Fill { color } | Visual::Stroke { color, .. } | Visual::Text { color, .. } =>
                color.a <= 0.5 + 1e-4,
            _ => true,
        }));
        assert!(texts(&ui).contains(&"BACKUP EXPORTADO!".to_string()));
    }

    #[test]
    fn the_usage_bar_fills_its_share_of_the_track() {
        let rect = Rect::new(0.0, 0.0, 104.0, 10.0);
        let mut ui = Ui::new();
        ui.begin(0);
        usage_bar(&mut ui, rect, 0.5);
        ui.end();
        let fill = ui
            .frame()
            .nodes
            .iter()
            .find(|node| matches!(node.visual, Visual::Fill { color } if color == theme::palette().accent))
            .unwrap();
        assert_eq!(fill.rect.w, (104.0 - theme::BORDER * 2.0) / 2.0);
    }
}
