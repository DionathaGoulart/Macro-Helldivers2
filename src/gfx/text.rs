//! DirectWrite: a Inter embutida, formatos cacheados e o desenho de texto.
//!
//! As duas faces (Regular 400 e Black 900) vêm de `assets/fonts/` para uma
//! coleção privada do processo — nada é instalado no sistema, e o app não
//! depende de o usuário ter a fonte. Se a carga falhar, o texto cai na fonte de
//! interface do Windows em vez de sumir.
//!
//! O `tracking` do tema (`letter-spacing` do CSS) só existe em
//! `IDWriteTextLayout`, então todo texto vira layout — que também é o que mede.
//! Layouts prontos ficam num cache: uma repintura reaproveita os da anterior, e
//! só o que mudou é remontado.

use std::collections::HashMap;
use std::os::windows::ffi::OsStrExt;

use windows::core::{w, Interface, Result, PCWSTR};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Brush, ID2D1RenderTarget, D2D1_DRAW_TEXT_OPTIONS_NONE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory5, IDWriteFontCollection1, IDWriteTextFormat,
    IDWriteTextLayout, IDWriteTextLayout1, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BLACK, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING_NO_WRAP, DWRITE_WORD_WRAPPING_WRAP,
};

use crate::ui::toolkit::{Align, Measure, Rect, TextStyle, Weight};
use crate::util;

/// Arquivos da família, relativos a `assets/`.
const FONT_FILES: [&str; 2] = ["fonts/Inter-Regular.ttf", "fonts/Inter-Black.ttf"];

/// Teto do cache de layouts. Uma tela cheia usa algumas centenas; passando
/// disso (busca digitada rápido, por exemplo) o cache recomeça do zero.
const LAYOUT_CACHE_MAX: usize = 512;

/// Largura usada quando a medição pede "sem limite" — o DirectWrite não aceita
/// infinito.
const UNBOUNDED: f32 = 100_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StyleKey {
    size: u32,
    tracking: u32,
    weight: u8,
    align: u8,
    flags: u8,
}

impl StyleKey {
    fn new(style: TextStyle) -> StyleKey {
        StyleKey {
            size: style.size.to_bits(),
            tracking: style.tracking.to_bits(),
            weight: match style.weight {
                Weight::Regular => 0,
                Weight::Black => 1,
            },
            align: match style.align {
                Align::Start => 0,
                Align::Center => 1,
                Align::End => 2,
            },
            flags: u8::from(style.middle) | (u8::from(style.wrap) << 1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LayoutKey {
    text: String,
    style: StyleKey,
    width: u32,
    height: u32,
}

/// Motor de texto de uma janela. Vive na thread dona da janela.
pub struct Text {
    factory: IDWriteFactory5,
    collection: Option<IDWriteFontCollection1>,
    formats: HashMap<StyleKey, IDWriteTextFormat>,
    layouts: HashMap<LayoutKey, IDWriteTextLayout>,
}

impl Text {
    pub fn new() -> Result<Text> {
        // SAFETY: fábrica compartilhada do DirectWrite; o tipo pedido casa com
        // o `riid` que a macro do binding deriva.
        let factory: IDWriteFactory5 = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };

        let collection = match load_collection(&factory) {
            Ok(collection) => Some(collection),
            Err(err) => {
                log::warn!("Inter não carregou ({err}); usando a fonte do sistema");
                None
            }
        };

        Ok(Text {
            factory,
            collection,
            formats: HashMap::new(),
            layouts: HashMap::new(),
        })
    }

    fn family(&self) -> PCWSTR {
        if self.collection.is_some() {
            w!("Inter")
        } else {
            w!("Segoe UI")
        }
    }

    fn format(&mut self, style: TextStyle) -> Result<IDWriteTextFormat> {
        let key = StyleKey::new(style);
        if let Some(format) = self.formats.get(&key) {
            return Ok(format.clone());
        }

        let weight = match style.weight {
            Weight::Regular => DWRITE_FONT_WEIGHT_NORMAL,
            Weight::Black => DWRITE_FONT_WEIGHT_BLACK,
        };
        // SAFETY: todos os ponteiros são strings estáticas ou COM válidos.
        let format = unsafe {
            self.factory.CreateTextFormat(
                self.family(),
                self.collection.as_deref(),
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                style.size,
                w!("en-us"),
            )?
        };

        // SAFETY: chamadas de configuração no formato recém-criado.
        unsafe {
            format.SetTextAlignment(match style.align {
                Align::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
                Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
                Align::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
            })?;
            format.SetParagraphAlignment(if style.middle {
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER
            } else {
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR
            })?;
            format.SetWordWrapping(if style.wrap {
                DWRITE_WORD_WRAPPING_WRAP
            } else {
                DWRITE_WORD_WRAPPING_NO_WRAP
            })?;

            // Linha única que não cabe termina em reticências, como o
            // `text-ellipsis` do legado, em vez de vazar para o vizinho.
            if !style.wrap {
                let sign = self.factory.CreateEllipsisTrimmingSign(&format)?;
                let trimming = DWRITE_TRIMMING {
                    granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter: 0,
                    delimiterCount: 0,
                };
                format.SetTrimming(&trimming, &sign)?;
            }
        }

        self.formats.insert(key, format.clone());
        Ok(format)
    }

    fn layout(
        &mut self,
        text: &str,
        style: TextStyle,
        width: f32,
        height: f32,
    ) -> Result<IDWriteTextLayout> {
        let width = if width.is_finite() { width } else { UNBOUNDED };
        let height = if height.is_finite() {
            height
        } else {
            UNBOUNDED
        };
        let key = LayoutKey {
            text: text.to_string(),
            style: StyleKey::new(style),
            width: width.to_bits(),
            height: height.to_bits(),
        };
        if let Some(layout) = self.layouts.get(&key) {
            return Ok(layout.clone());
        }

        let format = self.format(style)?;
        let wide: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: `wide` vive durante a chamada; o DirectWrite copia o texto.
        let layout = unsafe {
            self.factory
                .CreateTextLayout(&wide, &format, width, height)?
        };

        if style.tracking != 0.0 {
            // `letter-spacing` só existe no layout. O espaçamento vai depois de
            // cada caractere, exatamente como no CSS.
            let spacing = style.tracking * style.size;
            let layout1: IDWriteTextLayout1 = layout.cast()?;
            let range = DWRITE_TEXT_RANGE {
                startPosition: 0,
                length: wide.len() as u32,
            };
            // SAFETY: intervalo dentro do texto que acabou de ser medido.
            unsafe { layout1.SetCharacterSpacing(0.0, spacing, 0.0, range)? };
        }

        if self.layouts.len() >= LAYOUT_CACHE_MAX {
            self.layouts.clear();
        }
        self.layouts.insert(key, layout.clone());
        Ok(layout)
    }

    /// Desenha dentro de `rect`; alinhamento e quebra vêm do estilo.
    pub fn draw(
        &mut self,
        target: &ID2D1RenderTarget,
        rect: Rect,
        text: &str,
        style: TextStyle,
        brush: &ID2D1Brush,
    ) {
        let Ok(layout) = self.layout(text, style, rect.w, rect.h) else {
            return;
        };
        // SAFETY: layout e brush são objetos vivos; o target está entre
        // `BeginDraw` e `EndDraw` (garantido por quem chama o painter).
        unsafe {
            target.DrawTextLayout(
                windows_numerics::Vector2 {
                    X: rect.x,
                    Y: rect.y,
                },
                &layout,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
    }

    /// Esvazia os caches. Formatos e layouts não dependem do render target, mas
    /// a troca de idioma renova todas as strings de uma vez.
    pub fn clear_cache(&mut self) {
        self.layouts.clear();
    }
}

impl Measure for Text {
    fn text_size(&mut self, text: &str, style: TextStyle, max_width: f32) -> (f32, f32) {
        let Ok(layout) = self.layout(text, style, max_width, UNBOUNDED) else {
            // Estimativa grosseira só para o layout não colapsar caso o
            // DirectWrite recuse a chamada.
            return (text.chars().count() as f32 * style.size * 0.6, style.size);
        };
        let mut metrics = DWRITE_TEXT_METRICS::default();
        // SAFETY: leitura de métricas de um layout válido para uma struct nossa.
        if unsafe { layout.GetMetrics(&mut metrics) }.is_err() {
            return (0.0, style.size);
        }
        (
            metrics.widthIncludingTrailingWhitespace,
            metrics.height.max(style.size),
        )
    }
}

/// Monta a coleção privada a partir dos arquivos de `assets/fonts/`.
fn load_collection(factory: &IDWriteFactory5) -> Result<IDWriteFontCollection1> {
    // SAFETY: as chamadas recebem apenas caminhos válidos e objetos COM vivos.
    unsafe {
        let builder = factory.CreateFontSetBuilder()?;
        for file in FONT_FILES {
            let path = util::asset_path(file);
            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let font_file = factory.CreateFontFileReference(PCWSTR(wide.as_ptr()), None)?;
            builder.AddFontFile(&font_file)?;
        }
        let set = builder.CreateFontSet()?;
        factory.CreateFontCollectionFromFontSet(&set)
    }
}

/// Registra as mesmas fontes no GDI, só para o processo. É o único jeito de o
/// controle `EDIT` nativo (que não fala DirectWrite) usar a Inter.
pub fn register_gdi_fonts() {
    use windows::Win32::Graphics::Gdi::{AddFontResourceExW, FR_PRIVATE};

    for file in FONT_FILES {
        let path = util::asset_path(file);
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: caminho terminado em nulo; `FR_PRIVATE` limita a fonte a este
        // processo, então não há registro global para desfazer.
        let added = unsafe { AddFontResourceExW(PCWSTR(wide.as_ptr()), FR_PRIVATE, None) };
        if added == 0 {
            log::warn!("AddFontResourceExW recusou {}", path.display());
        }
    }
}
