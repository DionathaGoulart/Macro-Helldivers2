//! Painel de teclas do modo debug: a última sequência que o macro digitou,
//! acesa tecla a tecla enquanto sai, com o tempo real que cada direção ficou
//! segurada.
//!
//! Serve para gravar a tela jogando: no vídeo dá para comparar, quadro a
//! quadro, o que o macro mandou com as setas que o jogo acendeu no menu de
//! estratagema. A janela é click-through e só aparece com o jogo na frente.
//!
//! Puro como o strip: só empurra nós num [`Ui`], então roda e é testado no host.

use std::sync::Arc;

use crate::data::{Dir, GameData, SUPPORT_STRATS};
use crate::diag::RunOutcome;
use crate::i18n;
use crate::settings::{Settings, Speed};
use crate::shared::{DebugKeys, Slots};
use crate::ui::theme;
use crate::ui::toolkit::{Align, Measure, Rect, Ui};
use crate::ui::widgets::{self, styles};

/// Tamanho do painel, sem a sombra.
pub const WIDTH: f32 = 330.0;
pub const HEIGHT: f32 = 112.0;
/// Distância até a borda direita da tela.
pub const MARGIN: f32 = 24.0;
const PADDING: f32 = 12.0;
const HEADER_H: f32 = 14.0;
const NAME_H: f32 = 16.0;
const KEY: f32 = 26.0;
const KEY_GAP: f32 = 6.0;
const HOLD_H: f32 = 12.0;

/// O que cada tecla fez até agora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyState {
    Waiting,
    Down,
    Released,
    /// O Windows recusou a injeção. Fica marcada até a próxima chamada.
    Rejected,
}

#[derive(Debug, Clone, PartialEq)]
struct Shown {
    slot: usize,
    support: bool,
    modifier: &'static str,
    codex: Arc<[Dir]>,
    speed: Speed,
    /// O modificador no 0 e as direções a partir do 1, como os passos de
    /// [`DebugKeys::Key`].
    keys: Vec<KeyState>,
    outcome: Option<RunOutcome>,
    holds_ms: Vec<f32>,
}

/// Estado do painel: a última chamada, ou nada ainda.
#[derive(Debug, Default)]
pub struct Hud {
    shown: Option<Shown>,
}

pub struct Ctx<'a> {
    pub data: &'a GameData,
    pub settings: &'a Settings,
    pub slots: Slots,
    /// Limite de FPS do jogo: um hold mais curto que um quadro sai em vermelho.
    pub fps_cap: Option<u32>,
}

impl Hud {
    pub fn apply(&mut self, cmd: DebugKeys) {
        match cmd {
            DebugKeys::Start {
                slot,
                support,
                modifier,
                codex,
                speed,
            } => {
                self.shown = Some(Shown {
                    slot,
                    support,
                    modifier,
                    keys: vec![KeyState::Waiting; codex.len() + 1],
                    codex,
                    speed,
                    outcome: None,
                    holds_ms: Vec::new(),
                });
            }
            DebugKeys::Key { step, up, ok } => {
                let Some(key) = self
                    .shown
                    .as_mut()
                    .and_then(|shown| shown.keys.get_mut(step))
                else {
                    return;
                };
                *key = match (*key, ok, up) {
                    (KeyState::Rejected, _, _) | (_, false, _) => KeyState::Rejected,
                    (_, true, false) => KeyState::Down,
                    (_, true, true) => KeyState::Released,
                };
            }
            DebugKeys::End { outcome, holds_ms } => {
                if let Some(shown) = &mut self.shown {
                    shown.outcome = Some(outcome);
                    shown.holds_ms = holds_ms;
                }
            }
        }
    }

    pub fn build(&self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        let palette = theme::palette();
        let text = &i18n::tr(ctx.settings.language);
        let panel = Rect::new(area.x, area.y, WIDTH, HEIGHT);
        ui.fill(
            panel.translate(theme::SHADOW_SM, theme::SHADOW_SM),
            palette.shadow,
        );
        ui.fill(panel, palette.base_200);

        let mut content = panel.inset(PADDING);
        let mut header = content.cut_top(HEADER_H);
        let speed = self
            .shown
            .as_ref()
            .map_or(ctx.settings.macro_speed, |shown| shown.speed);
        let (status, status_color) = match self.shown.as_ref().map(|shown| shown.outcome) {
            Some(Some(RunOutcome::Completed)) => (
                text.debug.outcome(RunOutcome::Completed),
                palette.success.text,
            ),
            Some(Some(outcome)) => (text.debug.outcome(outcome), palette.error.text),
            Some(None) => ("...", palette.accent_text),
            None => ("", palette.muted),
        };
        let status = status.to_uppercase();
        let style = styles::micro_black().align(Align::End).middle();
        let width = measure.text_size(&status, style, f32::INFINITY).0;
        ui.text(header.cut_right(width), status, style, status_color);
        ui.text(
            header,
            format!(
                "DEBUG \u{00B7} {}",
                text.settings.speed_name(speed).to_uppercase()
            ),
            styles::micro_black().middle(),
            palette.accent_text,
        );
        content.skip_top(4.0);

        let mut name_row = content.cut_top(NAME_H);
        let Some(shown) = &self.shown else {
            ui.text(
                name_row,
                text.debug.hud_waiting.to_uppercase(),
                styles::label().middle(),
                palette.muted,
            );
            ui.stroke(panel, theme::BORDER, palette.base_300);
            return;
        };
        if let Some(shortest) = shown.holds_ms.iter().copied().reduce(f32::min) {
            let label = text.debug.hud_min_hold(shortest);
            let style = styles::micro().align(Align::End).middle();
            let width = measure.text_size(&label, style, f32::INFINITY).0;
            let color = if below_frame(shortest, ctx.fps_cap) {
                palette.error.text
            } else {
                palette.muted
            };
            ui.text(name_row.cut_right(width), label, style, color);
        }
        ui.text(
            name_row,
            stratagem_name(shown, ctx).to_uppercase(),
            styles::label().middle(),
            palette.content,
        );
        content.skip_top(8.0);

        let count = shown.keys.len();
        let size =
            ((content.w - KEY_GAP * (count.saturating_sub(1)) as f32) / count as f32).min(KEY);
        let row = content.cut_top(size);
        content.skip_top(4.0);
        let holds = content.cut_top(HOLD_H);
        for (step, state) in shown.keys.iter().enumerate() {
            let chip = Rect::new(row.x + (size + KEY_GAP) * step as f32, row.y, size, size);
            key_chip(
                ui,
                chip,
                *state,
                step.checked_sub(1).map(|dir| shown.codex[dir]),
                shown.modifier,
            );
            let Some(hold) = step.checked_sub(1).and_then(|dir| shown.holds_ms.get(dir)) else {
                continue;
            };
            let color = if below_frame(*hold, ctx.fps_cap) {
                palette.error.text
            } else {
                palette.muted
            };
            ui.text(
                Rect::new(chip.x - KEY_GAP / 2.0, holds.y, size + KEY_GAP, HOLD_H),
                format!("{hold:.0}"),
                styles::micro().align(Align::Center).middle(),
                color,
            );
        }
        ui.stroke(panel, theme::BORDER, palette.base_300);
    }
}

/// Um hold mais curto que um quadro no limite de FPS do jogo: é o que o jogo
/// pode deixar de ver.
fn below_frame(hold_ms: f32, fps_cap: Option<u32>) -> bool {
    fps_cap.is_some_and(|fps| fps > 0 && hold_ms < 1_000.0 / fps as f32)
}

fn stratagem_name(shown: &Shown, ctx: &Ctx) -> String {
    if shown.support {
        return SUPPORT_STRATS
            .get(shown.slot)
            .map_or_else(String::new, |strat| strat.nome.to_string());
    }
    ctx.slots
        .get(shown.slot)
        .copied()
        .flatten()
        .and_then(|id| ctx.data.by_id(id))
        .map_or_else(
            || format!("slot {}", shown.slot + 1),
            |strat| strat.nome.clone(),
        )
}

/// Uma tecla: o rótulo do modificador ou a seta da direção, na cor do estado.
fn key_chip(ui: &mut Ui, rect: Rect, state: KeyState, dir: Option<Dir>, modifier: &str) {
    let palette = theme::palette();
    let (fill, ink, frame) = match state {
        KeyState::Waiting => (palette.base_100, palette.muted, palette.base_300),
        KeyState::Down => (palette.accent, palette.accent_content, palette.accent),
        KeyState::Released => (palette.base_100, palette.success.text, palette.success.fill),
        KeyState::Rejected => (
            palette.error.fill,
            palette.error.content,
            palette.error.fill,
        ),
    };
    ui.fill(rect, fill);
    ui.stroke(rect, theme::BORDER, frame);
    match dir {
        Some(dir) => widgets::arrow(ui, rect.inset(rect.w * 0.2), dir, ink),
        None => ui.text(
            rect,
            modifier,
            styles::micro_black().align(Align::Center).middle(),
            ink,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::{TextStyle, Visual};

    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, _max: f32) -> (f32, f32) {
            (text.chars().count() as f32 * style.size * 0.6, style.size)
        }
    }

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
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

    fn build(hud: &Hud, data: &GameData, fps_cap: Option<u32>) -> Ui {
        let settings = Settings::default();
        let mut ui = Ui::new();
        ui.begin(0);
        hud.build(
            &mut ui,
            &mut Fixed,
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            &Ctx {
                data,
                settings: &settings,
                slots: [None; crate::settings::SLOT_COUNT],
                fps_cap,
            },
        );
        ui.end();
        ui
    }

    fn start(hud: &mut Hud) {
        hud.apply(DebugKeys::Start {
            slot: 0,
            support: true,
            modifier: "CTRL",
            codex: SUPPORT_STRATS[0].codex.into(),
            speed: Speed::Low,
        });
    }

    #[test]
    fn it_waits_for_the_first_call() {
        let data = data();
        let ui = build(&Hud::default(), &data, None);
        assert!(texts(&ui).iter().any(|text| text == "AGUARDANDO DISPARO"));
    }

    #[test]
    fn keys_light_up_as_they_go_and_rejections_stick() {
        let mut hud = Hud::default();
        start(&mut hud);
        let keys = |hud: &Hud| hud.shown.as_ref().unwrap().keys.clone();
        assert_eq!(keys(&hud).len(), 6, "modificador + cinco direções");

        hud.apply(DebugKeys::Key {
            step: 0,
            up: false,
            ok: true,
        });
        hud.apply(DebugKeys::Key {
            step: 1,
            up: false,
            ok: false,
        });
        hud.apply(DebugKeys::Key {
            step: 1,
            up: true,
            ok: true,
        });
        assert_eq!(keys(&hud)[0], KeyState::Down);
        assert_eq!(keys(&hud)[1], KeyState::Rejected);
        assert_eq!(keys(&hud)[2], KeyState::Waiting);

        // Passo fora da sequência não derruba nada.
        hud.apply(DebugKeys::Key {
            step: 40,
            up: true,
            ok: true,
        });
    }

    #[test]
    fn the_end_shows_the_outcome_and_flags_holds_shorter_than_a_frame() {
        let data = data();
        let mut hud = Hud::default();
        start(&mut hud);
        hud.apply(DebugKeys::End {
            outcome: RunOutcome::Completed,
            holds_ms: vec![48.0, 51.0, 30.0, 50.0, 49.0],
        });
        let ui = build(&hud, &data, Some(30));
        let all = texts(&ui);
        assert!(all.iter().any(|text| text == "REINFORCE"), "{all:?}");
        assert!(all.iter().any(|text| text == "COMPLETO"));
        assert!(all.iter().any(|text| text == "DEBUG \u{00B7} BAIXO FPS"));
        assert!(all.iter().any(|text| text == "menor 30 ms"));
        assert!(below_frame(30.0, Some(30)));
        assert!(!below_frame(34.0, Some(30)));
        assert!(!below_frame(10.0, None));
    }
}
