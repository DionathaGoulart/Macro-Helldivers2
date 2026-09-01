//! Kit de widgets em Direct2D: layout, lista de desenho, hit-test e animação.
//!
//! O modelo de render é o do R14: **não existe loop**. Uma passagem de
//! construção transforma o estado do app numa lista de nós com retângulos já
//! resolvidos; `WM_PAINT` só percorre essa lista. Entrada do mouse é testada
//! contra a lista da última passagem, e só quando algo muda (hover, clique,
//! rolagem, animação em curso) a janela reconstrói e invalida. App parado não
//! desenha nada e não acorda a CPU.
//!
//! Nada aqui fala Win32: o desenho sai por [`Painter`], implementado sobre
//! `ID2D1RenderTarget` em `gfx::d2d` — o mesmo trait serve à janela principal
//! (`ID2D1HwndRenderTarget`) e ao overlay (`ID2D1DCRenderTarget`, Fase 9). Por
//! isso o módulo inteiro compila e é testado no host.

use std::path::{Path, PathBuf};

use crate::ui::theme::{self, Color};

/// Identidade estável de um widget entre passagens. Vem do nome, não da posição
/// na lista: um item que muda de lugar na grade não perde o hover.
pub type Id = u64;

/// FNV-1a de 64 bits: sem dependência, sem alocação e determinístico entre
/// execuções (um `DefaultHasher` não garante isso).
pub fn id(name: &str) -> Id {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Id de um item de coleção (slot 2, aba 1, card 37).
pub fn id_at(name: &str, index: usize) -> Id {
    let mut hash = id(name);
    for byte in index.to_le_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Retângulo em DIP, canto superior esquerdo + tamanho.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn center_x(self) -> f32 {
        self.x + self.w / 2.0
    }

    pub fn center_y(self) -> f32 {
        self.y + self.h / 2.0
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Encolhe pelos quatro lados.
    pub fn inset(self, by: f32) -> Rect {
        self.inset_xy(by, by)
    }

    pub fn inset_xy(self, x: f32, y: f32) -> Rect {
        Rect {
            x: self.x + x,
            y: self.y + y,
            w: (self.w - x * 2.0).max(0.0),
            h: (self.h - y * 2.0).max(0.0),
        }
    }

    pub fn translate(self, dx: f32, dy: f32) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub fn with_h(self, h: f32) -> Rect {
        Rect { h, ..self }
    }

    pub fn with_w(self, w: f32) -> Rect {
        Rect { w, ..self }
    }

    /// Faixa de altura `h` no topo, tirada de `self` (o resto fica em `self`).
    pub fn cut_top(&mut self, h: f32) -> Rect {
        let h = h.min(self.h);
        let cut = self.with_h(h);
        self.y += h;
        self.h -= h;
        cut
    }

    pub fn cut_bottom(&mut self, h: f32) -> Rect {
        let h = h.min(self.h);
        self.h -= h;
        Rect {
            y: self.bottom(),
            h,
            ..*self
        }
    }

    pub fn cut_left(&mut self, w: f32) -> Rect {
        let w = w.min(self.w);
        let cut = self.with_w(w);
        self.x += w;
        self.w -= w;
        cut
    }

    pub fn cut_right(&mut self, w: f32) -> Rect {
        let w = w.min(self.w);
        self.w -= w;
        Rect {
            x: self.right(),
            w,
            ..*self
        }
    }

    /// Pula um espaço no topo (gap entre linhas).
    pub fn skip_top(&mut self, h: f32) {
        let _ = self.cut_top(h);
    }

    /// Retângulo de `w`×`h` centralizado dentro deste.
    pub fn centered(self, w: f32, h: f32) -> Rect {
        Rect {
            x: self.x + (self.w - w) / 2.0,
            y: self.y + (self.h - h) / 2.0,
            w,
            h,
        }
    }

    /// Faixa de altura `h` centralizada verticalmente.
    pub fn middle_row(self, h: f32) -> Rect {
        Rect {
            y: self.y + (self.h - h) / 2.0,
            h,
            ..self
        }
    }

    pub fn intersect(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Rect {
            x,
            y,
            w: (self.right().min(other.right()) - x).max(0.0),
            h: (self.bottom().min(other.bottom()) - y).max(0.0),
        }
    }
}

/// Divide `rect` em `count` colunas de mesma largura separadas por `gap`.
pub fn columns(rect: Rect, count: usize, gap: f32) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let width = (rect.w - gap * (count - 1) as f32) / count as f32;
    (0..count)
        .map(|i| Rect {
            x: rect.x + (width + gap) * i as f32,
            w: width.max(0.0),
            ..rect
        })
        .collect()
}

/// Célula `index` de uma grade de `cols` colunas com células de altura `cell_h`.
pub fn grid_cell(rect: Rect, cols: usize, cell_h: f32, gap: f32, index: usize) -> Rect {
    let cols = cols.max(1);
    let width = (rect.w - gap * (cols - 1) as f32) / cols as f32;
    let (row, col) = (index / cols, index % cols);
    Rect {
        x: rect.x + (width + gap) * col as f32,
        y: rect.y + (cell_h + gap) * row as f32,
        w: width.max(0.0),
        h: cell_h,
    }
}

/// Altura total de uma grade com `count` itens.
pub fn grid_height(count: usize, cols: usize, cell_h: f32, gap: f32) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let rows = count.div_ceil(cols.max(1));
    cell_h * rows as f32 + gap * (rows - 1) as f32
}

/// Peso da fonte. Só os dois que o app embute (Inter Regular e Black).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

/// Como um texto é desenhado. O `uppercase` do CSS é aplicado por quem monta a
/// string; aqui fica o que o DirectWrite precisa saber.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub weight: Weight,
    /// Espaçamento entre letras em EM (`tracking-[0.25em]` do tema).
    pub tracking: f32,
    pub align: Align,
    /// Centraliza verticalmente no retângulo.
    pub middle: bool,
    pub wrap: bool,
}

impl TextStyle {
    pub const fn new(size: f32, weight: Weight) -> TextStyle {
        TextStyle {
            size,
            weight,
            tracking: 0.0,
            align: Align::Start,
            middle: false,
            wrap: false,
        }
    }

    pub const fn tracking(self, tracking: f32) -> TextStyle {
        TextStyle { tracking, ..self }
    }

    pub const fn align(self, align: Align) -> TextStyle {
        TextStyle { align, ..self }
    }

    pub const fn middle(self) -> TextStyle {
        TextStyle {
            middle: true,
            ..self
        }
    }

    pub const fn wrap(self) -> TextStyle {
        TextStyle { wrap: true, ..self }
    }
}

/// Um desenho elementar. A lista de nós é o único formato que o painter enxerga.
#[derive(Debug, Clone, PartialEq)]
pub enum Visual {
    /// Retângulo (arredondado quando `radius > 0`).
    Fill {
        radius: f32,
        color: Color,
    },
    /// Gradiente vertical, usado nos headers dos cards de estratagema.
    Gradient {
        radius: f32,
        from: Color,
        to: Color,
    },
    Stroke {
        radius: f32,
        width: f32,
        color: Color,
    },
    Ellipse {
        color: Color,
    },
    /// Segmento com pontas arredondadas. É o traço das setas do codex, que os
    /// ícones do legado desenhavam como linha + chevron.
    Line {
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        color: Color,
    },
    Text {
        text: String,
        style: TextStyle,
        color: Color,
    },
    /// Caminho relativo à raiz de `assets/`.
    ///
    /// `radius` recorta a imagem num retângulo arredondado (o `overflow-hidden`
    /// dos cards) e `zoom` amplia o conteúdo em torno do centro sem mexer no
    /// retângulo — juntos são o `object-cover` + `group-hover:scale-110` do CSS.
    Image {
        path: PathBuf,
        opacity: f32,
        radius: f32,
        zoom: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub rect: Rect,
    /// Recorte herdado do container (scroll, card). `None` = tela toda.
    pub clip: Option<Rect>,
    pub visual: Visual,
}

/// Área que responde ao mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Hit {
    id: Id,
    rect: Rect,
    clip: Option<Rect>,
}

/// Posição de um controle `EDIT` nativo. A janela cria/move o filho real a
/// partir desta lista (R14: campo de texto é o único widget não desenhado).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditHost {
    pub id: Id,
    pub rect: Rect,
}

/// Resultado de uma passagem de construção.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Frame {
    pub nodes: Vec<Node>,
    hits: Vec<Hit>,
    edits: Vec<EditHost>,
}

impl Frame {
    /// Widget sob o ponto, do topo para baixo (o último desenhado ganha).
    pub fn hit_at(&self, x: f32, y: f32) -> Option<Id> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(x, y) && hit.clip.is_none_or(|clip| clip.contains(x, y)))
            .map(|hit| hit.id)
    }

    pub fn edits(&self) -> &[EditHost] {
        &self.edits
    }

    /// O widget existe nesta passagem? É o que a poda de hover pergunta quando
    /// a tela muda debaixo do cursor.
    pub fn has_hit(&self, id: Id) -> bool {
        self.hits.iter().any(|hit| hit.id == id)
    }
}

/// Mede texto. Separado de [`Painter`] porque a construção precisa medir (para
/// largura de aba, altura de parágrafo) sem ter um render target em mãos.
pub trait Measure {
    /// Largura e altura que o texto ocupa com `max_width` disponível.
    fn text_size(&mut self, text: &str, style: TextStyle, max_width: f32) -> (f32, f32);
}

/// Destino de desenho. Implementado sobre Direct2D em `gfx::d2d`.
pub trait Painter: Measure {
    fn clear(&mut self, color: Color);
    /// Recorte ativo dos próximos desenhos.
    fn set_clip(&mut self, clip: Option<Rect>);
    fn fill(&mut self, rect: Rect, radius: f32, color: Color);
    fn gradient(&mut self, rect: Rect, radius: f32, from: Color, to: Color);
    fn stroke(&mut self, rect: Rect, radius: f32, width: f32, color: Color);
    fn ellipse(&mut self, rect: Rect, color: Color);
    fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: Color);
    fn text(&mut self, rect: Rect, text: &str, style: TextStyle, color: Color);
    fn image(&mut self, rect: Rect, path: &Path, opacity: f32, radius: f32, zoom: f32);
}

/// Percorre a lista de desenho. É tudo o que acontece num `WM_PAINT`.
pub fn paint(frame: &Frame, painter: &mut dyn Painter) {
    for node in &frame.nodes {
        painter.set_clip(node.clip);
        match &node.visual {
            Visual::Fill { radius, color } => painter.fill(node.rect, *radius, *color),
            Visual::Gradient { radius, from, to } => {
                painter.gradient(node.rect, *radius, *from, *to)
            }
            Visual::Stroke {
                radius,
                width,
                color,
            } => painter.stroke(node.rect, *radius, *width, *color),
            Visual::Ellipse { color } => painter.ellipse(node.rect, *color),
            Visual::Line {
                from,
                to,
                width,
                color,
            } => painter.line(*from, *to, *width, *color),
            Visual::Text { text, style, color } => painter.text(node.rect, text, *style, *color),
            Visual::Image {
                path,
                opacity,
                radius,
                zoom,
            } => painter.image(node.rect, path, *opacity, *radius, *zoom),
        }
    }
    painter.set_clip(None);
}

/// Rolagem de um container, preservada entre passagens.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ScrollState {
    offset: f32,
    content: f32,
    view: Rect,
    touched: bool,
}

/// Valor animado de um widget (fade de hover, flash de disparo).
#[derive(Debug, Clone, Copy, PartialEq)]
struct AnimState {
    value: f32,
    target: f32,
    duration_ms: u32,
    updated_ms: u64,
    touched: bool,
}

/// Quanto uma "marcha" da roda rola, em DIP. Equivale às três linhas que o
/// Windows sugere por padrão.
const WHEEL_STEP_DIP: f32 = 60.0;

/// Evento de mouse já convertido para DIP, em coordenadas de cliente.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Move {
        x: f32,
        y: f32,
    },
    /// O cursor saiu da janela (`WM_MOUSELEAVE`).
    Leave,
    Down {
        x: f32,
        y: f32,
    },
    Up {
        x: f32,
        y: f32,
    },
    /// `delta` em marchas da roda (o `WHEEL_DELTA` já dividido), positivo = cima.
    Wheel {
        x: f32,
        y: f32,
        delta: f32,
    },
}

/// O que a janela faz depois de entregar um evento.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Response {
    /// Algo mudou: reconstruir a lista e invalidar.
    pub redraw: bool,
    pub clicked: Option<Id>,
}

/// Estado vivo da interface. Uma instância por janela.
pub struct Ui {
    now_ms: u64,
    mouse: Option<(f32, f32)>,
    hot: Option<Id>,
    pressed: Option<Id>,
    scrolls: Vec<(Id, ScrollState)>,
    anims: Vec<(Id, AnimState)>,
    frame: Frame,
    clips: Vec<Rect>,
    animating: bool,
}

impl Default for Ui {
    fn default() -> Ui {
        Ui::new()
    }
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            now_ms: 0,
            mouse: None,
            hot: None,
            pressed: None,
            scrolls: Vec::new(),
            anims: Vec::new(),
            frame: Frame::default(),
            clips: Vec::new(),
            animating: false,
        }
    }

    // --- Passagem de construção ---

    /// Abre a passagem. `now_ms` é um relógio monotônico em milissegundos
    /// (`GetTickCount64` na janela, valor fixo nos testes).
    pub fn begin(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.frame.nodes.clear();
        self.frame.hits.clear();
        self.frame.edits.clear();
        self.clips.clear();
        self.animating = false;
        for (_, state) in &mut self.scrolls {
            state.touched = false;
        }
        for (_, state) in &mut self.anims {
            state.touched = false;
        }
    }

    /// Fecha a passagem e descarta o estado de widgets que sumiram da tela
    /// (troca de aba, item filtrado pela busca).
    pub fn end(&mut self) {
        self.scrolls.retain(|(_, state)| state.touched);
        self.anims.retain(|(_, state)| state.touched);
        if self.hot.is_some_and(|id| !self.frame.has_hit(id)) {
            self.hot = None;
        }
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// Há animação em curso: a janela mantém o timer de 16ms vivo.
    pub fn animating(&self) -> bool {
        self.animating
    }

    pub fn hot(&self) -> Option<Id> {
        self.hot
    }

    pub fn is_hot(&self, id: Id) -> bool {
        self.hot == Some(id)
    }

    /// Botão pressionado agora (mouse baixo sobre ele, ainda sem soltar).
    pub fn is_pressed(&self, id: Id) -> bool {
        self.pressed == Some(id) && self.hot == Some(id)
    }

    fn clip(&self) -> Option<Rect> {
        self.clips.last().copied()
    }

    fn push_node(&mut self, rect: Rect, visual: Visual) {
        if rect.is_empty() {
            return;
        }
        let clip = self.clip();
        self.frame.nodes.push(Node { rect, clip, visual });
    }

    pub fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        self.push_node(rect, Visual::Fill { radius, color });
    }

    pub fn gradient(&mut self, rect: Rect, radius: f32, from: Color, to: Color) {
        self.push_node(rect, Visual::Gradient { radius, from, to });
    }

    pub fn stroke(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.push_node(
            rect,
            Visual::Stroke {
                radius,
                width,
                color,
            },
        );
    }

    /// Brilho aproximado: traço externo semitransparente na cor do acento, no
    /// lugar do `box-shadow` com blur do CSS (R5).
    pub fn glow(&mut self, rect: Rect, radius: f32, color: Color) {
        self.stroke(
            rect.inset(-theme::GLOW_WIDTH / 2.0),
            radius + theme::GLOW_WIDTH / 2.0,
            theme::GLOW_WIDTH,
            color.alpha(color.a * theme::GLOW_ALPHA),
        );
    }

    pub fn ellipse(&mut self, rect: Rect, color: Color) {
        self.push_node(rect, Visual::Ellipse { color });
    }

    /// Segmento entre dois pontos. O retângulo do nó é a caixa que o traço
    /// ocupa — um segmento vertical tem largura zero, e sem a folga da espessura
    /// ele seria descartado como vazio.
    pub fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: Color) {
        let half = width / 2.0;
        let rect = Rect {
            x: from.0.min(to.0) - half,
            y: from.1.min(to.1) - half,
            w: (from.0 - to.0).abs() + width,
            h: (from.1 - to.1).abs() + width,
        };
        self.push_node(
            rect,
            Visual::Line {
                from,
                to,
                width,
                color,
            },
        );
    }

    pub fn text(&mut self, rect: Rect, text: impl Into<String>, style: TextStyle, color: Color) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.push_node(rect, Visual::Text { text, style, color });
    }

    /// `path` é relativo à raiz de `assets/` (ex.: `icons/stratagems/x.webp`).
    pub fn image(&mut self, rect: Rect, path: impl Into<PathBuf>, opacity: f32) {
        self.image_rounded(rect, path, opacity, 0.0, 1.0);
    }

    /// Imagem recortada num retângulo arredondado e, com `zoom > 1`, ampliada
    /// em torno do centro — o que sobra para fora do recorte é cortado.
    pub fn image_rounded(
        &mut self,
        rect: Rect,
        path: impl Into<PathBuf>,
        opacity: f32,
        radius: f32,
        zoom: f32,
    ) {
        self.push_node(
            rect,
            Visual::Image {
                path: path.into(),
                opacity,
                radius,
                zoom,
            },
        );
    }

    /// Registra uma área clicável. Chamar depois de desenhar o widget.
    pub fn hit(&mut self, id: Id, rect: Rect) {
        if rect.is_empty() {
            return;
        }
        let clip = self.clip();
        self.frame.hits.push(Hit { id, rect, clip });
    }

    /// Reserva o lugar de um `EDIT` nativo; a janela posiciona o filho real.
    pub fn edit(&mut self, id: Id, rect: Rect) {
        self.frame.edits.push(EditHost { id, rect });
    }

    pub fn push_clip(&mut self, rect: Rect) {
        let clip = match self.clip() {
            Some(current) => current.intersect(rect),
            None => rect,
        };
        self.clips.push(clip);
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    // --- Animação ---

    fn anim_mut(&mut self, id: Id, duration_ms: u32) -> &mut AnimState {
        let now = self.now_ms;
        if let Some(index) = self.anims.iter().position(|(key, _)| *key == id) {
            let state = &mut self.anims[index].1;
            state.duration_ms = duration_ms;
            return state;
        }
        self.anims.push((
            id,
            AnimState {
                value: 0.0,
                target: 0.0,
                duration_ms,
                updated_ms: now,
                touched: true,
            },
        ));
        &mut self.anims.last_mut().expect("recém-inserido").1
    }

    /// Valor 0..1 que persegue `on`. Chamado na construção; enquanto não chega
    /// ao destino, [`Ui::animating`] fica ligado e a janela agenda o próximo
    /// quadro.
    pub fn fade(&mut self, id: Id, on: bool, duration_ms: u32) -> f32 {
        let now = self.now_ms;
        let (value, target) = {
            let state = self.anim_mut(id, duration_ms);
            state.target = if on { 1.0 } else { 0.0 };
            (advance(state, now), state.target)
        };
        if value != target {
            self.animating = true;
        }
        value
    }

    /// Dispara um pulso 1→0 (flash de slot). Chamado fora da construção, ao
    /// tratar um evento.
    pub fn flash(&mut self, id: Id, duration_ms: u32) {
        let now = self.now_ms;
        let state = self.anim_mut(id, duration_ms);
        state.value = 1.0;
        state.target = 0.0;
        state.updated_ms = now;
        state.touched = true;
    }

    /// Valor atual de um pulso, 1 no disparo e 0 quando acaba.
    pub fn anim(&mut self, id: Id, duration_ms: u32) -> f32 {
        let now = self.now_ms;
        let (value, target) = {
            let state = self.anim_mut(id, duration_ms);
            (advance(state, now), state.target)
        };
        if value != target {
            self.animating = true;
        }
        value
    }

    // --- Rolagem ---

    fn scroll_mut(&mut self, id: Id) -> &mut ScrollState {
        if let Some(index) = self.scrolls.iter().position(|(key, _)| *key == id) {
            return &mut self.scrolls[index].1;
        }
        self.scrolls.push((id, ScrollState::default()));
        &mut self.scrolls.last_mut().expect("recém-inserido").1
    }

    /// Abre um container rolável e devolve o deslocamento atual em DIP. O
    /// conteúdo deve ser desenhado a partir de `view.y - offset`.
    pub fn scroll_begin(&mut self, id: Id, view: Rect) -> f32 {
        let state = self.scroll_mut(id);
        state.view = view;
        state.touched = true;
        let offset = state.offset;
        self.push_clip(view);
        offset
    }

    /// Fecha o container: prende o deslocamento ao conteúdo real e desenha a
    /// barra (5px, polegar slate — `scrollbar-hd` do legado).
    pub fn scroll_end(&mut self, id: Id, view: Rect, content_height: f32) {
        self.pop_clip();

        let state = self.scroll_mut(id);
        state.content = content_height;
        state.offset = clamp_offset(state.offset, view.h, content_height);
        let (offset, max) = (state.offset, (content_height - view.h).max(0.0));
        if max <= 0.0 {
            return;
        }

        let width = theme::SCROLLBAR_WIDTH;
        let track = Rect::new(view.right() - width, view.y, width, view.h);
        let thumb_h = (view.h * view.h / content_height).max(24.0);
        let thumb_y = view.y + (view.h - thumb_h) * (offset / max);
        self.fill(
            Rect::new(track.x, thumb_y, width, thumb_h),
            width / 2.0,
            theme::SCROLL_THUMB,
        );
    }

    // --- Entrada ---

    /// Entrega um evento de mouse. Não redesenha nada: devolve o que a janela
    /// precisa fazer.
    pub fn input(&mut self, event: Input) -> Response {
        match event {
            Input::Move { x, y } => {
                self.mouse = Some((x, y));
                let hot = self.frame.hit_at(x, y);
                let changed = hot != self.hot;
                self.hot = hot;
                Response {
                    redraw: changed,
                    clicked: None,
                }
            }
            Input::Leave => {
                self.mouse = None;
                let changed = self.hot.is_some() || self.pressed.is_some();
                self.hot = None;
                self.pressed = None;
                Response {
                    redraw: changed,
                    clicked: None,
                }
            }
            Input::Down { x, y } => {
                self.mouse = Some((x, y));
                self.hot = self.frame.hit_at(x, y);
                self.pressed = self.hot;
                Response {
                    redraw: self.pressed.is_some(),
                    clicked: None,
                }
            }
            Input::Up { x, y } => {
                self.mouse = Some((x, y));
                self.hot = self.frame.hit_at(x, y);
                // Clique só conta quando soltou em cima de quem foi pressionado
                // — arrastar para fora cancela, como em qualquer botão nativo.
                let clicked = match self.pressed.take() {
                    Some(pressed) if Some(pressed) == self.hot => Some(pressed),
                    Some(_) => None,
                    None => None,
                };
                Response {
                    redraw: true,
                    clicked,
                }
            }
            Input::Wheel { x, y, delta } => {
                let step = delta * WHEEL_STEP_DIP;
                let target = self
                    .scrolls
                    .iter()
                    .rev()
                    .find(|(_, state)| state.view.contains(x, y) && state.content > state.view.h)
                    .map(|(id, _)| *id);
                let Some(id) = target else {
                    return Response::default();
                };
                let state = self.scroll_mut(id);
                let before = state.offset;
                state.offset = clamp_offset(state.offset - step, state.view.h, state.content);
                Response {
                    redraw: state.offset != before,
                    clicked: None,
                }
            }
        }
    }
}

fn clamp_offset(offset: f32, view_height: f32, content_height: f32) -> f32 {
    offset.clamp(0.0, (content_height - view_height).max(0.0))
}

/// Caminha o valor rumo ao alvo pelo tempo decorrido. Linear: a curva do CSS era
/// um `ease` de 300ms, e a diferença some num fade tão curto.
fn advance(state: &mut AnimState, now_ms: u64) -> f32 {
    state.touched = true;
    let elapsed = now_ms.saturating_sub(state.updated_ms);
    state.updated_ms = now_ms;

    if state.duration_ms == 0 {
        state.value = state.target;
        return state.value;
    }

    let step = elapsed as f32 / state.duration_ms as f32;
    let delta = state.target - state.value;
    state.value = if delta.abs() <= step {
        state.target
    } else {
        state.value + step * delta.signum()
    };
    state.value
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABEL: TextStyle = TextStyle::new(10.0, Weight::Black);

    /// Painter de teste: registra as chamadas e mede texto por uma largura fixa
    /// de caractere, o bastante para a construção decidir layouts.
    #[derive(Default)]
    struct Recorder {
        calls: Vec<String>,
        clip: Option<Rect>,
    }

    impl Measure for Recorder {
        fn text_size(&mut self, text: &str, style: TextStyle, _max_width: f32) -> (f32, f32) {
            (text.chars().count() as f32 * style.size * 0.6, style.size)
        }
    }

    impl Painter for Recorder {
        fn clear(&mut self, _color: Color) {
            self.calls.push("clear".into());
        }
        fn set_clip(&mut self, clip: Option<Rect>) {
            self.clip = clip;
        }
        fn fill(&mut self, rect: Rect, _radius: f32, _color: Color) {
            self.calls.push(format!("fill {} {}", rect.x, rect.y));
        }
        fn gradient(&mut self, _rect: Rect, _radius: f32, _from: Color, _to: Color) {
            self.calls.push("gradient".into());
        }
        fn stroke(&mut self, _rect: Rect, _radius: f32, _width: f32, _color: Color) {
            self.calls.push("stroke".into());
        }
        fn ellipse(&mut self, _rect: Rect, _color: Color) {
            self.calls.push("ellipse".into());
        }
        fn line(&mut self, from: (f32, f32), to: (f32, f32), _width: f32, _color: Color) {
            self.calls
                .push(format!("line {} {} {} {}", from.0, from.1, to.0, to.1));
        }
        fn text(&mut self, _rect: Rect, text: &str, _style: TextStyle, _color: Color) {
            self.calls.push(format!("text {text}"));
        }
        fn image(&mut self, _rect: Rect, path: &Path, _opacity: f32, _radius: f32, _zoom: f32) {
            self.calls.push(format!("image {}", path.display()));
        }
    }

    #[test]
    fn ids_are_stable_and_distinct() {
        assert_eq!(id("tab"), id("tab"));
        assert_ne!(id("tab"), id("tabs"));
        assert_ne!(id_at("slot", 0), id_at("slot", 1));
        assert_ne!(id_at("slot", 0), id("slot"));
    }

    #[test]
    fn cutting_a_rect_consumes_it() {
        let mut rect = Rect::new(0.0, 0.0, 100.0, 60.0);
        let top = rect.cut_top(20.0);
        assert_eq!(top, Rect::new(0.0, 0.0, 100.0, 20.0));
        assert_eq!(rect, Rect::new(0.0, 20.0, 100.0, 40.0));

        let right = rect.cut_right(30.0);
        assert_eq!(right, Rect::new(70.0, 20.0, 30.0, 40.0));
        assert_eq!(rect.w, 70.0);

        // Cortar mais do que sobra devolve o que existe, sem tamanho negativo.
        let all = rect.cut_bottom(999.0);
        assert_eq!(all.h, 40.0);
        assert!(rect.is_empty());
    }

    #[test]
    fn layout_helpers_split_evenly() {
        let cols = columns(Rect::new(0.0, 0.0, 100.0, 10.0), 3, 5.0);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].w, 30.0);
        assert_eq!(cols[1].x, 35.0);
        assert_eq!(cols[2].right(), 100.0);

        let cell = grid_cell(Rect::new(0.0, 0.0, 100.0, 200.0), 4, 40.0, 4.0, 5);
        assert_eq!(cell.y, 44.0, "segundo item da segunda linha");
        assert_eq!(cell.x, 26.0);
        assert_eq!(grid_height(9, 4, 40.0, 4.0), 3.0 * 40.0 + 2.0 * 4.0);
        assert_eq!(grid_height(0, 4, 40.0, 4.0), 0.0);
    }

    fn frame_with_two_buttons(ui: &mut Ui) {
        ui.begin(0);
        ui.fill(Rect::new(0.0, 0.0, 50.0, 20.0), 0.0, theme::CARD_BG);
        ui.hit(id("a"), Rect::new(0.0, 0.0, 50.0, 20.0));
        ui.hit(id("b"), Rect::new(50.0, 0.0, 50.0, 20.0));
        ui.end();
    }

    #[test]
    fn hover_only_redraws_when_it_changes() {
        let mut ui = Ui::new();
        frame_with_two_buttons(&mut ui);

        let first = ui.input(Input::Move { x: 10.0, y: 10.0 });
        assert!(first.redraw);
        assert!(ui.is_hot(id("a")));

        // Mexer dentro do mesmo widget não repinta nada.
        assert!(!ui.input(Input::Move { x: 20.0, y: 10.0 }).redraw);

        assert!(ui.input(Input::Move { x: 60.0, y: 10.0 }).redraw);
        assert!(ui.is_hot(id("b")));

        // Fora de qualquer widget o hover cai (uma vez só).
        assert!(ui.input(Input::Move { x: 10.0, y: 90.0 }).redraw);
        assert!(!ui.input(Input::Move { x: 12.0, y: 90.0 }).redraw);
        assert_eq!(ui.hot(), None);
    }

    #[test]
    fn a_click_needs_press_and_release_on_the_same_widget() {
        let mut ui = Ui::new();
        frame_with_two_buttons(&mut ui);

        ui.input(Input::Down { x: 10.0, y: 10.0 });
        assert!(ui.is_pressed(id("a")));
        assert_eq!(
            ui.input(Input::Up { x: 10.0, y: 10.0 }).clicked,
            Some(id("a"))
        );

        // Arrastar para fora antes de soltar cancela.
        ui.input(Input::Down { x: 10.0, y: 10.0 });
        assert_eq!(ui.input(Input::Up { x: 60.0, y: 10.0 }).clicked, None);

        // E soltar sem ter pressionado nada também.
        assert_eq!(ui.input(Input::Up { x: 10.0, y: 10.0 }).clicked, None);
        assert!(!ui.is_pressed(id("a")));
    }

    #[test]
    fn clipped_widgets_do_not_answer_outside_their_container() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.push_clip(Rect::new(0.0, 0.0, 100.0, 50.0));
        ui.hit(id("inside"), Rect::new(0.0, 40.0, 100.0, 40.0));
        ui.pop_clip();
        ui.end();

        assert_eq!(ui.frame().hit_at(10.0, 45.0), Some(id("inside")));
        assert_eq!(
            ui.frame().hit_at(10.0, 70.0),
            None,
            "a parte que passou do container não é clicável"
        );
    }

    #[test]
    fn the_topmost_widget_wins_the_hit() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.hit(id("below"), Rect::new(0.0, 0.0, 100.0, 100.0));
        ui.hit(id("above"), Rect::new(0.0, 0.0, 50.0, 50.0));
        ui.end();

        assert_eq!(ui.frame().hit_at(10.0, 10.0), Some(id("above")));
        assert_eq!(ui.frame().hit_at(80.0, 10.0), Some(id("below")));
    }

    #[test]
    fn wheel_scrolls_only_inside_an_overflowing_container() {
        let view = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut ui = Ui::new();

        ui.begin(0);
        let offset = ui.scroll_begin(id("list"), view);
        assert_eq!(offset, 0.0);
        ui.scroll_end(id("list"), view, 400.0);
        ui.end();

        assert!(
            ui.input(Input::Wheel {
                x: 50.0,
                y: 50.0,
                delta: -1.0
            })
            .redraw
        );
        ui.begin(0);
        assert_eq!(ui.scroll_begin(id("list"), view), WHEEL_STEP_DIP);
        ui.scroll_end(id("list"), view, 400.0);
        ui.end();

        // Fora do container a roda não mexe em nada.
        assert!(
            !ui.input(Input::Wheel {
                x: 200.0,
                y: 50.0,
                delta: -1.0
            })
            .redraw
        );

        // E o deslocamento é preso ao conteúdo nas duas pontas.
        for _ in 0..20 {
            ui.input(Input::Wheel {
                x: 50.0,
                y: 50.0,
                delta: -1.0,
            });
        }
        ui.begin(0);
        assert_eq!(ui.scroll_begin(id("list"), view), 300.0);
        ui.scroll_end(id("list"), view, 400.0);
        ui.end();

        for _ in 0..20 {
            ui.input(Input::Wheel {
                x: 50.0,
                y: 50.0,
                delta: 1.0,
            });
        }
        ui.begin(0);
        assert_eq!(ui.scroll_begin(id("list"), view), 0.0);
        ui.scroll_end(id("list"), view, 400.0);
        ui.end();
    }

    #[test]
    fn a_scrollbar_appears_only_when_the_content_overflows() {
        let view = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut ui = Ui::new();

        ui.begin(0);
        ui.scroll_begin(id("list"), view);
        ui.scroll_end(id("list"), view, 80.0);
        ui.end();
        assert!(ui.frame().nodes.is_empty(), "conteúdo cabe: sem barra");

        ui.begin(0);
        ui.scroll_begin(id("list"), view);
        ui.scroll_end(id("list"), view, 400.0);
        ui.end();
        assert_eq!(ui.frame().nodes.len(), 1);
        assert_eq!(ui.frame().nodes[0].rect.w, theme::SCROLLBAR_WIDTH);
    }

    #[test]
    fn fades_run_until_they_reach_the_target_and_then_stop() {
        let mut ui = Ui::new();

        ui.begin(0);
        assert_eq!(ui.fade(id("btn"), true, 200), 0.0);
        ui.end();
        assert!(ui.animating(), "acabou de começar a subir");

        ui.begin(100);
        assert_eq!(ui.fade(id("btn"), true, 200), 0.5);
        ui.end();
        assert!(ui.animating());

        ui.begin(300);
        assert_eq!(ui.fade(id("btn"), true, 200), 1.0);
        ui.end();
        assert!(!ui.animating(), "chegou no destino: o timer pode morrer");

        // E volta quando o hover sai.
        ui.begin(400);
        assert_eq!(ui.fade(id("btn"), false, 200), 0.5);
        ui.end();
        assert!(ui.animating());
    }

    #[test]
    fn a_flash_decays_to_zero() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.end();

        ui.flash(id("slot"), 500);
        ui.begin(0);
        assert_eq!(ui.anim(id("slot"), 500), 1.0);
        ui.end();
        assert!(ui.animating());

        ui.begin(250);
        assert_eq!(ui.anim(id("slot"), 500), 0.5);
        ui.end();

        ui.begin(1000);
        assert_eq!(ui.anim(id("slot"), 500), 0.0);
        ui.end();
        assert!(!ui.animating());
    }

    #[test]
    fn state_of_widgets_that_left_the_screen_is_dropped() {
        let mut ui = Ui::new();
        let view = Rect::new(0.0, 0.0, 100.0, 100.0);

        ui.begin(0);
        ui.scroll_begin(id("list"), view);
        ui.scroll_end(id("list"), view, 400.0);
        ui.fade(id("btn"), true, 200);
        ui.hit(id("btn"), view);
        ui.end();
        assert_eq!(ui.scrolls.len(), 1);
        assert_eq!(ui.anims.len(), 1);

        ui.input(Input::Move { x: 10.0, y: 10.0 });
        assert!(ui.is_hot(id("btn")));

        // Outra aba: nem a lista nem o botão são construídos.
        ui.begin(0);
        ui.end();
        assert!(ui.scrolls.is_empty());
        assert!(ui.anims.is_empty());
        assert_eq!(
            ui.hot(),
            None,
            "o hover não fica preso num widget que sumiu"
        );
    }

    #[test]
    fn painting_walks_the_list_in_order() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.fill(Rect::new(1.0, 2.0, 10.0, 10.0), 4.0, theme::CARD_BG);
        ui.text(
            Rect::new(0.0, 0.0, 50.0, 10.0),
            "MACROS",
            LABEL,
            theme::TEXT,
        );
        ui.image(Rect::new(0.0, 0.0, 10.0, 10.0), "icons/tray.png", 1.0);
        ui.end();

        let mut painter = Recorder::default();
        paint(ui.frame(), &mut painter);
        assert_eq!(
            painter.calls,
            vec!["fill 1 2", "text MACROS", "image icons/tray.png"]
        );
        assert_eq!(painter.clip, None, "o recorte é solto no fim");
    }

    #[test]
    fn a_line_keeps_its_endpoints_and_gets_a_box_with_the_stroke_width() {
        let mut ui = Ui::new();
        ui.begin(0);
        // Vertical: sem a folga da espessura o retângulo sairia vazio.
        ui.line((10.0, 4.0), (10.0, 20.0), 2.0, theme::CYAN);
        ui.end();

        let node = &ui.frame().nodes[0];
        assert_eq!(node.rect, Rect::new(9.0, 3.0, 2.0, 18.0));

        let mut painter = Recorder::default();
        paint(ui.frame(), &mut painter);
        assert_eq!(painter.calls, vec!["line 10 4 10 20"]);
    }

    #[test]
    fn an_image_carries_its_corner_radius_and_zoom() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.image(Rect::new(0.0, 0.0, 10.0, 10.0), "icons/tray.png", 1.0);
        ui.image_rounded(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            "icons/tray.png",
            0.7,
            16.0,
            1.1,
        );
        ui.end();

        assert_eq!(
            ui.frame().nodes[0].visual,
            Visual::Image {
                path: "icons/tray.png".into(),
                opacity: 1.0,
                radius: 0.0,
                zoom: 1.0
            },
            "o atalho desenha a imagem inteira, sem recorte nem ampliação"
        );
        assert!(matches!(
            ui.frame().nodes[1].visual,
            Visual::Image {
                radius: 16.0,
                zoom: 1.1,
                ..
            }
        ));
    }

    #[test]
    fn empty_shapes_and_strings_never_reach_the_painter() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.fill(Rect::new(0.0, 0.0, 0.0, 10.0), 0.0, theme::CARD_BG);
        ui.text(Rect::new(0.0, 0.0, 50.0, 10.0), "", LABEL, theme::TEXT);
        ui.hit(id("nada"), Rect::ZERO);
        ui.end();

        assert!(ui.frame().nodes.is_empty());
        assert_eq!(ui.frame().hit_at(0.0, 0.0), None);
    }

    #[test]
    fn nested_clips_intersect() {
        let mut ui = Ui::new();
        ui.begin(0);
        ui.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
        ui.push_clip(Rect::new(50.0, 0.0, 100.0, 40.0));
        ui.fill(Rect::new(0.0, 0.0, 200.0, 200.0), 0.0, theme::CARD_BG);
        ui.pop_clip();
        ui.pop_clip();
        ui.end();

        assert_eq!(
            ui.frame().nodes[0].clip,
            Some(Rect::new(50.0, 0.0, 50.0, 40.0))
        );
    }
}
