//! Direct2D: fábrica, render targets e o painter que executa a lista de desenho.
//!
//! São dois destinos, com o mesmo painter em cima (é o que o trait
//! [`Painter`](crate::ui::toolkit::Painter) resolve):
//!
//! - [`WindowTarget`] — `ID2D1HwndRenderTarget` da janela principal, que
//!   apresenta sozinho e sobrevive a perda de dispositivo recriando tudo.
//! - [`LayeredSurface`] — DIB de 32 bits pré-multiplicado + `ID2D1DCRenderTarget`,
//!   o par que o `UpdateLayeredWindow` do overlay exige (Fase 9).
//!
//! O render target recebe o DPI do monitor, então todo desenho continua em DIP:
//! a escala é aplicada pelo próprio Direct2D, sem arredondamento manual.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use windows::core::Result;
use windows::Win32::Foundation::{D2DERR_RECREATE_TARGET, HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_IGNORE, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_GRADIENT_STOP,
    D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Brush, ID2D1DCRenderTarget, ID2D1Factory1, ID2D1HwndRenderTarget,
    ID2D1LinearGradientBrush, ID2D1RenderTarget, ID2D1SolidColorBrush, ID2D1StrokeStyle,
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_BITMAP_BRUSH_PROPERTIES,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_CAP_STYLE_ROUND, D2D1_DASH_STYLE_SOLID,
    D2D1_DEBUG_LEVEL_NONE, D2D1_ELLIPSE, D2D1_EXTEND_MODE_CLAMP, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_MULTI_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_GAMMA_2_2,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES,
    D2D1_LINE_JOIN_ROUND, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1_STROKE_STYLE_PROPERTIES1, D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};

use crate::gfx::images::BitmapCache;
use crate::gfx::text::Text;
use crate::ui::theme::{Color, BASE_DPI};
use crate::ui::toolkit::{self, Frame, ImageStyle, Measure, Painter, Rect, TextStyle};

/// Fábrica do processo. Multi-threaded porque a thread do overlay (Fase 9) cria
/// o próprio target a partir dela.
static FACTORY: OnceLock<ID2D1Factory1> = OnceLock::new();

pub fn factory() -> Result<&'static ID2D1Factory1> {
    if let Some(factory) = FACTORY.get() {
        return Ok(factory);
    }
    let options = D2D1_FACTORY_OPTIONS {
        debugLevel: D2D1_DEBUG_LEVEL_NONE,
    };
    // SAFETY: o tipo pedido casa com o `riid` derivado pelo binding, e as
    // opções vivem durante a chamada.
    let created: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, Some(&options))? };
    // Numa corrida entre threads uma das fábricas é descartada; as duas são
    // válidas, e quem perde só solta a sua.
    Ok(FACTORY.get_or_init(|| created))
}

/// Traço de pontas e junções arredondadas, do jeito que os ícones do legado
/// (SVG com `stroke-linecap="round"`) desenhavam as setas do codex. É um recurso
/// da fábrica, não do dispositivo: vale para o processo inteiro.
fn round_stroke() -> Option<&'static ID2D1StrokeStyle> {
    static STYLE: OnceLock<Option<ID2D1StrokeStyle>> = OnceLock::new();
    STYLE
        .get_or_init(|| {
            // A fábrica 1.1 só expõe a variante com tipo de transformação; o
            // "normal" é o comportamento da versão antiga (a espessura segue a
            // escala do target, que é como o DPI entra no traço).
            let properties = D2D1_STROKE_STYLE_PROPERTIES1 {
                startCap: D2D1_CAP_STYLE_ROUND,
                endCap: D2D1_CAP_STYLE_ROUND,
                dashCap: D2D1_CAP_STYLE_ROUND,
                lineJoin: D2D1_LINE_JOIN_ROUND,
                miterLimit: 10.0,
                dashStyle: D2D1_DASH_STYLE_SOLID,
                dashOffset: 0.0,
                transformType: D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
            };
            // SAFETY: as propriedades vivem durante a chamada e não há traçado.
            let style = unsafe { factory().ok()?.CreateStrokeStyle(&properties, None) };
            match style {
                Ok(style) => Some(style.into()),
                Err(err) => {
                    log::warn!("CreateStrokeStyle falhou ({err}); traços com ponta reta");
                    None
                }
            }
        })
        .as_ref()
}

fn target_properties(dpi: u32, opaque: bool) -> D2D1_RENDER_TARGET_PROPERTIES {
    let dpi = dpi as f32;
    D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: if opaque {
                D2D1_ALPHA_MODE_IGNORE
            } else {
                D2D1_ALPHA_MODE_PREMULTIPLIED
            },
        },
        dpiX: dpi,
        dpiY: dpi,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    }
}

/// Render target da janela principal.
pub struct WindowTarget {
    hwnd: HWND,
    dpi: u32,
    size: D2D_SIZE_U,
    target: Option<ID2D1HwndRenderTarget>,
    brushes: BrushCache,
    images: BitmapCache,
}

impl WindowTarget {
    pub fn new(hwnd: HWND, dpi: u32) -> WindowTarget {
        WindowTarget {
            hwnd,
            dpi,
            size: D2D_SIZE_U {
                width: 0,
                height: 0,
            },
            target: None,
            brushes: BrushCache::default(),
            images: BitmapCache::default(),
        }
    }

    /// Novo tamanho de cliente, em pixels.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = D2D_SIZE_U { width, height };
        let Some(target) = &self.target else {
            return;
        };
        // SAFETY: target vivo; falha aqui só acontece em perda de dispositivo,
        // e aí o próximo `draw` recria tudo.
        if let Err(err) = unsafe { target.Resize(&self.size) } {
            log::debug!("Resize recusado ({err}); recriando o render target");
            self.discard();
        }
    }

    pub fn set_dpi(&mut self, dpi: u32) {
        self.dpi = dpi;
        if let Some(target) = &self.target {
            // SAFETY: target vivo; `SetDpi` não falha.
            unsafe { target.SetDpi(dpi as f32, dpi as f32) };
        }
    }

    /// Solta o target e tudo que pertence a ele. O próximo desenho recria.
    pub fn discard(&mut self) {
        self.target = None;
        self.brushes.clear();
        self.images.clear();
    }

    fn ensure(&mut self) -> Result<ID2D1HwndRenderTarget> {
        if let Some(target) = &self.target {
            return Ok(target.clone());
        }
        if self.size.width == 0 || self.size.height == 0 {
            // Um `WM_PAINT` antes do primeiro `WM_SIZE` criaria um target de
            // tamanho zero, que nunca desenharia nada.
            let mut client = RECT::default();
            // SAFETY: retângulo próprio, vivo durante a chamada.
            if unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetClientRect(self.hwnd, &mut client)
            }
            .is_ok()
            {
                self.size = D2D_SIZE_U {
                    width: (client.right - client.left).max(1) as u32,
                    height: (client.bottom - client.top).max(1) as u32,
                };
            }
        }
        let properties = target_properties(self.dpi, true);
        let hwnd_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: self.hwnd,
            pixelSize: self.size,
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        // SAFETY: a janela existe enquanto este target existe (ambos morrem no
        // `WM_DESTROY`), e as duas estruturas vivem durante a chamada.
        let target = unsafe { factory()?.CreateHwndRenderTarget(&properties, &hwnd_properties)? };
        Ok(self.target.insert(target).clone())
    }

    /// Pinta a lista de desenho. Perda de dispositivo é tratada aqui: o target
    /// é descartado e a próxima pintura recomeça com um novo.
    pub fn draw(&mut self, text: &mut Text, frame: &Frame, background: Color) {
        let target = match self.ensure() {
            Ok(target) => target,
            Err(err) => {
                log::error!("render target indisponível: {err}");
                return;
            }
        };

        let render: &ID2D1RenderTarget = &target;
        // SAFETY: par `BeginDraw`/`EndDraw` fechado logo abaixo; o painter só
        // desenha entre os dois.
        unsafe { render.BeginDraw() };
        {
            let mut painter = D2dPainter {
                target: render,
                brushes: &mut self.brushes,
                images: &mut self.images,
                text,
                clip: None,
            };
            painter.clear(background);
            toolkit::paint(frame, &mut painter);
        }
        // SAFETY: fecha o par aberto acima.
        if let Err(err) = unsafe { render.EndDraw(None, None) } {
            if err.code() == D2DERR_RECREATE_TARGET {
                log::info!("dispositivo Direct2D perdido; recriando o render target");
            } else {
                log::warn!("EndDraw falhou: {err}");
            }
            self.discard();
        }
    }
}

/// Superfície em memória para janelas layered: DIB pré-multiplicado desenhado
/// pelo Direct2D e apresentado com `UpdateLayeredWindow`.
pub struct LayeredSurface {
    target: ID2D1DCRenderTarget,
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    width: i32,
    height: i32,
    brushes: BrushCache,
    images: BitmapCache,
}

impl LayeredSurface {
    /// Cria a superfície com o tamanho em pixels informado.
    pub fn new(width: i32, height: i32, dpi: u32) -> Result<LayeredSurface> {
        let properties = target_properties(dpi, false);
        // SAFETY: propriedades vivem durante a chamada.
        let target = unsafe { factory()?.CreateDCRenderTarget(&properties)? };

        let mut surface = LayeredSurface {
            target,
            dc: HDC::default(),
            bitmap: HBITMAP::default(),
            previous: HGDIOBJ::default(),
            width: 0,
            height: 0,
            brushes: BrushCache::default(),
            images: BitmapCache::default(),
        };
        surface.resize(width.max(1), height.max(1));
        Ok(surface)
    }

    /// Troca o DIB por um de outro tamanho. Sem efeito quando o tamanho é o mesmo.
    pub fn resize(&mut self, width: i32, height: i32) {
        let (width, height) = (width.max(1), height.max(1));
        if self.width == width && self.height == height && !self.dc.is_invalid() {
            return;
        }
        self.release();

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Altura negativa: linhas de cima para baixo, na mesma ordem do
                // Direct2D — sem isso a janela sai de cabeça para baixo.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        // SAFETY: DC de memória e DIB do tamanho pedido; os handles ficam
        // guardados e são liberados em `release`.
        unsafe {
            let dc = CreateCompatibleDC(None);
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0);
            match bitmap {
                Ok(bitmap) => {
                    self.previous = SelectObject(dc, bitmap.into());
                    self.dc = dc;
                    self.bitmap = bitmap;
                    self.width = width;
                    self.height = height;
                }
                Err(err) => {
                    log::error!("CreateDIBSection falhou: {err}");
                    let _ = DeleteDC(dc);
                }
            }
        }
    }

    /// Novo DPI do monitor. O desenho continua em DIP: quem multiplica é o
    /// render target, então só ele precisa saber da troca.
    pub fn set_dpi(&mut self, dpi: u32) {
        let target: &ID2D1RenderTarget = &self.target;
        // SAFETY: target vivo; `SetDpi` não falha.
        unsafe { target.SetDpi(dpi as f32, dpi as f32) };
    }

    pub fn hdc(&self) -> HDC {
        self.dc
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Desenha a lista no DIB. O fundo entra transparente: quem compõe é o
    /// `UpdateLayeredWindow`, com o jogo atrás.
    pub fn draw(&mut self, text: &mut Text, frame: &Frame) {
        if self.dc.is_invalid() {
            return;
        }
        let bind = RECT {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        };
        // SAFETY: DC e retângulo vivos; o target passa a apontar para o DIB.
        if let Err(err) = unsafe { self.target.BindDC(self.dc, &bind) } {
            log::warn!("BindDC falhou: {err}");
            return;
        }

        let render: &ID2D1RenderTarget = &self.target;
        // SAFETY: par `BeginDraw`/`EndDraw` fechado abaixo.
        unsafe { render.BeginDraw() };
        {
            let mut painter = D2dPainter {
                target: render,
                brushes: &mut self.brushes,
                images: &mut self.images,
                text,
                clip: None,
            };
            painter.clear(Color::rgba(0x000000, 0.0));
            toolkit::paint(frame, &mut painter);
        }
        // SAFETY: fecha o par.
        if let Err(err) = unsafe { render.EndDraw(None, None) } {
            log::warn!("EndDraw da superfície layered falhou: {err}");
            self.brushes.clear();
            self.images.clear();
        }
    }

    fn release(&mut self) {
        // SAFETY: handles nossos; cada um só é liberado uma vez.
        unsafe {
            if !self.dc.is_invalid() {
                if !self.previous.is_invalid() {
                    SelectObject(self.dc, self.previous);
                }
                let _ = DeleteDC(self.dc);
            }
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(self.bitmap.into());
            }
        }
        self.dc = HDC::default();
        self.bitmap = HBITMAP::default();
        self.previous = HGDIOBJ::default();
        self.brushes.clear();
        self.images.clear();
    }
}

impl Drop for LayeredSurface {
    fn drop(&mut self) {
        self.release();
    }
}

/// Pincéis do render target. São recursos de dispositivo: morrem com ele.
#[derive(Default)]
struct BrushCache {
    solid: HashMap<u128, ID2D1SolidColorBrush>,
    linear: HashMap<(u128, u128), ID2D1LinearGradientBrush>,
}

impl BrushCache {
    fn key(color: Color) -> u128 {
        let pack = |value: f32| value.to_bits() as u128;
        pack(color.r) | pack(color.g) << 32 | pack(color.b) << 64 | pack(color.a) << 96
    }

    fn solid(&mut self, target: &ID2D1RenderTarget, color: Color) -> Option<ID2D1SolidColorBrush> {
        let key = BrushCache::key(color);
        if let Some(brush) = self.solid.get(&key) {
            return Some(brush.clone());
        }
        // SAFETY: a cor vive durante a chamada.
        let brush = unsafe { target.CreateSolidColorBrush(&color_f(color), None) };
        match brush {
            Ok(brush) => Some(self.solid.entry(key).or_insert(brush).clone()),
            Err(err) => {
                log::warn!("CreateSolidColorBrush falhou: {err}");
                None
            }
        }
    }

    fn linear(
        &mut self,
        target: &ID2D1RenderTarget,
        from: Color,
        to: Color,
    ) -> Option<ID2D1LinearGradientBrush> {
        let key = (BrushCache::key(from), BrushCache::key(to));
        if let Some(brush) = self.linear.get(&key) {
            return Some(brush.clone());
        }
        let stops = [
            D2D1_GRADIENT_STOP {
                position: 0.0,
                color: color_f(from),
            },
            D2D1_GRADIENT_STOP {
                position: 1.0,
                color: color_f(to),
            },
        ];
        // SAFETY: as paradas e as propriedades vivem durante as chamadas.
        let collection = unsafe {
            target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP)
        };
        let collection = match collection {
            Ok(collection) => collection,
            Err(err) => {
                log::warn!("CreateGradientStopCollection falhou: {err}");
                return None;
            }
        };
        // SAFETY: coleção recém-criada no mesmo target.
        let brush = unsafe {
            target.CreateLinearGradientBrush(
                &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES::default(),
                None,
                &collection,
            )
        };
        match brush {
            Ok(brush) => Some(self.linear.entry(key).or_insert(brush).clone()),
            Err(err) => {
                log::warn!("CreateLinearGradientBrush falhou: {err}");
                None
            }
        }
    }

    fn clear(&mut self) {
        self.solid.clear();
        self.linear.clear();
    }
}

fn color_f(color: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn rect_f(rect: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.x,
        top: rect.y,
        right: rect.right(),
        bottom: rect.bottom(),
    }
}

fn rounded(rect: Rect, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect: rect_f(rect),
        radiusX: radius,
        radiusY: radius,
    }
}

/// Executa a lista de desenho sobre um render target qualquer.
struct D2dPainter<'a> {
    target: &'a ID2D1RenderTarget,
    brushes: &'a mut BrushCache,
    images: &'a mut BitmapCache,
    text: &'a mut Text,
    clip: Option<Rect>,
}

impl D2dPainter<'_> {
    fn brush(&mut self, color: Color) -> Option<ID2D1SolidColorBrush> {
        self.brushes.solid(self.target, color)
    }
}

impl Drop for D2dPainter<'_> {
    fn drop(&mut self) {
        // Um recorte aberto no fim da lista faria o `EndDraw` falhar.
        self.set_clip(None);
    }
}

impl Measure for D2dPainter<'_> {
    fn text_size(&mut self, text: &str, style: TextStyle, max_width: f32) -> (f32, f32) {
        self.text.text_size(text, style, max_width)
    }
}

impl toolkit::Painter for D2dPainter<'_> {
    fn clear(&mut self, color: Color) {
        // SAFETY: cor viva durante a chamada.
        unsafe { self.target.Clear(Some(&color_f(color))) };
    }

    fn set_clip(&mut self, clip: Option<Rect>) {
        if self.clip == clip {
            return;
        }
        // SAFETY: cada `Push` tem seu `Pop`; o estado fica no campo `clip`.
        unsafe {
            if self.clip.is_some() {
                self.target.PopAxisAlignedClip();
            }
            if let Some(rect) = clip {
                self.target
                    .PushAxisAlignedClip(&rect_f(rect), D2D1_ANTIALIAS_MODE_ALIASED);
            }
        }
        self.clip = clip;
    }

    fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        let Some(brush) = self.brush(color) else {
            return;
        };
        // SAFETY: geometria e pincel vivos durante a chamada.
        unsafe {
            if radius > 0.0 {
                self.target
                    .FillRoundedRectangle(&rounded(rect, radius), &brush);
            } else {
                self.target.FillRectangle(&rect_f(rect), &brush);
            }
        }
    }

    fn gradient(&mut self, rect: Rect, radius: f32, from: Color, to: Color) {
        let Some(brush) = self.brushes.linear(self.target, from, to) else {
            return;
        };
        // SAFETY: o pincel é do mesmo target; pontos e geometria vivem durante
        // as chamadas.
        unsafe {
            brush.SetStartPoint(windows_numerics::Vector2 {
                X: rect.x,
                Y: rect.y,
            });
            brush.SetEndPoint(windows_numerics::Vector2 {
                X: rect.x,
                Y: rect.bottom(),
            });
            if radius > 0.0 {
                self.target
                    .FillRoundedRectangle(&rounded(rect, radius), &brush);
            } else {
                self.target.FillRectangle(&rect_f(rect), &brush);
            }
        }
    }

    fn stroke(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        let Some(brush) = self.brush(color) else {
            return;
        };
        // O traço do Direct2D é centrado na borda; recuar meia espessura o
        // mantém dentro do retângulo, como a borda do CSS.
        let rect = rect.inset(width / 2.0);
        // SAFETY: geometria e pincel vivos durante a chamada.
        unsafe {
            if radius > 0.0 {
                self.target
                    .DrawRoundedRectangle(&rounded(rect, radius), &brush, width, None);
            } else {
                self.target
                    .DrawRectangle(&rect_f(rect), &brush, width, None);
            }
        }
    }

    fn ellipse(&mut self, rect: Rect, color: Color) {
        let Some(brush) = self.brush(color) else {
            return;
        };
        let ellipse = D2D1_ELLIPSE {
            point: windows_numerics::Vector2 {
                X: rect.center_x(),
                Y: rect.center_y(),
            },
            radiusX: rect.w / 2.0,
            radiusY: rect.h / 2.0,
        };
        // SAFETY: geometria e pincel vivos durante a chamada.
        unsafe { self.target.FillEllipse(&ellipse, &brush) };
    }

    fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: Color) {
        let Some(brush) = self.brush(color) else {
            return;
        };
        let point = |(x, y): (f32, f32)| windows_numerics::Vector2 { X: x, Y: y };
        // SAFETY: pincel e estilo vivos durante a chamada.
        unsafe {
            self.target
                .DrawLine(point(from), point(to), &brush, width, round_stroke());
        }
    }

    fn text(&mut self, rect: Rect, text: &str, style: TextStyle, color: Color) {
        let Some(brush) = self.brush(color) else {
            return;
        };
        let brush: &ID2D1Brush = &brush;
        self.text.draw(self.target, rect, text, style, brush);
    }

    fn image(&mut self, rect: Rect, path: &Path, style: ImageStyle) {
        let Some(bitmap) = self.images.get(self.target, path) else {
            return;
        };
        let (opacity, radius, zoom) = (style.opacity, style.radius, style.zoom);

        if style.contain {
            // Proporção preservada: a imagem cabe inteira e é centralizada, então
            // não há o que recortar nem que ampliar.
            // SAFETY: leitura do tamanho de um bitmap vivo.
            let size = unsafe { bitmap.GetSize() };
            if size.width <= 0.0 || size.height <= 0.0 {
                return;
            }
            let scale = (rect.w / size.width).min(rect.h / size.height);
            let dest = rect.centered(size.width * scale, size.height * scale);
            // SAFETY: bitmap do mesmo target; o retângulo vive durante a chamada.
            unsafe {
                self.target.DrawBitmap(
                    &bitmap,
                    Some(&rect_f(dest)),
                    opacity,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    None,
                );
            }
            return;
        }

        if radius <= 0.0 && zoom == 1.0 {
            // SAFETY: bitmap do mesmo target; o retângulo vive durante a chamada.
            unsafe {
                self.target.DrawBitmap(
                    &bitmap,
                    Some(&rect_f(rect)),
                    opacity,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    None,
                );
            }
            return;
        }

        // Com canto arredondado ou zoom o desenho vira o preenchimento de uma
        // geometria com pincel de bitmap: é o que recorta o que passa da borda,
        // sem camada de composição. O pincel é criado a cada uso — é um objeto
        // de CPU, e guardá-lo prenderia o bitmap depois de o cache o descartar.
        // SAFETY: o bitmap pertence a este target e vive durante a chamada.
        let size = unsafe { bitmap.GetSize() };
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let properties = D2D1_BITMAP_BRUSH_PROPERTIES {
            extendModeX: D2D1_EXTEND_MODE_CLAMP,
            extendModeY: D2D1_EXTEND_MODE_CLAMP,
            interpolationMode: D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
        };
        // SAFETY: bitmap e propriedades vivos durante a chamada.
        let brush = match unsafe {
            self.target
                .CreateBitmapBrush(Some(&bitmap), Some(&properties), None)
        } {
            Ok(brush) => brush,
            Err(err) => {
                log::warn!("CreateBitmapBrush falhou para {}: {err}", path.display());
                return;
            }
        };

        // O conteúdo cresce em volta do centro; o retângulo do nó não muda.
        let dest = rect.inset_xy(rect.w * (1.0 - zoom) / 2.0, rect.h * (1.0 - zoom) / 2.0);
        // SAFETY: pincel recém-criado; a matriz vive durante a chamada.
        unsafe {
            brush.SetOpacity(opacity);
            brush.SetTransform(&windows_numerics::Matrix3x2 {
                M11: dest.w / size.width,
                M12: 0.0,
                M21: 0.0,
                M22: dest.h / size.height,
                M31: dest.x,
                M32: dest.y,
            });
            self.target
                .FillRoundedRectangle(&rounded(rect, radius), &brush);
        }
    }
}

/// DPI do monitor da janela, com o padrão de 96 quando o sistema não sabe
/// responder (janela ainda sem monitor).
pub fn window_dpi(hwnd: HWND) -> u32 {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;

    // SAFETY: leitura de propriedade da janela.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        BASE_DPI
    } else {
        dpi
    }
}
