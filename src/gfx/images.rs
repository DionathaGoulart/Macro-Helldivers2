//! Ícones: decodificação de WebP/PNG e o cache de bitmaps do Direct2D.
//!
//! A decodificação é do host (crate `image`), então é testável fora do Windows.
//! O que depende do D2D é só a virada do buffer em `ID2D1Bitmap`.
//!
//! Os arquivos da wiki são 256×256 — 256KB por ícone em BGRA, e são 91
//! estratagemas mais 470 equipamentos. Guardar tudo estouraria sozinho o
//! orçamento de 20MB de RAM do app, então cada imagem entra reduzida ao maior
//! tamanho que a UI usa ([`MAX_EDGE_PX`]) e o cache tem teto: quando passa,
//! descarta as menos usadas. Na prática cabem todas as visíveis de uma vez.

use std::path::Path;

use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::ImageReader;

/// Maior lado guardado em memória. O maior ícone desenhado é o card de
/// estratagema (~140 DIP), o que ainda deixa margem para 150% de DPI.
pub const MAX_EDGE_PX: u32 = 192;

/// Imagem pronta para virar `ID2D1Bitmap`: BGRA de 8 bits com alfa
/// pré-multiplicado, que é o formato dos nossos render targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Decoded {
    /// Bytes por linha, o `pitch` que o `CreateBitmap` pede.
    pub fn stride(&self) -> u32 {
        self.width * 4
    }
}

/// Decodifica um arquivo do disco. O formato vem do conteúdo, não da extensão.
pub fn decode(path: &Path) -> Result<Decoded> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("não foi possível abrir {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("formato ilegível em {}", path.display()))?;
    let image = reader
        .decode()
        .with_context(|| format!("falha ao decodificar {}", path.display()))?;

    let mut rgba = image.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    if width.max(height) > MAX_EDGE_PX {
        let scale = MAX_EDGE_PX as f32 / width.max(height) as f32;
        let target = |value: u32| ((value as f32 * scale).round() as u32).max(1);
        rgba = image::imageops::resize(
            &rgba,
            target(width),
            target(height),
            // Triangular: bom o bastante para reduzir ícone e muito mais barato
            // que Lanczos, que apareceria no boot de 91 imagens.
            FilterType::Triangle,
        );
    }

    let (width, height) = (rgba.width(), rgba.height());
    let mut bgra = rgba.into_raw();
    for pixel in bgra.chunks_exact_mut(4) {
        let alpha = pixel[3] as u32;
        // RGBA direto → BGRA pré-multiplicado, numa passada só.
        let premultiply = |channel: u8| ((channel as u32 * alpha + 127) / 255) as u8;
        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
        pixel[0] = premultiply(b);
        pixel[1] = premultiply(g);
        pixel[2] = premultiply(r);
    }

    Ok(Decoded {
        width,
        height,
        bgra,
    })
}

#[cfg(windows)]
pub use windows_impl::BitmapCache;

#[cfg(windows)]
mod windows_impl {
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};

    use windows::Win32::Graphics::Direct2D::Common::{
        D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT, D2D_SIZE_U,
    };
    use windows::Win32::Graphics::Direct2D::{
        ID2D1Bitmap, ID2D1RenderTarget, D2D1_BITMAP_PROPERTIES,
    };
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    use crate::util;

    /// Teto de memória do cache. Uma tela cheia de cards usa ~3MB.
    const BUDGET_BYTES: usize = 8 * 1024 * 1024;

    struct Entry {
        bitmap: ID2D1Bitmap,
        bytes: usize,
        used: u64,
    }

    /// Bitmaps por caminho relativo a `assets/`.
    ///
    /// Os bitmaps pertencem ao render target que os criou: numa perda de
    /// dispositivo (troca de GPU, atualização de driver) o cache inteiro é
    /// jogado fora junto com o target, em [`BitmapCache::clear`].
    #[derive(Default)]
    pub struct BitmapCache {
        entries: HashMap<PathBuf, Entry>,
        /// Arquivos que já falharam: sem isto, um caminho quebrado tentaria
        /// decodificar de novo a cada repintura.
        broken: HashSet<PathBuf>,
        bytes: usize,
        tick: u64,
    }

    impl BitmapCache {
        /// Bitmap do arquivo, decodificando na primeira vez.
        pub fn get(&mut self, target: &ID2D1RenderTarget, rel: &Path) -> Option<ID2D1Bitmap> {
            self.tick += 1;
            if let Some(entry) = self.entries.get_mut(rel) {
                entry.used = self.tick;
                return Some(entry.bitmap.clone());
            }
            if self.broken.contains(rel) {
                return None;
            }

            let path = util::asset_path(&rel.to_string_lossy());
            let decoded = match super::decode(&path) {
                Ok(decoded) => decoded,
                Err(err) => {
                    log::warn!("ícone ignorado ({err:#})");
                    self.broken.insert(rel.to_path_buf());
                    return None;
                }
            };

            let properties = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                // 96 DPI: o bitmap é medido em pixels e desenhado no retângulo
                // que a lista de nós manda, então a escala vem do destino.
                dpiX: 96.0,
                dpiY: 96.0,
            };
            let size = D2D_SIZE_U {
                width: decoded.width,
                height: decoded.height,
            };

            // SAFETY: o buffer vive durante a chamada e tem
            // `stride * height` bytes, como o `CreateBitmap` exige.
            let bitmap = unsafe {
                target.CreateBitmap(
                    size,
                    Some(decoded.bgra.as_ptr() as *const _),
                    decoded.stride(),
                    &properties,
                )
            };
            let bitmap = match bitmap {
                Ok(bitmap) => bitmap,
                Err(err) => {
                    log::warn!("CreateBitmap falhou para {}: {err}", rel.display());
                    self.broken.insert(rel.to_path_buf());
                    return None;
                }
            };

            self.bytes += decoded.bgra.len();
            self.entries.insert(
                rel.to_path_buf(),
                Entry {
                    bitmap: bitmap.clone(),
                    bytes: decoded.bgra.len(),
                    used: self.tick,
                },
            );
            self.evict();
            Some(bitmap)
        }

        /// Descarta as entradas mais antigas até caber no orçamento.
        fn evict(&mut self) {
            while self.bytes > BUDGET_BYTES && self.entries.len() > 1 {
                let Some(oldest) = self
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.used)
                    .map(|(path, _)| path.clone())
                else {
                    return;
                };
                if let Some(entry) = self.entries.remove(&oldest) {
                    self.bytes -= entry.bytes;
                }
            }
        }

        /// Esvazia o cache — chamado quando o render target é recriado.
        pub fn clear(&mut self) {
            self.entries.clear();
            self.broken.clear();
            self.bytes = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn a_webp_icon_decodes_to_premultiplied_bgra() {
        let path = util::asset_path("icons/stratagems/A_FLAM-40_Flame_Sentry_Stratagem_Icon.webp");
        let decoded = decode(&path).expect("ícone do repositório");

        assert!(decoded.width > 0 && decoded.height > 0);
        assert_eq!(decoded.bgra.len() as u32, decoded.stride() * decoded.height);
        // Pré-multiplicado: nenhum canal pode passar do alfa.
        assert!(decoded
            .bgra
            .chunks_exact(4)
            .all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]));
    }

    #[test]
    fn oversized_icons_are_reduced_to_the_cache_budget() {
        let path = util::asset_path("icons/stratagems/A_FLAM-40_Flame_Sentry_Stratagem_Icon.webp");
        let decoded = decode(&path).unwrap();
        assert!(decoded.width.max(decoded.height) <= MAX_EDGE_PX);
    }

    #[test]
    fn png_assets_decode_too() {
        let decoded = decode(&util::asset_path("icons/tray.png")).expect("tray.png");
        assert!(decoded.width > 0);
    }

    #[test]
    fn a_missing_file_is_an_error_not_a_panic() {
        assert!(decode(&util::asset_path("icons/nao-existe.webp")).is_err());
    }
}
