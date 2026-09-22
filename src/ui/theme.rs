//! Tokens visuais da skin `retro` (neobrutal) e a conversão DIP↔pixel.
//!
//! A fonte de verdade é o `styleguide.md` da raiz: os hex das duas paletas, a
//! fonte e a geometria vêm de lá sem alteração. Aqui eles viram três camadas,
//! na mesma ordem do guia:
//!
//! 1. [`raw`]: a paleta bruta. **Hex existe só ali**, uma vez (§2.1).
//! 2. [`Palette`]: os tokens semânticos de cada tema (§2.2-§2.4). Widget
//!    nenhum lê a paleta bruta nem pergunta qual é o tema: ele pede
//!    [`palette()`] e usa o papel (`base_300` é toda moldura, `accent` é todo
//!    fill de ênfase…). Trocar o tema é trocar a paleta inteira de uma vez.
//! 3. A geometria (§4, §5): moldura de 2px, sombra dura deslocada, zero raio.
//!    Não existe constante de raio: o toolkit nem aceita um.
//!
//! Tudo é medido em DIP. O render target recebe o DPI do monitor
//! (`SetDpi`) e faz a multiplicação sozinho, então nenhum widget precisa saber a
//! escala. Ela só aparece onde o Win32 fala em pixel: tamanho de janela, posição
//! do mouse e bounds persistidos.

use std::sync::atomic::{AtomicU8, Ordering};

pub use crate::settings::Theme;

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

    /// Multiplica o alfa: é assim que um bloco inteiro apaga junto (estado
    /// `:disabled`, saída de um toast) sem virar outra cor.
    pub const fn faded(self, factor: f32) -> Color {
        Color {
            a: self.a * factor,
            ..self
        }
    }

    /// Interpola até `other`. Serve pros fades de hover, que andam por um valor
    /// 0..1 vindo do toolkit.
    pub fn mix(self, other: Color, t: f32) -> Color {
        // Nas pontas a cor é a do token, bit a bit, sem o resíduo da conta.
        if t <= 0.0 {
            return self;
        }
        if t >= 1.0 {
            return other;
        }
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Compõe `self` sobre `backdrop`: um token translúcido sobre um fundo
    /// opaco vira a cor sólida que aparece na tela.
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

    /// Luminância relativa do WCAG, para as contas de contraste dos testes.
    pub fn luminance(self) -> f32 {
        let channel = |c: f32| {
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// Razão de contraste do WCAG entre duas cores opacas.
    pub fn contrast(self, other: Color) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

/// Paleta bruta (§2.1). É o único lugar do app com hex de cor.
///
/// A identidade vem do logo: amarelo `#fbee23` e preto `#000000`. O claro é
/// amarelo com preto de apoio; o escuro, o contrário.
pub mod raw {
    use super::Color;

    pub const YELLOW: Color = Color::rgb(0xFBEE23);
    pub const YELLOW_RAISED: Color = Color::rgb(0xFDF699);
    pub const BLACK: Color = Color::rgb(0x000000);
    pub const BLACK_RAISED: Color = Color::rgb(0x141414);
    pub const WHITE: Color = Color::rgb(0xFFFFFF);
    pub const INFO: Color = Color::rgb(0x2563EB);
    pub const SUCCESS: Color = Color::rgb(0x16A34A);
    pub const WARNING: Color = Color::rgb(0xD97706);
    pub const ERROR: Color = Color::rgb(0xDC2626);
    pub const INFO_DEEP: Color = Color::rgb(0x1D4ED8);
    pub const SUCCESS_DEEP: Color = Color::rgb(0x166534);
    pub const WARNING_DEEP: Color = Color::rgb(0x92400E);
    pub const ERROR_DEEP: Color = Color::rgb(0xB91C1C);
    pub const INFO_SOFT: Color = Color::rgb(0x60A5FA);
    pub const SUCCESS_SOFT: Color = Color::rgb(0x4ADE80);
    pub const WARNING_SOFT: Color = Color::rgb(0xFBBF24);
    pub const ERROR_SOFT: Color = Color::rgb(0xF87171);
    pub const SCANLINE_LIGHT: Color = Color::rgba(0x000000, 0.05);
    pub const SCANLINE_DARK: Color = Color::rgba(0x000000, 0.2);
}

/// Um status (§2.2): a cor de fill, o conteúdo que vai por cima dela e a cor
/// para quando o status é o próprio glifo (§2.4: fill não serve de texto).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Status {
    pub fill: Color,
    pub content: Color,
    pub text: Color,
}

/// Os tokens semânticos de um tema (§2.2 e §2.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Fundo de página.
    pub base_100: Color,
    /// Superfície elevada: painel, campo, dropdown, diálogo.
    pub base_200: Color,
    /// Toda moldura. Neutra, nunca accent; no escuro é a cor clara do texto.
    pub base_300: Color,
    /// Texto.
    pub content: Color,
    /// Micro-texto e rótulo apagados: 60% do `content`. Apagar é cor, nunca
    /// opacidade de grupo (§2.4).
    pub muted: Color,
    /// Ênfase como **fill**: estado ativo, item selecionado, CTA.
    pub accent: Color,
    /// O que vai por cima do fill accent.
    pub accent_content: Color,
    /// Accent como **glifo** (kicker, número em destaque).
    pub accent_text: Color,
    pub info: Status,
    pub success: Status,
    pub warning: Status,
    pub error: Status,
    /// Cor da sombra dura. No escuro a sombra é accent.
    pub shadow: Color,
    /// Linha da textura de scanline, já com a opacidade de 0.3 do motif §4.4.
    pub scanline: Color,
}

impl Palette {
    /// Paleta de um tema.
    pub fn of(theme: Theme) -> &'static Palette {
        match theme {
            Theme::Rose => &ROSE,
            Theme::Crimson => &CRIMSON,
        }
    }

    /// Fill de hover das linhas e itens de navegação: 8% do texto sobre o que
    /// estiver atrás (`color-mix(base-content 8%)`).
    pub fn hover_fill(&self) -> Color {
        self.content.faded(0.08)
    }

    /// Divisor de linha de lista (`1px base-300` com `opacity-30`).
    pub fn rule(&self) -> Color {
        self.base_300.faded(0.3)
    }
}

/// Opacidade da scanline sobre a cor base do tema (§4.4).
const SCANLINE_OPACITY: f32 = 0.3;

/// `crimson`: o claro (`color-scheme: light`). Amarelo na frente, preto de
/// apoio: fundo amarelo, moldura, texto e fill de ênfase pretos.
pub static CRIMSON: Palette = Palette {
    base_100: raw::YELLOW,
    base_200: raw::YELLOW_RAISED,
    base_300: raw::BLACK,
    content: raw::BLACK,
    muted: raw::BLACK.faded(0.6),
    accent: raw::BLACK,
    accent_content: raw::YELLOW,
    accent_text: raw::BLACK,
    info: Status {
        fill: raw::INFO,
        content: raw::WHITE,
        text: raw::INFO_DEEP,
    },
    success: Status {
        fill: raw::SUCCESS,
        content: raw::BLACK,
        text: raw::SUCCESS_DEEP,
    },
    warning: Status {
        fill: raw::WARNING,
        content: raw::BLACK,
        text: raw::WARNING_DEEP,
    },
    error: Status {
        fill: raw::ERROR,
        content: raw::WHITE,
        text: raw::ERROR_DEEP,
    },
    shadow: raw::BLACK,
    scanline: raw::SCANLINE_LIGHT.faded(SCANLINE_OPACITY),
};

/// `rose`: o escuro (`color-scheme: dark`), padrão do app. Preto na frente,
/// amarelo de apoio: moldura, texto, ênfase e sombra amarelos.
pub static ROSE: Palette = Palette {
    base_100: raw::BLACK,
    base_200: raw::BLACK_RAISED,
    base_300: raw::YELLOW,
    content: raw::YELLOW,
    muted: raw::YELLOW.faded(0.6),
    accent: raw::YELLOW,
    accent_content: raw::BLACK,
    accent_text: raw::YELLOW,
    info: Status {
        fill: raw::INFO_SOFT,
        content: raw::BLACK,
        text: raw::INFO_SOFT,
    },
    success: Status {
        fill: raw::SUCCESS_SOFT,
        content: raw::BLACK,
        text: raw::SUCCESS_SOFT,
    },
    warning: Status {
        fill: raw::WARNING_SOFT,
        content: raw::BLACK,
        text: raw::WARNING_SOFT,
    },
    error: Status {
        fill: raw::ERROR_SOFT,
        content: raw::BLACK,
        text: raw::ERROR_SOFT,
    },
    shadow: raw::YELLOW,
    scanline: raw::SCANLINE_DARK.faded(SCANLINE_OPACITY),
};

/// Tema em uso pelo processo. Um só para a janela e o overlay: a troca vale
/// para as duas threads na próxima passagem de construção de cada uma.
static CURRENT: AtomicU8 = AtomicU8::new(0);

fn encode(theme: Theme) -> u8 {
    match theme {
        Theme::Rose => 0,
        Theme::Crimson => 1,
    }
}

/// Tema em uso. Padrão `rose` (§0.2).
pub fn current() -> Theme {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Theme::Crimson,
        _ => Theme::Rose,
    }
}

/// Troca o tema do processo. Quem chama refaz as telas.
pub fn set(theme: Theme) {
    CURRENT.store(encode(theme), Ordering::Relaxed);
}

/// Os tokens do tema em uso. É por aqui que todo widget pega cor.
pub fn palette() -> &'static Palette {
    Palette::of(current())
}

// --- Geometria (§4, §5) ---

/// Moldura grossa reta (`--frame-border`, §4.2), em toda caixa e controle.
pub const BORDER: f32 = 2.0;
/// Sombra dura deslocada (`--frame-shadow`, §4.1): sem blur, sem spread.
pub const SHADOW: f32 = 6.0;
/// `--frame-shadow-sm`: o repouso dos clicáveis e o nível dos popovers.
pub const SHADOW_SM: f32 = 3.0;
/// Quanto um clicável sobe no hover (`hover:-translate-y-1`, §4.9).
pub const LIFT: f32 = 4.0;
/// Divisor fino de linha de lista.
pub const HAIRLINE: f32 = 1.0;
/// Anel de foco (§4.12): 2px sólidos, afastados 2px do controle.
pub const FOCUS_RING: f32 = 2.0;
pub const FOCUS_OFFSET: f32 = 2.0;
/// Barra de rolagem: polegar reto, na cor da moldura.
pub const SCROLLBAR_WIDTH: f32 = 6.0;
/// Uma linha de scanline a cada tantos DIP (§4.4).
pub const SCANLINE_STEP: f32 = 4.0;
/// `:disabled` (§2.4): a única opacidade legítima num controle.
pub const DISABLED_ALPHA: f32 = 0.4;

/// Tempos do motion (§4 "Motion").
pub mod motion {
    /// `transition-all duration-300` do hover que levanta.
    pub const HOVER_MS: u32 = 300;
    /// `animate-enter`: fade + 8px de subida em 200ms, sem overshoot.
    pub const ENTER_MS: u32 = 200;
    pub const ENTER_RISE: f32 = 8.0;
    /// Período do caret piscando (`blink 1s step-end infinite`, §4.5).
    pub const BLINK_MS: u32 = 1_000;
    /// Saída de toast e de overlay (fade de 150ms).
    pub const EXIT_MS: u32 = 150;

    /// `cubic-bezier(0.33, 1, 0.68, 1)`: o ease-out do `animate-enter`, sem
    /// overshoot. Resolvido por bisseção em `x`, que é monotônico na curva.
    pub fn ease_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let bezier = |p1: f32, p2: f32, s: f32| {
            let u = 1.0 - s;
            3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s
        };
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if bezier(0.33, 0.68, mid) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        bezier(1.0, 1.0, (lo + hi) / 2.0)
    }
}

/// Tipografia (§3). Uma família para tudo, carregada de `assets/fonts/` numa
/// coleção própria do DirectWrite; nada é instalado no sistema.
pub mod font {
    pub const FAMILY: &str = "JetBrains Mono";

    /// Etiqueta de atalho dentro do slot.
    pub const SIZE_TINY: f32 = 9.0;
    /// Micro-texto: hora, id, status, kicker, cabeçalho de lista (`text-[10px]`).
    pub const SIZE_MICRO: f32 = 10.0;
    /// Rótulo de formulário e texto de botão (`text-xs`).
    pub const SIZE_LABEL: f32 = 11.0;
    /// Corpo (`text-sm` na escala da janela de 820 DIP).
    pub const SIZE_BODY: f32 = 12.0;
    /// Nome da marca na topbar (`screen-title` pequeno da sidebar).
    pub const SIZE_BRAND: f32 = 18.0;
    /// Título de tela e de diálogo (`screen-title`).
    pub const SIZE_TITLE: f32 = 22.0;

    /// `tracking-[0.2em]` do micro-texto.
    pub const TRACKING_MICRO: f32 = 0.2;
    /// `tracking-widest` dos rótulos.
    pub const TRACKING_WIDEST: f32 = 0.1;
    /// `letter-spacing: 0.05em` dos botões grandes.
    pub const TRACKING_BUTTON: f32 = 0.05;
    /// `tracking-wide` dos itens de navegação.
    pub const TRACKING_WIDE: f32 = 0.025;
    /// `tracking-tighter` dos títulos grandes.
    pub const TRACKING_TIGHTER: f32 = -0.05;

    /// Altura de uma linha em EM: ascendente 1020 + descendente 300 sobre
    /// 1000 unidades da JetBrains Mono. É o que o DirectWrite mede.
    pub const LINE: f32 = 1.32;
    /// Avanço de um caractere em EM: a fonte é monoespaçada.
    pub const ADVANCE: f32 = 0.6;
}

/// DPI em que 1 DIP = 1 pixel.
pub const BASE_DPI: u32 = 96;

/// Escala do monitor. Windows entrega DPI inteiro por monitor (per-monitor v2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    factor: f32,
}

impl Scale {
    /// 100%: o que vale no host de desenvolvimento e nos testes.
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
        let rose = raw::YELLOW;
        assert!(close(rose.r, 0xFB as f32 / 255.0));
        assert!(close(rose.g, 0xEE as f32 / 255.0));
        assert!(close(rose.b, 0x23 as f32 / 255.0));
        assert!(close(rose.a, 1.0));

        assert!(close(Color::rgba(0xFFFFFF, 0.05).a, 0.05));
        assert!(close(rose.alpha(0.1).a, 0.1));
        assert_eq!(rose.alpha(0.1).r, rose.r);
        assert!(close(rose.alpha(0.5).faded(0.5).a, 0.25));
    }

    #[test]
    fn mixing_walks_from_one_color_to_the_other() {
        let (from, to) = (raw::BLACK, raw::YELLOW);
        assert_eq!(from.mix(to, 0.0), from);
        assert_eq!(from.mix(to, 1.0), to);

        let half = from.mix(to, 0.5);
        assert!(close(half.r, (from.r + to.r) / 2.0));
        // Fora da faixa o valor é preso nas pontas, não extrapolado.
        assert_eq!(from.mix(to, 5.0), to);
    }

    #[test]
    fn compositing_over_an_opaque_backdrop_stays_opaque() {
        let solid = ROSE.hover_fill().over(ROSE.base_100);
        assert!(close(solid.a, 1.0));
        // 8% de amarelo clareia o preto um pouco, sem chegar perto do amarelo.
        assert!(solid.r > ROSE.base_100.r && solid.r < 0.2);
    }

    #[test]
    fn transparent_over_transparent_is_transparent() {
        let empty = Color::rgba(0xFFFFFF, 0.0).over(Color::rgba(0x000000, 0.0));
        assert_eq!(empty.a, 0.0);
    }

    /// §2.2: os dois temas copiados do guia, papel por papel.
    #[test]
    fn the_two_themes_map_the_raw_palette_like_the_styleguide() {
        // Claro: amarelo na frente, preto de apoio.
        assert_eq!(CRIMSON.base_100, raw::YELLOW);
        assert_eq!(CRIMSON.base_200, raw::YELLOW_RAISED);
        assert_eq!(CRIMSON.base_300, raw::BLACK);
        assert_eq!(CRIMSON.accent, raw::BLACK);
        assert_eq!(CRIMSON.accent_content, raw::YELLOW);
        assert_eq!(CRIMSON.shadow, raw::BLACK);

        // Escuro: o contrário.
        assert_eq!(ROSE.base_100, raw::BLACK);
        assert_eq!(ROSE.base_200, raw::BLACK_RAISED);
        // No escuro a moldura é a cor clara do texto, e a sombra é accent.
        assert_eq!(ROSE.base_300, raw::YELLOW);
        assert_eq!(ROSE.base_300, ROSE.content);
        assert_eq!(ROSE.accent, raw::YELLOW);
        assert_eq!(ROSE.shadow, raw::YELLOW);
    }

    /// §2.4: texto a 4.5:1, conferido nos dois temas.
    ///
    /// Texto neutro e accent valem nos dois fundos. Os `-text` de status foram
    /// conferidos pelo guia sobre a superfície elevada (é onde status aparece:
    /// dentro de painel, toast e banner). A página nunca recebe texto de
    /// status: lá o status vai num quadrado de cor, com o rótulo em `content`.
    #[test]
    fn every_text_token_passes_aa_where_it_is_used() {
        for palette in [&CRIMSON, &ROSE] {
            for surface in [palette.base_100, palette.base_200] {
                let tokens = [
                    ("content", palette.content),
                    ("muted", palette.muted.over(surface)),
                    ("accent_text", palette.accent_text),
                ];
                for (name, color) in tokens {
                    let ratio = color.contrast(surface);
                    assert!(ratio >= 4.5, "{name} a {ratio:.2}:1");
                }
            }
            let surface = palette.base_200;
            for (name, status) in [
                ("info", palette.info),
                ("success", palette.success),
                ("warning", palette.warning),
                ("error", palette.error),
            ] {
                let ratio = status.text.contrast(surface);
                assert!(ratio >= 4.5, "{name}-text a {ratio:.2}:1");
            }
            // O conteúdo de cada fill também.
            let fills = [
                ("accent", palette.accent, palette.accent_content),
                ("info", palette.info.fill, palette.info.content),
                ("success", palette.success.fill, palette.success.content),
                ("warning", palette.warning.fill, palette.warning.content),
                ("error", palette.error.fill, palette.error.content),
            ];
            for (name, fill, content) in fills {
                let ratio = content.contrast(fill);
                assert!(ratio >= 4.5, "{name}-content a {ratio:.2}:1");
            }
        }
    }

    /// O global do processo não é trocado aqui: os testes rodam em paralelo e
    /// leem `palette()`. O que se confere é o mapeamento e o padrão.
    #[test]
    fn each_theme_has_its_palette_and_rose_is_the_default() {
        assert_eq!(Palette::of(Theme::Crimson), &CRIMSON);
        assert_eq!(Palette::of(Theme::Rose), &ROSE);
        assert_eq!(current(), Theme::Rose);
        assert_eq!(palette(), &ROSE);
        assert_eq!(encode(Theme::Crimson), 1);
    }

    #[test]
    fn the_enter_curve_eases_out_without_overshoot() {
        assert!(close(motion::ease_out(0.0), 0.0));
        assert!(close(motion::ease_out(1.0), 1.0));
        let mut previous = 0.0;
        for step in 1..=20 {
            let value = motion::ease_out(step as f32 / 20.0);
            assert!(value >= previous && value <= 1.0 + 1e-4, "{value}");
            previous = value;
        }
        // Ease-out: a primeira metade anda mais que a segunda.
        assert!(motion::ease_out(0.5) > 0.75);
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
