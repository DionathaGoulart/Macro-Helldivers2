//! Tokens visuais do tema HD (R5) e a conversão DIP↔pixel.
//!
//! Os valores vêm do CSS da v1 (`legacy/src/renderer/index.css` + as classes
//! Tailwind usadas no JSX). O que lá era `bg-slate-900/40` sobre um fundo opaco
//! aqui já entra pré-composto: o Direct2D pinta uma camada a menos, e a cor final
//! é a mesma.
//!
//! Tudo é medido em DIP. O render target recebe o DPI do monitor
//! (`SetDpi`) e faz a multiplicação sozinho, então nenhum widget precisa saber a
//! escala — ela só aparece onde o Win32 fala em pixel: tamanho de janela, posição
//! do mouse e bounds persistidos.

/// Cor RGBA com alfa direto (não pré-multiplicado), do jeito que o
/// `D2D1_COLOR_F` espera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Cor opaca a partir de `0xRRGGBB`.
    pub const fn rgb(hex: u32) -> Color {
        Color::rgba(hex, 1.0)
    }

    /// Cor a partir de `0xRRGGBB` com alfa em 0..=1.
    pub const fn rgba(hex: u32, a: f32) -> Color {
        Color {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: (hex & 0xFF) as f32 / 255.0,
            a,
        }
    }

    pub const fn alpha(self, a: f32) -> Color {
        Color { a, ..self }
    }

    /// Interpola até `other`. Serve pros fades de hover, que andam por um valor
    /// 0..1 vindo do toolkit.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Compõe `self` sobre `backdrop`, que é como os tokens translúcidos do
    /// legado viram cor sólida quando o fundo é opaco.
    pub fn over(self, backdrop: Color) -> Color {
        let a = self.a + backdrop.a * (1.0 - self.a);
        if a <= f32::EPSILON {
            return Color::rgba(0, 0.0);
        }
        let blend =
            |top: f32, bottom: f32| (top * self.a + bottom * backdrop.a * (1.0 - self.a)) / a;
        Color {
            r: blend(self.r, backdrop.r),
            g: blend(self.g, backdrop.g),
            b: blend(self.b, backdrop.b),
            a,
        }
    }
}

// --- Paleta (R5) ---

/// Fundo da janela (`--color-hd-bg-deep`).
pub const BG_DEEP: Color = Color::rgb(0x020617);
/// `bg-slate-900/40` sobre [`BG_DEEP`], já composto.
pub const CARD_BG: Color = Color::rgb(0x0B1120);
/// slate-800: borda padrão de cards, inputs e divisores.
pub const BORDER: Color = Color::rgb(0x1E293B);
/// slate-200: texto principal.
pub const TEXT: Color = Color::rgb(0xE2E8F0);
/// slate-500: labels e texto de apoio.
pub const TEXT_DIM: Color = Color::rgb(0x64748B);
/// Acento principal (ativo, slots, aba de configurações).
pub const YELLOW: Color = Color::rgb(0xFBBF24);
/// Acento secundário (`--color-hd-primary`) e categoria Supply.
pub const CYAN: Color = Color::rgb(0x22D3EE);
/// Categoria Offensive, erros e bloqueio de macro.
pub const RED: Color = Color::rgb(0xEF4444);
/// Categoria Defensive e sucesso.
pub const GREEN: Color = Color::rgb(0x22C55E);

// --- Derivados de uso frequente ---

/// slate-300: texto de item inativo sob o mouse.
pub const TEXT_HOVER: Color = Color::rgb(0xCBD5E1);
/// slate-950: texto sobre botão de acento (`hd-btn-primary`).
pub const TEXT_ON_ACCENT: Color = Color::rgb(0x020617);
/// `border-white/5`: divisores internos do header e dos cards.
pub const HAIRLINE: Color = Color::rgba(0xFFFFFF, 0.05);
/// `bg-slate-950/60`: fundo de input, slot e botão secundário.
pub const SURFACE: Color = Color::rgba(0x020617, 0.60);
/// `hover:bg-slate-900/50` das abas inativas.
pub const SURFACE_HOVER: Color = Color::rgba(0x0F172A, 0.50);
/// `bg-slate-800/80`: fundo do slot em edição.
pub const SURFACE_ACTIVE: Color = Color::rgba(0x1E293B, 0.80);
/// Trilho e polegar da barra de rolagem (`scrollbar-hd`, 5px).
pub const SCROLL_THUMB: Color = Color::rgba(0x1E293B, 0.80);

/// Alfa do brilho que substitui o `box-shadow` do CSS: um traço externo na cor
/// do acento. Sem blur gaussiano — o custo não pagaria a diferença.
pub const GLOW_ALPHA: f32 = 0.35;
/// Espessura do traço de brilho, em DIP.
pub const GLOW_WIDTH: f32 = 2.5;

// --- Raios e métricas ---

pub const RADIUS_CARD: f32 = 16.0;
pub const RADIUS_BUTTON: f32 = 12.0;
pub const RADIUS_SLOT: f32 = 12.0;
/// Espessura das bordas e divisores de 1px do legado.
pub const HAIRLINE_WIDTH: f32 = 1.0;
/// Largura da barra de rolagem (`scrollbar-hd`).
pub const SCROLLBAR_WIDTH: f32 = 5.0;

/// Fonte e tamanhos. A família é carregada de `assets/fonts/` numa coleção
/// própria do DirectWrite — nada é instalado no sistema.
pub mod font {
    pub const FAMILY: &str = "Inter";

    pub const SIZE_TINY: f32 = 9.0;
    pub const SIZE_LABEL: f32 = 10.0;
    pub const SIZE_BODY: f32 = 11.0;
    pub const SIZE_CARD_HEADER: f32 = 13.0;
    pub const SIZE_TITLE: f32 = 16.0;

    /// `tracking-[0.25em]` das abas e headers, em EM.
    pub const TRACKING_WIDE: f32 = 0.25;
    /// `tracking-widest` dos labels, em EM.
    pub const TRACKING_LABEL: f32 = 0.2;
}

/// DPI em que 1 DIP = 1 pixel.
pub const BASE_DPI: u32 = 96;

/// Escala do monitor. Windows entrega DPI inteiro por monitor (per-monitor v2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    factor: f32,
}

impl Scale {
    /// 100% — o que vale no host de desenvolvimento e nos testes.
    pub const ONE: Scale = Scale { factor: 1.0 };

    pub fn from_dpi(dpi: u32) -> Scale {
        // DPI zerado aparece quando a janela ainda não tem monitor; 96 é o que o
        // Windows devolveria de qualquer forma.
        let dpi = if dpi == 0 { BASE_DPI } else { dpi };
        Scale {
            factor: dpi as f32 / BASE_DPI as f32,
        }
    }

    pub fn factor(self) -> f32 {
        self.factor
    }

    /// DIP → pixel, arredondado: coordenada de janela é inteira.
    pub fn px(self, dip: f32) -> i32 {
        (dip * self.factor).round() as i32
    }

    /// Pixel → DIP. É por aqui que a posição do mouse entra no toolkit.
    pub fn dip(self, px: f32) -> f32 {
        px / self.factor
    }
}

impl Default for Scale {
    fn default() -> Scale {
        Scale::ONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.002
    }

    #[test]
    fn hex_becomes_normalized_channels() {
        let cyan = Color::rgb(0x22D3EE);
        assert!(close(cyan.r, 0x22 as f32 / 255.0));
        assert!(close(cyan.g, 0xD3 as f32 / 255.0));
        assert!(close(cyan.b, 0xEE as f32 / 255.0));
        assert!(close(cyan.a, 1.0));

        assert!(close(Color::rgba(0xFFFFFF, 0.05).a, 0.05));
        assert!(close(CYAN.alpha(0.1).a, 0.1));
        assert_eq!(CYAN.alpha(0.1).r, CYAN.r);
    }

    #[test]
    fn mixing_walks_from_one_color_to_the_other() {
        assert_eq!(TEXT_DIM.mix(TEXT_HOVER, 0.0), TEXT_DIM);
        assert_eq!(TEXT_DIM.mix(TEXT_HOVER, 1.0), TEXT_HOVER);

        let half = TEXT_DIM.mix(TEXT_HOVER, 0.5);
        assert!(close(half.r, (TEXT_DIM.r + TEXT_HOVER.r) / 2.0));
        // Fora da faixa o valor é preso nas pontas, não extrapolado.
        assert_eq!(TEXT_DIM.mix(TEXT_HOVER, 5.0), TEXT_HOVER);
    }

    /// `CARD_BG` é o `bg-slate-900/40` do legado sobre o fundo profundo. O token
    /// da tabela R5 (`#0B1120`) arredonda essa composição para cima; o teste
    /// garante que ele continua sendo a mesma cor, e não outra qualquer.
    #[test]
    fn card_background_matches_the_legacy_composition() {
        let composed = Color::rgba(0x0F172A, 0.40).over(BG_DEEP);
        for (token, exact) in [
            (CARD_BG.r, composed.r),
            (CARD_BG.g, composed.g),
            (CARD_BG.b, composed.b),
        ] {
            assert!(
                (token - exact).abs() < 0.02,
                "token {token} longe da composição {exact}"
            );
        }
        assert!(close(composed.a, 1.0));
        assert!(close(CARD_BG.a, 1.0));
    }

    #[test]
    fn compositing_over_an_opaque_backdrop_stays_opaque() {
        let solid = HAIRLINE.over(CARD_BG);
        assert!(close(solid.a, 1.0));
        // Branco a 5% clareia um pouco, sem chegar perto do branco.
        assert!(solid.r > CARD_BG.r && solid.r < 0.2);
    }

    #[test]
    fn transparent_over_transparent_is_transparent() {
        let empty = Color::rgba(0xFFFFFF, 0.0).over(Color::rgba(0x000000, 0.0));
        assert_eq!(empty.a, 0.0);
    }

    #[test]
    fn scale_converts_both_ways() {
        let at_150 = Scale::from_dpi(144);
        assert!(close(at_150.factor(), 1.5));
        assert_eq!(at_150.px(820.0), 1230);
        assert!(close(at_150.dip(1230.0), 820.0));

        let at_100 = Scale::from_dpi(96);
        assert_eq!(at_100, Scale::ONE);
        assert_eq!(at_100.px(640.0), 640);
    }

    #[test]
    fn a_missing_dpi_falls_back_to_one_hundred_percent() {
        assert_eq!(Scale::from_dpi(0), Scale::ONE);
        assert_eq!(Scale::default(), Scale::ONE);
    }
}
