//! Overlay: duas janelas layered em Direct2D desenhando por cima do jogo.
//!
//! A regra que manda em tudo aqui vem da v1 (`legacy/src/main/index.js` ~13-19):
//! **as janelas nunca são escondidas**. Mostrar uma janela transparente no
//! Windows a ativa, e ativar qualquer coisa rouba o foco do jogo — que, em tela
//! cheia, se minimiza. O que muda entre os estados são os *bounds*: `hidden`
//! encolhe as duas para 1×1 num canto, onde o compositor praticamente não tem o
//! que compor.
//!
//! - `strip` — barra de slots, sempre click-through (`WS_EX_TRANSPARENT`).
//! - `panel` — painel interativo; recebe o mouse por ser `WS_EX_NOACTIVATE`, que
//!   deixa a janela clicável sem nunca virar a janela ativa.
//!
//! A thread só existe enquanto `enableOverlay` estiver ligado ([`set_enabled`]).
//! Ela é dona das duas janelas, do seu próprio pump e dos alvos de render; o
//! resto do app fala com ela pelo canal de [`OverlayCmd`](crate::shared::OverlayCmd).
//!
//! Este módulo guarda a parte que não depende do Windows — a geometria dos
//! estados e o que o atalho faz —, testada no host.

pub mod panel;
pub mod strip;

use crate::shared::OverlayState;
use crate::ui::theme::Scale;
use crate::ui::window::Bounds;

/// Tamanho do painel em DIP: os 820×640 do conteúdo mais a folga da sombra.
pub const PANEL_WIDTH: f32 = 840.0;
pub const PANEL_HEIGHT: f32 = 660.0;
/// Tamanho do strip: os quatro slots escalados mais a folga do brilho.
pub const STRIP_WIDTH: f32 = 340.0;
pub const STRIP_HEIGHT: f32 = 130.0;

/// Qual das duas janelas do overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    Strip,
    Panel,
}

/// O overlay tem conteúdo na tela?
pub fn is_visible(state: OverlayState) -> bool {
    state != OverlayState::Hidden
}

/// Para onde o Ctrl+H leva (`toggleOverlay` da v1): o painel fecha para o HUD
/// persistente, quando ele está ligado, e some de vez quando não.
pub fn toggled(state: OverlayState, always_show_slots: bool) -> OverlayState {
    match state {
        OverlayState::Panel if always_show_slots => OverlayState::Minimal,
        OverlayState::Panel => OverlayState::Hidden,
        _ => OverlayState::Panel,
    }
}

/// Bounds de uma das janelas no estado dado, em pixels do monitor primário
/// (R11 — o overlay vive no primário, como na v1).
///
/// A janela que não tem o que mostrar naquele estado vai para 1×1 no canto:
/// continua viva, e o compositor não paga por ela.
pub fn window_bounds(state: OverlayState, which: Which, monitor: Bounds, scale: Scale) -> Bounds {
    let size = match (state, which) {
        (OverlayState::Minimal, Which::Strip) => Some((STRIP_WIDTH, STRIP_HEIGHT)),
        (OverlayState::Panel, Which::Panel) => Some((PANEL_WIDTH, PANEL_HEIGHT)),
        _ => None,
    };
    let Some((width, height)) = size else {
        return Bounds {
            x: monitor.x,
            y: monitor.y,
            width: 1,
            height: 1,
        };
    };

    let (width, height) = (scale.px(width), scale.px(height));
    Bounds {
        x: monitor.x + (monitor.width - width) / 2,
        y: match which {
            // O strip mora encostado na base da tela, onde o CSS o punha.
            Which::Strip => monitor.y + monitor.height - height,
            Which::Panel => monitor.y + (monitor.height - height) / 2,
        },
        width,
        height,
    }
}

#[cfg(windows)]
pub use platform::{init, set_enabled};

/// Fora do Windows não há overlay: o binário serve para `check` e `test`.
#[cfg(not(windows))]
pub fn init(
    _shared: std::sync::Arc<crate::shared::Shared>,
    _data: std::sync::Arc<crate::data::GameData>,
    _rx: crossbeam_channel::Receiver<crate::shared::OverlayCmd>,
) {
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) {}

#[cfg(windows)]
mod platform {
    use std::cell::RefCell;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex, OnceLock};
    use std::thread::JoinHandle;

    use crossbeam_channel::Receiver;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, ScreenToClient, AC_SRC_ALPHA, AC_SRC_OVER,
        BLENDFUNCTION, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::SystemInformation::GetTickCount64;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Controls::WM_MOUSELEAVE;
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::*;

    use super::{panel, strip, Which};
    use crate::data::GameData;
    use crate::gfx::d2d::LayeredSurface;
    use crate::gfx::text::Text;
    use crate::shared::{
        FlashKind, OverlayCmd, OverlayState, Shared, Slots, UiEvent, WM_APP_OVERLAY,
    };
    use crate::ui::theme::{self, Scale};
    use crate::ui::toolkit::{Input, Rect, Ui};
    use crate::ui::widgets;
    use crate::ui::window::Bounds;
    use crate::{focus, hooks, loadouts};

    /// Uma classe só para as duas janelas: o que as diferencia é o estilo
    /// estendido, escolhido na criação.
    const CLASS_NAME: PCWSTR = w!("HD2OverlayWindow");
    /// Timer de animação, criado só enquanto uma piscada ou um fade corre (R14).
    const TIMER_ANIM: usize = 1;
    const ANIM_INTERVAL_MS: u32 = 16;

    struct Runtime {
        shared: Arc<Shared>,
        data: Arc<GameData>,
        rx: Receiver<OverlayCmd>,
        worker: Mutex<Option<Worker>>,
    }

    /// A thread viva e o id que recebe o `WM_QUIT` que a encerra.
    struct Worker {
        handle: JoinHandle<()>,
        thread_id: u32,
    }

    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    /// Guarda o que a thread do overlay precisa. Chamar uma vez, no boot.
    pub fn init(shared: Arc<Shared>, data: Arc<GameData>, rx: Receiver<OverlayCmd>) {
        let runtime = Runtime {
            shared,
            data,
            rx,
            worker: Mutex::new(None),
        };
        if RUNTIME.set(runtime).is_err() {
            log::warn!("overlay::init chamado mais de uma vez");
        }
    }

    /// Sobe ou derruba a thread do overlay conforme `enableOverlay`.
    pub fn set_enabled(enabled: bool) {
        let Some(runtime) = RUNTIME.get() else {
            log::warn!("overlay::set_enabled antes de overlay::init");
            return;
        };
        let mut worker = runtime.worker.lock().unwrap_or_else(|err| err.into_inner());

        if enabled {
            if worker.is_some() {
                return;
            }
            *worker = start(runtime);
            return;
        }

        let Some(current) = worker.take() else {
            return;
        };
        // SAFETY: id de uma thread viva, cuja fila de mensagens já existe (ela é
        // criada antes de a thread anunciar o id).
        if let Err(err) =
            unsafe { PostThreadMessageW(current.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) }
        {
            log::warn!("overlay não recebeu o pedido de encerramento: {err}");
        }
        if current.handle.join().is_err() {
            log::error!("a thread do overlay terminou em panic");
        }
        // Sem ninguém para ler, os comandos que sobraram no canal só ocupariam
        // memória até o overlay voltar a ser ligado.
        while runtime.rx.try_recv().is_ok() {}
        log::info!("overlay desligado");
    }

    fn start(runtime: &'static Runtime) -> Option<Worker> {
        let (shared, data, rx) = (
            Arc::clone(&runtime.shared),
            Arc::clone(&runtime.data),
            runtime.rx.clone(),
        );
        // A thread avisa o id assim que a fila de mensagens existe; sem isso o
        // `PostThreadMessageW` do desligamento poderia chegar antes dela.
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("overlay".to_string())
            .spawn(move || run(shared, data, rx, ready_tx))
            .inspect_err(|err| log::error!("thread do overlay não subiu: {err}"))
            .ok()?;

        match ready_rx.recv() {
            Ok(thread_id) => {
                log::info!("overlay ligado");
                Some(Worker { handle, thread_id })
            }
            Err(_) => {
                log::error!("a thread do overlay morreu antes de iniciar");
                let _ = handle.join();
                None
            }
        }
    }

    fn run(
        shared: Arc<Shared>,
        data: Arc<GameData>,
        rx: Receiver<OverlayCmd>,
        ready: std::sync::mpsc::Sender<u32>,
    ) {
        // Uma thread só ganha fila de mensagens na primeira chamada que a exija;
        // forçá-la aqui garante que o `WM_QUIT` do desligamento tenha onde cair.
        let mut msg = MSG::default();
        // SAFETY: `msg` é nosso e vive durante a chamada.
        unsafe {
            let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE);
        }
        // SAFETY: leitura do id da própria thread.
        let _ = ready.send(unsafe { GetCurrentThreadId() });

        if let Err(err) = register_class() {
            log::error!("classe do overlay não registrou: {err}");
            return;
        }
        let mut app = match App::new(shared, data, rx) {
            Ok(app) => app,
            Err(err) => {
                log::error!("overlay não pôde iniciar: {err:#}");
                return;
            }
        };
        pump(&mut app);
    }

    fn register_class() -> windows::core::Result<()> {
        // SAFETY: `None` pede o módulo do próprio processo.
        let instance = unsafe { GetModuleHandleW(None) }?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            // SAFETY: cursor padrão do sistema.
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        // SAFETY: a classe vive durante a chamada. Registrar de novo depois de um
        // ciclo desliga/liga devolve zero com `ERROR_CLASS_ALREADY_EXISTS`, e a
        // classe antiga continua valendo.
        if unsafe { RegisterClassW(&class) } == 0 {
            let err = windows::core::Error::from_thread();
            if err.code() != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS.to_hresult() {
                return Err(err);
            }
        }
        Ok(())
    }

    // --- Eventos ---

    /// O que o `WndProc` viu. Ele não toca no [`App`]: só empilha, e o pump
    /// processa com posse exclusiva. Assim nenhuma chamada reentrante (um
    /// `SetWindowPos` que gera mensagens no meio do tratamento) pode produzir
    /// dois empréstimos mutáveis do mesmo estado.
    enum Event {
        /// Há comandos para drenar do canal.
        Commands,
        /// Tique de animação de uma das janelas.
        Animate(isize),
        /// Mouse no painel, em pixels de cliente.
        Pointer(Pointer),
        /// Monitor ou DPI mudou: recalcular bounds.
        Display,
    }

    enum Pointer {
        Move { x: i32, y: i32 },
        Leave,
        Down { x: i32, y: i32 },
        Up { x: i32, y: i32 },
        Wheel { x: i32, y: i32, delta: f32 },
    }

    thread_local! {
        static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    }

    fn push(event: Event) {
        EVENTS.with(|events| events.borrow_mut().push(event));
    }

    fn take_events() -> Vec<Event> {
        EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
    }

    fn pump(app: &mut App) {
        let mut msg = MSG::default();
        loop {
            // SAFETY: `msg` é nosso e vive durante a chamada.
            let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
            match result.0 {
                0 => break,
                -1 => {
                    log::error!("GetMessageW falhou na thread do overlay");
                    break;
                }
                _ => {
                    // SAFETY: mensagem recém-preenchida pelo `GetMessageW`.
                    unsafe {
                        DispatchMessageW(&msg);
                    }
                    app.process_events();
                }
            }
        }
    }

    // --- Janela layered ---

    /// Uma das duas janelas: o HWND, o DIB que a compõe e os bounds atuais.
    struct Window {
        hwnd: HWND,
        surface: Option<LayeredSurface>,
        bounds: Bounds,
        dpi: u32,
        anim_timer: bool,
    }

    impl Window {
        fn create(click_through: bool, dpi: u32) -> anyhow::Result<Window> {
            let mut ex_style = WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
            if click_through {
                ex_style |= WS_EX_TRANSPARENT;
            }
            // O título é o que a classificação de foco procura (R10): as duas
            // janelas contam como "nossas", e o app não se desarma quando o
            // mouse leva o foco para o painel.
            let title = wide(focus::OVERLAY_WINDOW_TITLE);
            // SAFETY: a classe está registrada e o título vive durante a chamada.
            let hwnd = unsafe {
                CreateWindowExW(
                    ex_style,
                    CLASS_NAME,
                    PCWSTR(title.as_ptr()),
                    WS_POPUP,
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    None,
                    None,
                )
            }?;

            // Mostrada uma única vez, sem ativar, e nunca mais escondida.
            // SAFETY: janela recém-criada.
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            Ok(Window {
                hwnd,
                surface: None,
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                dpi,
                anim_timer: false,
            })
        }

        fn set_bounds(&mut self, bounds: Bounds) {
            if self.bounds == bounds {
                return;
            }
            self.bounds = bounds;
            // SAFETY: janela viva; `NOACTIVATE` mantém o foco onde está.
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    SWP_NOACTIVATE,
                );
            }
        }

        /// Liga ou desliga o `WS_EX_TRANSPARENT`. O painel só recebe mouse
        /// quando é ele que está na tela; nos outros estados tudo atravessa para
        /// o jogo.
        fn set_click_through(&mut self, click_through: bool) {
            // SAFETY: leitura e escrita do estilo da própria janela.
            unsafe {
                let current = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32;
                let wanted = if click_through {
                    current | WS_EX_TRANSPARENT.0
                } else {
                    current & !WS_EX_TRANSPARENT.0
                };
                if wanted != current {
                    SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, wanted as isize);
                }
            }
        }

        fn set_dpi(&mut self, dpi: u32) {
            if self.dpi == dpi {
                return;
            }
            self.dpi = dpi;
            if let Some(surface) = &mut self.surface {
                surface.set_dpi(dpi);
            }
        }

        /// A janela está encolhida no canto (estado sem conteúdo)?
        fn is_collapsed(&self) -> bool {
            self.bounds.width <= 1 && self.bounds.height <= 1
        }

        /// Desenha a lista e entrega o DIB pronto ao compositor.
        fn present(&mut self, text: &mut Text, ui: &Ui) {
            if self.is_collapsed() {
                return;
            }
            let (width, height) = (self.bounds.width, self.bounds.height);
            let surface = match &mut self.surface {
                Some(surface) => surface,
                None => match LayeredSurface::new(width, height, self.dpi) {
                    Ok(surface) => self.surface.insert(surface),
                    Err(err) => {
                        log::error!("superfície do overlay não pôde ser criada: {err}");
                        return;
                    }
                },
            };
            surface.resize(width, height);
            surface.draw(text, ui.frame());

            let size = SIZE {
                cx: width,
                cy: height,
            };
            let origin = POINT { x: 0, y: 0 };
            let position = POINT {
                x: self.bounds.x,
                y: self.bounds.y,
            };
            // Alfa por pixel: o DIB já sai pré-multiplicado do Direct2D, que é o
            // que o `ULW_ALPHA` espera.
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            // SAFETY: janela e DC vivos; as estruturas vivem durante a chamada.
            let updated = unsafe {
                UpdateLayeredWindow(
                    self.hwnd,
                    None,
                    Some(&position),
                    Some(&size),
                    Some(surface.hdc()),
                    Some(&origin),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                )
            };
            if let Err(err) = updated {
                log::warn!("UpdateLayeredWindow falhou: {err}");
            }
        }

        /// O timer de 16ms só existe enquanto há animação (R14).
        fn sync_anim_timer(&mut self, animating: bool) {
            let wanted = animating && !self.is_collapsed();
            if wanted == self.anim_timer {
                return;
            }
            // SAFETY: timer da própria janela.
            unsafe {
                if wanted {
                    SetTimer(Some(self.hwnd), TIMER_ANIM, ANIM_INTERVAL_MS, None);
                } else {
                    let _ = KillTimer(Some(self.hwnd), TIMER_ANIM);
                }
            }
            self.anim_timer = wanted;
        }

        fn reassert(&self) {
            // SAFETY: janela viva; sem mover nem ativar.
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            // A superfície morre antes da janela: ela é quem segura o DIB.
            self.surface = None;
            // SAFETY: janela criada por esta thread, destruída uma vez só.
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }

    // --- Estado da thread ---

    struct App {
        shared: Arc<Shared>,
        data: Arc<GameData>,
        rx: Receiver<OverlayCmd>,
        text: Text,
        strip: Window,
        panel: Window,
        strip_ui: Ui,
        panel_ui: Ui,
        panel_state: panel::Panel,
        state: OverlayState,
        slots: Slots,
        monitor: Bounds,
        scale: Scale,
    }

    impl App {
        fn new(
            shared: Arc<Shared>,
            data: Arc<GameData>,
            rx: Receiver<OverlayCmd>,
        ) -> anyhow::Result<App> {
            let text = Text::new()?;
            let (monitor, dpi) = primary_monitor();
            let mut panel_state = panel::Panel::new();
            panel_state.reload_loadouts();

            let mut app = App {
                strip: Window::create(true, dpi)?,
                panel: Window::create(false, dpi)?,
                state: shared.overlay_state(),
                slots: shared.slots(),
                shared,
                data,
                rx,
                text,
                strip_ui: Ui::new(),
                panel_ui: Ui::new(),
                panel_state,
                monitor,
                scale: Scale::from_dpi(dpi),
            };

            // É o strip que recebe o `WM_APP_OVERLAY`: basta uma das janelas
            // para acordar o pump da thread.
            app.shared
                .overlay_hwnd
                .store(app.strip.hwnd.0 as isize, Ordering::Relaxed);

            // Comandos que chegaram antes de a janela existir não acordaram
            // ninguém; drenar aqui evita que fiquem esperando o próximo.
            app.apply_layout();
            app.drain_commands();
            app.redraw_all();
            Ok(app)
        }

        // --- Eventos ---

        /// Trata o que o `WndProc` empilhou. O laço existe porque tratar um
        /// evento pode gerar outros (um `SetWindowPos` no meio do caminho); o
        /// teto evita girar para sempre caso isso vire um ciclo.
        fn process_events(&mut self) {
            for _ in 0..4 {
                let events = take_events();
                if events.is_empty() {
                    return;
                }
                for event in events {
                    match event {
                        Event::Commands => {
                            self.drain_commands();
                        }
                        Event::Animate(hwnd) => self.animate(hwnd),
                        Event::Pointer(pointer) => self.on_pointer(pointer),
                        Event::Display => {
                            self.apply_layout();
                            self.redraw_all();
                        }
                    }
                }
            }
        }

        fn drain_commands(&mut self) {
            let mut state = None;
            let (mut strip_dirty, mut panel_dirty) = (false, false);

            while let Ok(cmd) = self.rx.try_recv() {
                match cmd {
                    OverlayCmd::SetState(next) => state = Some(next),
                    OverlayCmd::Toggle => {
                        // Mesmas condições da v1: sem jogo em foco, o Ctrl+H (e
                        // o × do painel) não fazem nada.
                        if self.shared.is_game_focused() {
                            let always = self.shared.settings_snapshot().always_show_slots;
                            state = Some(super::toggled(state.unwrap_or(self.state), always));
                        }
                    }
                    OverlayCmd::Slots(slots) => {
                        self.slots = slots;
                        strip_dirty = true;
                        panel_dirty = true;
                    }
                    OverlayCmd::LoadoutsChanged => {
                        self.panel_state.reload_loadouts();
                        panel_dirty = true;
                    }
                    OverlayCmd::SettingsChanged => {
                        strip_dirty = true;
                        panel_dirty = true;
                    }
                    OverlayCmd::Flash {
                        slot,
                        support,
                        kind,
                    } => {
                        // O overlay mostra os quatro slots de macro; a piscada
                        // de um apoio fixo não tem onde aparecer aqui.
                        if !support {
                            self.flash(slot, kind);
                            strip_dirty = true;
                            panel_dirty = true;
                        }
                    }
                    // O jogo re-agarra o topo do z-order em alt-tab e em troca
                    // de modo de vídeo. Quem vigia isso é o timer de 5s da
                    // thread de hooks, que já revalida o foco no mesmo ritmo —
                    // um segundo timer aqui só repetiria o trabalho dela.
                    OverlayCmd::Reassert => self.reassert(),
                    OverlayCmd::FullscreenWarning(warning) => {
                        self.panel_state.set_warning(warning);
                        panel_dirty = true;
                    }
                }
            }

            if let Some(state) = state {
                // A troca de estado já redesenha as duas janelas.
                self.set_state(state);
                return;
            }
            if strip_dirty {
                self.redraw_strip();
            }
            if panel_dirty {
                self.redraw_panel();
            }
        }

        fn flash(&mut self, slot: usize, kind: FlashKind) {
            let duration = match kind {
                FlashKind::Triggered => widgets::FLASH_TRIGGERED_MS,
                FlashKind::Blocked => widgets::FLASH_BLOCKED_MS,
            };
            let id = widgets::flash_id(slot, false, kind);
            let now = tick_ms();
            // O relógio de cada `Ui` parou na última passagem, que pode ser de
            // minutos atrás: sem acertá-lo, o pulso nasceria vencido.
            for ui in [&mut self.strip_ui, &mut self.panel_ui] {
                ui.set_now(now);
                ui.flash(id, duration);
            }
        }

        fn animate(&mut self, hwnd: isize) {
            if hwnd == self.strip.hwnd.0 as isize {
                self.redraw_strip();
            } else {
                self.redraw_panel();
            }
        }

        fn on_pointer(&mut self, pointer: Pointer) {
            if self.state != OverlayState::Panel {
                return;
            }
            let dip = |x: i32, y: i32| (self.scale.dip(x as f32), self.scale.dip(y as f32));
            let input = match pointer {
                Pointer::Move { x, y } => {
                    let (x, y) = dip(x, y);
                    Input::Move { x, y }
                }
                Pointer::Leave => Input::Leave,
                Pointer::Down { x, y } => {
                    let (x, y) = dip(x, y);
                    Input::Down { x, y }
                }
                Pointer::Up { x, y } => {
                    let (x, y) = dip(x, y);
                    Input::Up { x, y }
                }
                Pointer::Wheel { x, y, delta } => {
                    let (x, y) = dip(x, y);
                    Input::Wheel { x, y, delta }
                }
            };

            let response = self.panel_ui.input(input);
            if let Some(clicked) = response.clicked {
                self.on_panel_click(clicked);
            } else if response.redraw {
                self.redraw_panel();
            }
        }

        fn on_panel_click(&mut self, clicked: crate::ui::toolkit::Id) {
            let settings = self.shared.settings_snapshot();
            let ctx = panel::Ctx {
                data: &self.data,
                settings: &settings,
                slots: self.slots,
            };
            match self.panel_state.on_click(clicked, &ctx) {
                Some(panel::Action::Close) => {
                    let state = super::toggled(self.state, settings.always_show_slots);
                    self.set_state(state);
                }
                Some(panel::Action::SlotsChanged(slots)) => self.apply_slots(slots),
                _ => self.redraw_panel(),
            }
        }

        /// Espalha os slots que o painel mudou: estado, disco, tabela de atalhos
        /// e a janela principal, que mostra os mesmos quatro slots.
        fn apply_slots(&mut self, slots: Slots) {
            self.slots = slots;
            self.shared.set_slots(slots);
            loadouts::save_slots(&slots);
            hooks::rebuild_bindings();
            self.shared.send_ui(UiEvent::SlotsChanged(slots));
            self.redraw_all();
        }

        // --- Estados e geometria ---

        fn set_state(&mut self, state: OverlayState) {
            if self.state != state {
                self.state = state;
                self.shared.set_overlay_state(state);
                self.shared.send_ui(UiEvent::OverlayState(state));
            }
            self.apply_layout();
            self.redraw_all();
            if super::is_visible(state) {
                self.reassert();
            }
        }

        fn apply_layout(&mut self) {
            let (monitor, dpi) = primary_monitor();
            self.monitor = monitor;
            self.scale = Scale::from_dpi(dpi);
            self.strip.set_dpi(dpi);
            self.panel.set_dpi(dpi);

            self.strip.set_bounds(super::window_bounds(
                self.state,
                Which::Strip,
                monitor,
                self.scale,
            ));
            self.panel.set_bounds(super::window_bounds(
                self.state,
                Which::Panel,
                monitor,
                self.scale,
            ));
            // O painel só é clicável quando é ele que está na tela.
            self.panel
                .set_click_through(self.state != OverlayState::Panel);
        }

        fn reassert(&self) {
            self.strip.reassert();
            self.panel.reassert();
        }

        // --- Desenho ---

        fn redraw_all(&mut self) {
            self.redraw_strip();
            self.redraw_panel();
        }

        fn redraw_strip(&mut self) {
            if self.strip.is_collapsed() {
                self.strip.sync_anim_timer(false);
                return;
            }
            let settings = self.shared.settings_snapshot();
            let area = self.area(&self.strip);

            self.strip_ui.begin(tick_ms());
            strip::build(
                &mut self.strip_ui,
                &mut self.text,
                area,
                &strip::Ctx {
                    data: &self.data,
                    settings: &settings,
                    slots: self.slots,
                },
            );
            self.strip_ui.end();

            self.strip.present(&mut self.text, &self.strip_ui);
            self.strip.sync_anim_timer(self.strip_ui.animating());
        }

        fn redraw_panel(&mut self) {
            if self.panel.is_collapsed() {
                self.panel.sync_anim_timer(false);
                return;
            }
            let settings = self.shared.settings_snapshot();
            let area = self.area(&self.panel);

            self.panel_ui.begin(tick_ms());
            self.panel_state.build(
                &mut self.panel_ui,
                &mut self.text,
                area,
                &panel::Ctx {
                    data: &self.data,
                    settings: &settings,
                    slots: self.slots,
                },
            );
            self.panel_ui.end();

            self.panel.present(&mut self.text, &self.panel_ui);
            self.panel.sync_anim_timer(self.panel_ui.animating());
        }

        /// Área de desenho de uma janela, em DIP.
        fn area(&self, window: &Window) -> Rect {
            Rect::new(
                0.0,
                0.0,
                self.scale.dip(window.bounds.width as f32),
                self.scale.dip(window.bounds.height as f32),
            )
        }
    }

    impl Drop for App {
        fn drop(&mut self) {
            self.shared.overlay_hwnd.store(0, Ordering::Relaxed);
            // As janelas somem com o `Drop` de cada `Window`; o estado volta
            // para "escondido" porque é o que vale sem overlay na tela.
            self.shared.set_overlay_state(OverlayState::Hidden);
        }
    }

    // --- WndProc ---

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        // SAFETY: todo o corpo roda na thread dona das janelas, com os ponteiros
        // que o próprio Windows entrega. Nada aqui toca no `App`: os handlers só
        // empilham eventos (ver [`Event`]).
        unsafe {
            match message {
                WM_APP_OVERLAY => {
                    push(Event::Commands);
                    LRESULT(0)
                }
                WM_TIMER if wparam.0 == TIMER_ANIM => {
                    push(Event::Animate(hwnd.0 as isize));
                    LRESULT(0)
                }
                WM_MOUSEMOVE => {
                    // Re-arma o aviso de saída a cada movimento: sem estado para
                    // guardar, e o Windows ignora o pedido repetido.
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut track);
                    let (x, y) = mouse_point(lparam);
                    push(Event::Pointer(Pointer::Move { x, y }));
                    LRESULT(0)
                }
                WM_MOUSELEAVE => {
                    push(Event::Pointer(Pointer::Leave));
                    LRESULT(0)
                }
                WM_LBUTTONDOWN => {
                    let (x, y) = mouse_point(lparam);
                    push(Event::Pointer(Pointer::Down { x, y }));
                    LRESULT(0)
                }
                WM_LBUTTONUP => {
                    let (x, y) = mouse_point(lparam);
                    push(Event::Pointer(Pointer::Up { x, y }));
                    LRESULT(0)
                }
                WM_MOUSEWHEEL => {
                    // A roda chega em coordenadas de tela.
                    let (x, y) = mouse_point(lparam);
                    let mut point = POINT { x, y };
                    let _ = ScreenToClient(hwnd, &mut point);
                    let delta = hiword(wparam.0 as u32) as i16 as f32 / WHEEL_DELTA as f32;
                    push(Event::Pointer(Pointer::Wheel {
                        x: point.x,
                        y: point.y,
                        delta,
                    }));
                    LRESULT(0)
                }
                WM_DISPLAYCHANGE | WM_DPICHANGED => {
                    push(Event::Display);
                    LRESULT(0)
                }
                // A composição é toda do `UpdateLayeredWindow`; deixar o Windows
                // apagar o fundo só produziria tremulação.
                WM_ERASEBKGND => LRESULT(1),
                // Uma janela `NOACTIVATE` ainda recebe o pedido de ativação por
                // clique; recusá-lo é o que garante que o jogo não perca o foco.
                WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
                _ => DefWindowProcW(hwnd, message, wparam, lparam),
            }
        }
    }

    // --- Utilidades ---

    /// Bounds e DPI do monitor primário — o único que o overlay usa (R11).
    fn primary_monitor() -> (Bounds, u32) {
        // SAFETY: a origem do desktop virtual está sempre no monitor primário.
        let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` vive durante a chamada e declara o próprio tamanho.
        let bounds = if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            let rect = info.rcMonitor;
            Bounds {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            }
        } else {
            log::warn!("GetMonitorInfoW falhou; usando 1920×1080 na origem");
            Bounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }
        };

        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        // SAFETY: monitor válido; os dois destinos são nossos.
        let dpi =
            match unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } {
                Ok(()) if dpi_x != 0 => dpi_x,
                _ => theme::BASE_DPI,
            };
        (bounds, dpi)
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn tick_ms() -> u64 {
        // SAFETY: leitura do relógio monotônico do sistema.
        unsafe { GetTickCount64() }
    }

    fn hiword(value: u32) -> u16 {
        ((value >> 16) & 0xFFFF) as u16
    }

    /// Coordenadas de mouse que vêm no `lParam` (16 bits com sinal).
    fn mouse_point(lparam: LPARAM) -> (i32, i32) {
        let value = lparam.0 as u32;
        (
            (value & 0xFFFF) as u16 as i16 as i32,
            hiword(value) as i16 as i32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: Bounds = Bounds {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    /// Monitor secundário à esquerda do primário: coordenadas negativas são
    /// normais no desktop virtual do Windows.
    const OFFSET: Bounds = Bounds {
        x: -1920,
        y: -200,
        width: 1920,
        height: 1080,
    };

    #[test]
    fn the_hotkey_walks_the_three_states_like_the_v1() {
        // Fechado com HUD persistente ligado: volta para o strip.
        assert_eq!(toggled(OverlayState::Panel, true), OverlayState::Minimal);
        // Sem HUD persistente, some de vez.
        assert_eq!(toggled(OverlayState::Panel, false), OverlayState::Hidden);
        // E de qualquer outro estado o atalho abre o painel.
        assert_eq!(toggled(OverlayState::Hidden, false), OverlayState::Panel);
        assert_eq!(toggled(OverlayState::Minimal, true), OverlayState::Panel);
    }

    #[test]
    fn hidden_collapses_both_windows_into_the_corner() {
        for which in [Which::Strip, Which::Panel] {
            let bounds = window_bounds(OverlayState::Hidden, which, MONITOR, Scale::ONE);
            assert_eq!(bounds.width, 1);
            assert_eq!(bounds.height, 1);
            assert_eq!((bounds.x, bounds.y), (MONITOR.x, MONITOR.y));
        }
        assert!(!is_visible(OverlayState::Hidden));
    }

    #[test]
    fn the_strip_sits_at_the_bottom_center_and_the_panel_in_the_middle() {
        let strip = window_bounds(OverlayState::Minimal, Which::Strip, MONITOR, Scale::ONE);
        assert_eq!(strip.width, STRIP_WIDTH as i32);
        assert_eq!(strip.x, (1920 - STRIP_WIDTH as i32) / 2);
        assert_eq!(strip.y + strip.height, MONITOR.height, "encostado na base");

        let panel = window_bounds(OverlayState::Panel, Which::Panel, MONITOR, Scale::ONE);
        assert_eq!(panel.width, PANEL_WIDTH as i32);
        assert_eq!(panel.x, (1920 - PANEL_WIDTH as i32) / 2);
        assert_eq!(panel.y, (1080 - PANEL_HEIGHT as i32) / 2);
    }

    #[test]
    fn only_the_window_of_the_current_state_has_size() {
        // No strip o painel encolhe, e vice-versa: só um dos dois desenha.
        let panel = window_bounds(OverlayState::Minimal, Which::Panel, MONITOR, Scale::ONE);
        assert_eq!((panel.width, panel.height), (1, 1));

        let strip = window_bounds(OverlayState::Panel, Which::Strip, MONITOR, Scale::ONE);
        assert_eq!((strip.width, strip.height), (1, 1));
    }

    #[test]
    fn the_monitor_origin_and_the_dpi_scale_are_respected() {
        let strip = window_bounds(OverlayState::Minimal, Which::Strip, OFFSET, Scale::ONE);
        assert_eq!(strip.x, OFFSET.x + (1920 - STRIP_WIDTH as i32) / 2);
        assert_eq!(strip.y + strip.height, OFFSET.y + OFFSET.height);

        // 150%: a janela cresce junto com o conteúdo, que continua em DIP.
        let scaled = window_bounds(
            OverlayState::Panel,
            Which::Panel,
            MONITOR,
            Scale::from_dpi(144),
        );
        assert_eq!(scaled.width, (PANEL_WIDTH * 1.5) as i32);
        assert_eq!(scaled.height, (PANEL_HEIGHT * 1.5) as i32);
        assert_eq!(scaled.x, (1920 - scaled.width) / 2);
    }
}
