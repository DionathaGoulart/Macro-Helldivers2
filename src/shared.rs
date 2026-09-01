//! Estado compartilhado entre as threads e os canais que as ligam.
//!
//! O caminho quente (atalho → primeira tecla) NÃO passa pela UI: o hook manda
//! direto pro engine e a interface é avisada depois. Por isso os comandos do
//! engine já chegam com tudo resolvido — nada de consultar settings no meio.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, RwLock};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::data::Dir;
use crate::keys::Scan;
use crate::settings::{Settings, Speed, SLOT_COUNT};

/// Ids de estratagema equipados nos 4 slots.
pub type Slots = [Option<u32>; SLOT_COUNT];

/// O overlay nunca é escondido de verdade (mostrar janela rouba foco do jogo);
/// o que muda é o estado, e com ele os bounds das janelas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OverlayState {
    #[default]
    Hidden,
    Minimal,
    Panel,
}

/// Piscada de um slot: amarela quando dispara, vermelha quando é bloqueado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashKind {
    Triggered,
    Blocked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available { version: String },
    UpToDate,
    Downloading { percent: f32 },
    Ready { version: String },
    Error { message: String },
}

/// Ordem de execução para a thread do engine.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineCmd {
    Run {
        codex: Vec<Dir>,
        /// Scancode do modificador, já resolvido: o hook não pode pagar lookup.
        modifier: Scan,
        use_arrows: bool,
        speed: Speed,
        slot: usize,
        support: bool,
    },
}

/// Aviso para a janela principal, drenado quando ela recebe o `WM_APP` de UI.
#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    GameFocus(bool),
    MacroTriggered {
        slot: usize,
        support: bool,
    },
    MacroBlocked {
        slot: usize,
        support: bool,
    },
    MacroStatus {
        slot: usize,
        support: bool,
        running: bool,
    },
    FullscreenWarning(bool),
    OverlayState(OverlayState),
    UpdateStatus(UpdateStatus),
}

/// Ordem para a thread do overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayCmd {
    SetState(OverlayState),
    Toggle,
    Slots(Slots),
    LoadoutsChanged,
    Flash {
        slot: usize,
        kind: FlashKind,
    },
    /// Reafirma o z-order acima do jogo (alt-tab, troca de modo de vídeo).
    Reassert,
    FullscreenWarning(bool),
}

/// Pontas de recepção dos canais, entregues às threads donas de cada um.
pub struct Receivers {
    pub engine: Receiver<EngineCmd>,
    pub overlay: Receiver<OverlayCmd>,
    pub ui: Receiver<UiEvent>,
}

pub struct Shared {
    pub settings: RwLock<Settings>,
    pub slots: RwLock<Slots>,
    pub game_focused: AtomicBool,
    /// Capturando um atalho: o hook repassa tudo e não dispara nada.
    pub recording: AtomicBool,
    pub macro_running: AtomicBool,
    pub overlay_state: RwLock<OverlayState>,
    pub engine_tx: Sender<EngineCmd>,
    pub overlay_tx: Sender<OverlayCmd>,
    pub ui_tx: Sender<UiEvent>,
    /// HWNDs como `isize` para qualquer thread poder chamar `PostMessageW`.
    pub main_hwnd: AtomicIsize,
    pub overlay_hwnd: AtomicIsize,
}

impl Shared {
    pub fn new(settings: Settings, slots: Slots) -> (Arc<Shared>, Receivers) {
        let (engine_tx, engine_rx) = unbounded();
        let (overlay_tx, overlay_rx) = unbounded();
        let (ui_tx, ui_rx) = unbounded();

        let shared = Arc::new(Shared {
            settings: RwLock::new(settings),
            slots: RwLock::new(slots),
            game_focused: AtomicBool::new(false),
            recording: AtomicBool::new(false),
            macro_running: AtomicBool::new(false),
            overlay_state: RwLock::new(OverlayState::Hidden),
            engine_tx,
            overlay_tx,
            ui_tx,
            main_hwnd: AtomicIsize::new(0),
            overlay_hwnd: AtomicIsize::new(0),
        });

        let receivers = Receivers {
            engine: engine_rx,
            overlay: overlay_rx,
            ui: ui_rx,
        };
        (shared, receivers)
    }

    // As flags são independentes entre si e ninguém publica dados através
    // delas, então `Relaxed` basta e evita barreira no callback do hook.

    pub fn is_game_focused(&self) -> bool {
        self.game_focused.load(Ordering::Relaxed)
    }

    pub fn set_game_focused(&self, focused: bool) {
        self.game_focused.store(focused, Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }

    pub fn set_recording(&self, recording: bool) {
        self.recording.store(recording, Ordering::Relaxed);
    }

    /// Cópia dos settings. Leitores de longa duração devem travar direto.
    pub fn settings_snapshot(&self) -> Settings {
        self.settings.read().expect("settings envenenado").clone()
    }

    pub fn set_settings(&self, settings: Settings) {
        *self.settings.write().expect("settings envenenado") = settings;
    }

    pub fn slots(&self) -> Slots {
        *self.slots.read().expect("slots envenenado")
    }

    pub fn set_slots(&self, slots: Slots) {
        *self.slots.write().expect("slots envenenado") = slots;
    }

    pub fn overlay_state(&self) -> OverlayState {
        *self.overlay_state.read().expect("overlay_state envenenado")
    }

    pub fn set_overlay_state(&self, state: OverlayState) {
        *self
            .overlay_state
            .write()
            .expect("overlay_state envenenado") = state;
    }

    // Canal fechado significa que a thread destino já morreu (encerramento do
    // app); o remetente não tem o que fazer a respeito.

    pub fn send_engine(&self, cmd: EngineCmd) {
        let _ = self.engine_tx.send(cmd);
    }

    pub fn send_overlay(&self, cmd: OverlayCmd) {
        let _ = self.overlay_tx.send(cmd);
    }

    pub fn send_ui(&self, event: UiEvent) {
        let _ = self.ui_tx.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;

    fn shared() -> (Arc<Shared>, Receivers) {
        Shared::new(Settings::default(), Slots::default())
    }

    #[test]
    fn starts_idle_and_hidden() {
        let (shared, _rx) = shared();
        assert!(!shared.is_game_focused());
        assert!(!shared.is_recording());
        assert_eq!(shared.overlay_state(), OverlayState::Hidden);
        assert_eq!(shared.slots(), [None; SLOT_COUNT]);
        assert_eq!(shared.main_hwnd.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn commands_reach_their_receiver() {
        let (shared, rx) = shared();
        let cmd = EngineCmd::Run {
            codex: vec![Dir::Up, Dir::Down],
            modifier: keys::modifier_scan("LeftControl"),
            use_arrows: false,
            speed: Speed::Turbo,
            slot: 2,
            support: false,
        };
        shared.send_engine(cmd.clone());
        assert_eq!(rx.engine.try_recv().unwrap(), cmd);

        shared.send_overlay(OverlayCmd::Flash {
            slot: 1,
            kind: FlashKind::Blocked,
        });
        assert_eq!(
            rx.overlay.try_recv().unwrap(),
            OverlayCmd::Flash {
                slot: 1,
                kind: FlashKind::Blocked
            }
        );

        shared.send_ui(UiEvent::GameFocus(true));
        assert_eq!(rx.ui.try_recv().unwrap(), UiEvent::GameFocus(true));
    }

    #[test]
    fn sending_after_the_receiver_is_gone_is_harmless() {
        let (shared, rx) = shared();
        drop(rx);
        shared.send_engine(EngineCmd::Run {
            codex: vec![Dir::Left],
            modifier: keys::modifier_scan("LeftAlt"),
            use_arrows: true,
            speed: Speed::Normal,
            slot: 0,
            support: true,
        });
        shared.send_ui(UiEvent::FullscreenWarning(true));
    }

    #[test]
    fn state_is_visible_across_threads() {
        let (shared, _rx) = shared();
        let worker = Arc::clone(&shared);
        std::thread::spawn(move || {
            worker.set_game_focused(true);
            worker.set_slots([Some(7), None, None, None]);
            worker.set_overlay_state(OverlayState::Minimal);
        })
        .join()
        .unwrap();

        assert!(shared.is_game_focused());
        assert_eq!(shared.slots()[0], Some(7));
        assert_eq!(shared.overlay_state(), OverlayState::Minimal);
    }

    #[test]
    fn settings_snapshot_is_detached_from_later_writes() {
        let (shared, _rx) = shared();
        let before = shared.settings_snapshot();
        let mut changed = before.clone();
        changed.use_arrows = !before.use_arrows;
        shared.set_settings(changed);

        assert_ne!(before.use_arrows, shared.settings_snapshot().use_arrows);
    }
}
