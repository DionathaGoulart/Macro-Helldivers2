//! Janela principal: classe, `WndProc`, DPI, bounds persistidos e a montagem da
//! tela a partir do estado do app.
//!
//! O modelo de render é o do R14, e a consequência prática está no `WndProc`:
//! ele não tem loop de quadro. Uma passagem de construção acontece quando algo
//! muda — mouse, `UiEvent`, redimensionamento, tique de animação — e o
//! `WM_PAINT` só executa a lista pronta. Parada, a janela não consome CPU.
//!
//! A parte que não depende do Windows (formato e validação dos bounds) fica
//! fora do módulo de plataforma para ser testada no host.

use serde::{Deserialize, Serialize};

use crate::util;

/// Arquivo com a última posição da janela (R9).
pub const BOUNDS_FILE: &str = "window-bounds.json";

/// Classe da janela principal. É por ela que uma segunda instância encontra a
/// primeira (`util::focus_running_instance`).
pub const CLASS_NAME: &str = "MacroHelldivers2Main";

/// Tamanho inicial, em DIP — o mesmo da v1.
pub const DEFAULT_WIDTH: f32 = 820.0;
pub const DEFAULT_HEIGHT: f32 = 640.0;
/// Abaixo disto a grade de 4 colunas não fecha.
pub const MIN_WIDTH: f32 = 720.0;
pub const MIN_HEIGHT: f32 = 520.0;

/// Retângulo de janela em pixels de tela, no mesmo formato que a v1 gravava
/// (`BrowserWindow.getBounds`), para um backup antigo continuar valendo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Bounds {
    /// A janela cabe inteira em alguma área de trabalho? Monitor desligado ou
    /// resolução trocada deixaria a janela abrir fora da tela — a v1 descartava
    /// a posição nesse caso (`legacy/src/main/index.js` ~453-469), e aqui é igual.
    pub fn fits_in(&self, work_areas: &[Bounds]) -> bool {
        self.width > 0
            && self.height > 0
            && work_areas.iter().any(|area| {
                self.x >= area.x
                    && self.y >= area.y
                    && self.x + self.width <= area.x + area.width
                    && self.y + self.height <= area.y + area.height
            })
    }

    /// Lê o arquivo; devolve `None` quando não existe ou está ilegível.
    pub fn load() -> Option<Bounds> {
        let path = util::config_path(BOUNDS_FILE);
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self) {
        let path = util::config_path(BOUNDS_FILE);
        match serde_json::to_vec(self) {
            Ok(json) => {
                if let Err(err) = util::write_atomic(&path, &json) {
                    log::warn!("posição da janela não foi salva: {err:#}");
                }
            }
            Err(err) => log::warn!("posição da janela não serializou: {err}"),
        }
    }
}

#[cfg(windows)]
pub use platform::run;

#[cfg(windows)]
mod platform {
    use std::sync::Arc;

    use anyhow::{Context, Result};
    use crossbeam_channel::Receiver;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, EndPaint, EnumDisplayMonitors,
        GetMonitorInfoW, InvalidateRect, ScreenToClient, SetBkColor, SetTextColor, UpdateWindow,
        CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, FF_DONTCARE, FW_NORMAL, HBRUSH,
        HDC, HFONT, HMONITOR, MONITORINFO, OUT_TT_PRECIS, PAINTSTRUCT, VARIABLE_PITCH,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::SystemInformation::GetTickCount64;
    use windows::Win32::UI::HiDpi::{
        AdjustWindowRectExForDpi, GetDpiForSystem, SetProcessDpiAwarenessContext,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    // `WM_MOUSELEAVE` mora no módulo de controles, não no de janelas.
    use windows::Win32::UI::Controls::WM_MOUSELEAVE;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        ReleaseCapture, SetCapture, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::*;

    use super::{Bounds, DEFAULT_HEIGHT, DEFAULT_WIDTH, MIN_HEIGHT, MIN_WIDTH};
    use crate::data::GameData;
    use crate::gfx::d2d::{window_dpi, WindowTarget};
    use crate::gfx::text::{register_gdi_fonts, Text};
    use crate::meta_stats::{self, MetaResult};
    use crate::settings::{Language, Settings};
    use crate::shared::{
        FlashKind, OverlayCmd, OverlayState, Shared, Slots, UiEvent, UpdateStatus, WM_APP_UI_EVENT,
    };
    use crate::ui::build_tab::{self, BuildTab};
    use crate::ui::macro_tab::{self, Action, MacroTab};
    use crate::ui::modal::{self, Modal};
    use crate::ui::settings_tab::{self, BackupStatus, Change, SettingsTab};
    use crate::ui::theme::{self, font, Color, Scale};
    use crate::ui::toolkit::{id, Id, Input, Measure, Rect, TextStyle, Ui, Weight};
    use crate::ui::widgets::{self, ButtonVariant, Tab, TAB_BAR_HEIGHT};
    use crate::{focus, hooks, i18n, loadouts, overlay, tray, updater, util};

    use tray::Tray;

    /// A mesma classe de [`super::CLASS_NAME`], como literal estático: o
    /// `RegisterClassW` quer um ponteiro que viva por toda a sessão.
    const CLASS: PCWSTR = w!("MacroHelldivers2Main");

    /// Salvamento de posição com atraso: arrastar a janela dispara dezenas de
    /// `WM_MOVE`, e só o último interessa.
    const TIMER_BOUNDS: usize = 1;
    const BOUNDS_DEBOUNCE_MS: u32 = 500;
    /// Só existe enquanto uma animação está correndo (R14).
    const TIMER_ANIM: usize = 2;
    const ANIM_INTERVAL_MS: u32 = 16;
    /// Check de atualização do boot, disparado uma vez só.
    const TIMER_UPDATE: usize = 3;
    /// Fora do caminho crítico do boot, como na v1: rede e disco não competem
    /// com a primeira pintura da janela.
    const UPDATE_CHECK_DELAY_MS: u32 = 1_500;

    /// Pedido de backup adiado, na faixa `WM_APP` reservada em `shared.rs`.
    /// É só desta janela: nenhuma outra thread a envia.
    const WM_APP_BACKUP: u32 = 0x8000 + 3;
    /// Callback do ícone da bandeja: o Windows manda o evento de mouse no
    /// `lParam`.
    const WM_APP_TRAY: u32 = 0x8000 + 4;
    /// Pedido de encerramento adiado (ver [`App::install_update`]).
    const WM_APP_QUIT: u32 = 0x8000 + 5;
    /// Chamada nativa de `EDIT` adiada (ver [`App::defer_edit_op`]).
    const WM_APP_EDIT: u32 = 0x8000 + 6;

    /// Id de controle do primeiro `EDIT` filho; os seguintes vêm em sequência.
    const FIRST_EDIT_CTRL: usize = 1000;

    /// Id do ícone que o `build.rs` embute no executável.
    const ICON_RESOURCE_ID: usize = 1;

    const PAGE_PADDING: f32 = 24.0;
    const FOOTER_HEIGHT: f32 = 40.0;
    const WARNING_HEIGHT: f32 = 56.0;
    /// Botão do updater no rodapé (`px-3 py-1` da v1).
    const FOOTER_BUTTON_H: f32 = 24.0;
    const FOOTER_BUTTON_PADDING: f32 = 24.0;
    /// Espaço entre os pedaços do rodapé.
    const FOOTER_GAP: f32 = 10.0;

    /// Sobe a janela e roda o message loop até o app encerrar.
    pub fn run(shared: Arc<Shared>, data: Arc<GameData>, ui_rx: Receiver<UiEvent>) -> Result<()> {
        enable_dpi_awareness();
        register_gdi_fonts();

        // SAFETY: `None` pede o módulo do próprio processo, que sempre existe.
        let instance = unsafe { GetModuleHandleW(None) }.context("GetModuleHandleW")?;
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            // SAFETY: cursor padrão do sistema.
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.context("LoadCursorW")?,
            hIcon: app_icon(instance.into()),
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: a classe vive durante a chamada e não é registrada duas vezes
        // (o app tem instância única).
        if unsafe { RegisterClassW(&class) } == 0 {
            anyhow::bail!(
                "RegisterClassW falhou: {}",
                windows::core::Error::from_thread()
            );
        }

        let dpi = system_dpi();
        let bounds = restore_bounds(dpi);
        let title = wide(focus::APP_WINDOW_TITLE);
        let boot = Box::new(Boot {
            shared: Arc::clone(&shared),
            data,
            ui_rx,
        });

        // SAFETY: a classe está registrada, o título vive durante a chamada e o
        // parâmetro de criação é reconstituído no `WM_NCCREATE`.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS,
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                None,
                None,
                Some(instance.into()),
                Some(Box::into_raw(boot) as *const _),
            )
        }
        .context("CreateWindowExW")?;

        shared
            .main_hwnd
            .store(hwnd.0 as isize, std::sync::atomic::Ordering::Relaxed);

        // SAFETY: janela recém-criada.
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);
        }

        pump();
        shared
            .main_hwnd
            .store(0, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn pump() {
        let mut msg = MSG::default();
        loop {
            // SAFETY: `msg` é nosso e vive durante a chamada.
            let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
            match result.0 {
                0 => break,
                -1 => {
                    log::error!("GetMessageW falhou na janela principal");
                    break;
                }
                _ => {
                    // SAFETY: mensagem recém-preenchida.
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }
    }

    /// Ícone do executável, embutido como recurso de id 1 pelo `build.rs`.
    ///
    /// Sem o recurso (build sem compilador de recursos) a janela fica com o
    /// ícone padrão do sistema, que é o que o Windows já usaria.
    fn app_icon(instance: windows::Win32::Foundation::HINSTANCE) -> HICON {
        // Um recurso por id viaja no lugar do ponteiro do nome — é o
        // `MAKEINTRESOURCE` do C, e não um endereço que alguém vá desreferenciar.
        let by_id = PCWSTR(std::ptr::without_provenance(ICON_RESOURCE_ID));
        // SAFETY: o módulo é o do próprio processo e o "ponteiro" é um id.
        unsafe { LoadIconW(Some(instance), by_id) }.unwrap_or_default()
    }

    /// Per-monitor v2 antes de qualquer janela: sem isto o Windows esticaria a
    /// janela em telas com escala, e o texto sairia borrado. O manifesto da
    /// Fase 10 declara o mesmo — a chamada aqui vale para `cargo run`.
    fn enable_dpi_awareness() {
        // SAFETY: chamada sem ponteiros; falha só quando o manifesto já definiu
        // a consciência de DPI, que é o mesmo resultado.
        let _ =
            unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    }

    fn system_dpi() -> u32 {
        // SAFETY: leitura de configuração do sistema.
        let dpi = unsafe { GetDpiForSystem() };
        if dpi == 0 {
            theme::BASE_DPI
        } else {
            dpi
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Posição inicial: a última salva, se ainda couber em algum monitor;
    /// senão, o tamanho padrão centralizado pelo Windows.
    fn restore_bounds(dpi: u32) -> Bounds {
        if let Some(saved) = Bounds::load() {
            if saved.fits_in(&work_areas()) {
                return saved;
            }
            log::info!("posição salva fora dos monitores atuais; usando o padrão");
        }

        let scale = Scale::from_dpi(dpi);
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: scale.px(DEFAULT_WIDTH),
            bottom: scale.px(DEFAULT_HEIGHT),
        };
        // A moldura não faz parte do tamanho útil: sem este ajuste a área de
        // cliente sairia menor que os 820×640 do desenho.
        // SAFETY: retângulo próprio, vivo durante a chamada.
        let _ = unsafe {
            AdjustWindowRectExForDpi(
                &mut rect,
                WS_OVERLAPPEDWINDOW,
                false,
                WINDOW_EX_STYLE(0),
                dpi,
            )
        };
        Bounds {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }

    /// Áreas de trabalho de todos os monitores (sem a barra de tarefas).
    fn work_areas() -> Vec<Bounds> {
        let mut areas: Vec<Bounds> = Vec::new();
        // SAFETY: o ponteiro do vetor vive durante a enumeração, que é síncrona.
        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(monitor_proc),
                LPARAM(&mut areas as *mut Vec<Bounds> as isize),
            );
        }
        areas
    }

    unsafe extern "system" fn monitor_proc(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        param: LPARAM,
    ) -> windows::core::BOOL {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` vive durante a chamada; `param` é o vetor passado pelo
        // `EnumDisplayMonitors` logo acima.
        unsafe {
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                let work = info.rcWork;
                let areas = &mut *(param.0 as *mut Vec<Bounds>);
                areas.push(Bounds {
                    x: work.left,
                    y: work.top,
                    width: work.right - work.left,
                    height: work.bottom - work.top,
                });
            }
        }
        true.into()
    }

    /// O que a janela precisa receber antes de existir.
    struct Boot {
        shared: Arc<Shared>,
        data: Arc<GameData>,
        ui_rx: Receiver<UiEvent>,
    }

    struct EditChild {
        id: Id,
        hwnd: HWND,
        visible: bool,
        /// Última posição aplicada: `SetWindowPos` repetido a cada rebuild
        /// custaria mensagens cruzadas por quadro de animação.
        rect: RECT,
        /// Última fonte aplicada: `WM_SETFONT` com redraw repinta o filho
        /// nativo mesmo quando a fonte não mudou.
        font: HFONT,
    }

    /// Chamada nativa sobre um `EDIT`, adiada por mensagem.
    ///
    /// `SetWindowTextW` e `SetFocus` notificam de volta (`EN_CHANGE`,
    /// `EN_SETFOCUS`, `EN_KILLFOCUS`) por `SendMessage` **síncrono**: chamá-las
    /// com um `&mut App` vivo reentra no `WndProc` e cria um segundo `&mut App`
    /// — o mesmo UB que o backup e o menu da bandeja já evitam adiando.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum EditOp {
        /// Esvazia o texto do filho nativo.
        Clear(Id),
        /// Entrega o teclado ao filho nativo.
        Focus(Id),
        /// Devolve o teclado à janela principal.
        FocusMain,
    }

    /// Estado da janela. Vive num `Box` apontado pelo `GWLP_USERDATA`.
    struct App {
        hwnd: HWND,
        shared: Arc<Shared>,
        data: Arc<GameData>,
        ui_rx: Receiver<UiEvent>,
        ui: Ui,
        text: Text,
        target: WindowTarget,
        scale: Scale,
        /// Tamanho da área de cliente, em DIP.
        size: (f32, f32),
        tab: usize,
        language: Language,
        macro_tab: MacroTab,
        build_tab: BuildTab,
        settings_tab: SettingsTab,
        /// Diálogo de backup pedido e ainda não aberto (ver [`App::request_backup`]).
        pending_backup: Option<BackupRequest>,
        /// Chamadas de `EDIT` agendadas e ainda não executadas.
        pending_edit_ops: Vec<EditOp>,
        edits: Vec<EditChild>,
        edit_font: HFONT,
        /// Fonte da escala anterior, viva até os filhos trocarem para a nova.
        retired_font: Option<HFONT>,
        edit_brush: HBRUSH,
        focused_edit: Option<Id>,
        game_focused: bool,
        /// Aba de configurações esperando uma tecla: o hook fica desarmado.
        recording: bool,
        fullscreen_warning: bool,
        tracking_mouse: bool,
        anim_timer: bool,
        /// Andamento do ciclo de atualização, mostrado no rodapé.
        update: UpdateStatus,
        /// `None` quando o ícone não pôde ser criado — e aí fechar a janela
        /// encerra o app, porque não haveria como trazê-la de volta.
        tray: Option<Tray>,
        /// O usuário pediu "Sair": o próximo `WM_CLOSE` encerra de verdade.
        quitting: bool,
    }

    impl App {
        fn new(hwnd: HWND, boot: Boot) -> Result<App> {
            let text = Text::new().context("DirectWrite")?;
            let dpi = window_dpi(hwnd);
            let settings = boot.shared.settings_snapshot();
            let game_focused = boot.shared.is_game_focused();

            let app = App {
                hwnd,
                shared: boot.shared,
                data: boot.data,
                ui_rx: boot.ui_rx,
                ui: Ui::new(),
                text,
                target: WindowTarget::new(hwnd, dpi),
                scale: Scale::from_dpi(dpi),
                size: (DEFAULT_WIDTH, DEFAULT_HEIGHT),
                tab: 0,
                language: settings.language,
                macro_tab: MacroTab::new(),
                build_tab: BuildTab::new(),
                settings_tab: SettingsTab::new(),
                pending_backup: None,
                pending_edit_ops: Vec::new(),
                edits: Vec::new(),
                edit_font: create_edit_font(dpi),
                retired_font: None,
                // SAFETY: cor sólida; o pincel é destruído no `Drop`.
                edit_brush: unsafe {
                    CreateSolidBrush(colorref(theme::SURFACE.over(theme::BG_DEEP)))
                },
                focused_edit: None,
                game_focused,
                recording: false,
                fullscreen_warning: false,
                tracking_mouse: false,
                anim_timer: false,
                update: UpdateStatus::Idle,
                tray: Tray::new(hwnd, WM_APP_TRAY, focus::APP_WINDOW_TITLE),
                quitting: false,
            };

            // SAFETY: timer da própria janela, morto no primeiro tique.
            unsafe {
                SetTimer(Some(hwnd), TIMER_UPDATE, UPDATE_CHECK_DELAY_MS, None);
            }
            Ok(app)
        }

        // --- Estado ---

        /// Drena os avisos das outras threads (`WM_APP_UI_EVENT`).
        fn drain_events(&mut self) {
            let mut changed = false;
            while let Ok(event) = self.ui_rx.try_recv() {
                match event {
                    UiEvent::GameFocus(focused) => {
                        changed |= self.game_focused != focused;
                        self.game_focused = focused;
                    }
                    // Veio do painel do overlay: o valor já está no `Shared`, e
                    // a tela é montada a partir dele.
                    UiEvent::SlotsChanged(_) => changed = true,
                    UiEvent::FullscreenWarning(warning) => {
                        changed |= self.fullscreen_warning != warning;
                        self.fullscreen_warning = warning;
                    }
                    UiEvent::MacroTriggered { slot, support } => {
                        self.flash(slot, support, FlashKind::Triggered);
                        changed = true;
                    }
                    UiEvent::MacroBlocked { slot, support } => {
                        self.flash(slot, support, FlashKind::Blocked);
                        changed = true;
                    }
                    UiEvent::MetaStats(result) => {
                        self.build_tab.set_meta(result, &self.data);
                        changed = true;
                    }
                    UiEvent::UpdateStatus(status) => {
                        changed |= self.update != status;
                        self.update = status;
                    }
                    // O estado do overlay é dele; o andamento de uma sequência
                    // não tem indicador próprio na v1.
                    other => log::debug!("evento de UI sem tela na janela principal: {other:?}"),
                }
            }
            if changed {
                self.rebuild();
            }
        }

        /// Acende a piscada de um slot: amarela no disparo, vermelha quando o
        /// engine recusa por já haver uma sequência rodando.
        fn flash(&mut self, slot: usize, support: bool, kind: FlashKind) {
            let duration = match kind {
                FlashKind::Triggered => widgets::FLASH_TRIGGERED_MS,
                FlashKind::Blocked => widgets::FLASH_BLOCKED_MS,
            };
            // O evento chega com a janela parada, e o relógio do toolkit parou
            // com ela na última pintura.
            self.ui.set_now(tick_ms());
            self.ui
                .flash(widgets::flash_id(slot, support, kind), duration);
        }

        /// Espalha os slots novos: disco, tabela de atalhos e overlay. É o
        /// caminho único de toda alteração vinda da aba de macros.
        fn update_slots(&mut self, slots: Slots) {
            self.shared.set_slots(slots);
            loadouts::save_slots(&slots);
            hooks::rebuild_bindings();
            self.shared.send_overlay(OverlayCmd::Slots(slots));
            self.rebuild();
        }

        // --- Ciclo de desenho ---

        /// Reconstrói a lista, acerta os filhos nativos e marca a janela para
        /// repintura. É o único caminho que leva a um `WM_PAINT`.
        fn rebuild(&mut self) {
            self.build();
            self.start_meta_request();
            self.sync_edits();
            self.sync_anim_timer();
            // SAFETY: janela viva; `None` invalida o cliente inteiro.
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }

        /// Dispara a consulta de estatísticas que a aba de builds registrou.
        ///
        /// A aba não fala com a rede: quem tem o `Shared` (e, portanto, o
        /// caminho de volta do worker) é a janela. Cache fresco responde na
        /// hora, e aí a tela é refeita já com os números.
        fn start_meta_request(&mut self) {
            let Some((faction, difficulty)) = self.build_tab.take_meta_request() else {
                return;
            };
            let Some(stats) = meta_stats::request(&self.shared, faction, difficulty) else {
                // Sem cache, a resposta chega por `UiEvent::MetaStats`.
                return;
            };
            self.build_tab.set_meta(
                MetaResult {
                    key: meta_stats::cache_key(faction, difficulty),
                    stats: Some(stats),
                },
                &self.data,
            );
            self.build();
        }

        fn paint(&mut self) {
            // A lista pode estar vazia se o primeiro `WM_PAINT` chegar antes do
            // primeiro `WM_SIZE`; construir aqui evita o quadro em branco.
            if self.ui.frame().nodes.is_empty() {
                self.build();
            }
            self.target
                .draw(&mut self.text, self.ui.frame(), theme::BG_DEEP);
        }

        fn sync_anim_timer(&mut self) {
            // `WM_TIMER` dispara mesmo com a janela escondida ou minimizada, e
            // cada tique refaz o quadro inteiro. Um modal pulsando com o app na
            // bandeja viraria um rebuild completo a cada 16ms, por horas, no
            // meio do jogo — invisível para todo mundo. Escondida, a animação
            // congela; o `WM_SHOWWINDOW`/`WM_SIZE` da volta refaz o quadro e
            // rearma o timer.
            // SAFETY: leituras de estado da própria janela.
            let visible =
                unsafe { IsWindowVisible(self.hwnd).as_bool() && !IsIconic(self.hwnd).as_bool() };
            let wanted = self.ui.animating() && visible;
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

        fn on_resize(&mut self, width_px: u32, height_px: u32) {
            self.target.resize(width_px, height_px);
            self.size = (
                self.scale.dip(width_px as f32),
                self.scale.dip(height_px as f32),
            );
            self.rebuild();
        }

        fn on_dpi_changed(&mut self, dpi: u32) {
            self.scale = Scale::from_dpi(dpi);
            self.target.set_dpi(dpi);
            // A fonte antiga ainda está selecionada nos `EDIT`; ela só é
            // destruída depois que eles receberem a nova, em `sync_edits`.
            self.retire_font(create_edit_font(dpi));
            self.text.clear_cache();
        }

        // --- Entrada ---

        fn on_mouse(&mut self, event: Input) {
            let response = self.ui.input(event);
            if let Some(clicked) = response.clicked {
                self.on_click(clicked);
            } else if response.redraw {
                self.rebuild();
            }
        }

        fn on_click(&mut self, clicked: Id) {
            if self.on_update_click(clicked) {
                return;
            }
            if let Some(index) = (0..3).find(|index| widgets::tab_id(*index) == clicked) {
                if self.tab != index {
                    self.tab = index;
                    // Sair da aba desiste da captura em curso: o hook não pode
                    // ficar desarmado por uma tela que não está mais na frente.
                    self.settings_tab.cancel_capture();
                    self.sync_recording();
                }
                self.rebuild();
                return;
            }
            match self.tab {
                0 => {
                    let settings = self.shared.settings_snapshot();
                    // O contexto é montado campo a campo: `macro_tab` precisa ser
                    // emprestado mutável ao mesmo tempo que `data` é lido.
                    let ctx = macro_tab::Ctx {
                        data: &self.data,
                        settings: &settings,
                        slots: self.shared.slots(),
                        focused_edit: self.focused_edit,
                    };
                    if let Some(action) = self.macro_tab.on_click(clicked, &ctx) {
                        self.apply(action);
                        return;
                    }
                }
                1 => {
                    let settings = self.shared.settings_snapshot();
                    let ctx = build_tab::Ctx {
                        data: &self.data,
                        settings: &settings,
                        slots: self.shared.slots(),
                        focused_edit: self.focused_edit,
                    };
                    if let Some(action) = self.build_tab.on_click(clicked, &ctx) {
                        self.apply_build(action);
                        return;
                    }
                }
                2 => {
                    let settings = self.shared.settings_snapshot();
                    let ctx = settings_tab::Ctx {
                        settings: &settings,
                    };
                    if let Some(action) = self.settings_tab.on_click(clicked, &ctx) {
                        self.apply_settings(action);
                        return;
                    }
                }
                _ => {}
            }
            self.rebuild();
        }

        /// Cliques do updater: o botão do rodapé e os do modal. `true` quando o
        /// clique era de lá e as abas não devem vê-lo.
        fn on_update_click(&mut self, clicked: Id) -> bool {
            if clicked == update_action_id() || clicked == modal::primary_id() {
                match &self.update {
                    UpdateStatus::Available { .. } => updater::download(&self.shared),
                    UpdateStatus::Ready { .. } => self.install_update(),
                    // O botão só existe nesses dois estados; um clique que
                    // chegue aqui é de um quadro que já mudou.
                    _ => {}
                }
                self.rebuild();
                return true;
            }
            if clicked == modal::secondary_id() {
                // "Depois": o modal sai e o rodapé volta ao texto de repouso,
                // como na v1. O instalador continua no disco para o próximo
                // check encontrar.
                self.update = UpdateStatus::Idle;
                self.rebuild();
                return true;
            }
            // O véu come o clique sem fazer nada — é o que impede a tela de
            // trás de reagir com o modal aberto.
            if clicked == modal::scrim_id() {
                return true;
            }
            false
        }

        /// Executa o instalador baixado e encerra o app.
        ///
        /// O encerramento é adiado por mensagem: `DestroyWindow` libera o
        /// próprio `App` de dentro dela, e este método roda com um `&mut App`
        /// vivo (o clique que veio do `WndProc`).
        fn install_update(&mut self) {
            if let Err(err) = updater::install() {
                log::error!("instalador não pôde ser executado: {err:#}");
                self.update = UpdateStatus::Error {
                    message: err.to_string(),
                };
                return;
            }
            // SAFETY: `PostMessageW` é assíncrono; a mensagem cai na fila desta
            // própria janela.
            unsafe {
                let _ = PostMessageW(Some(self.hwnd), WM_APP_QUIT, WPARAM(0), LPARAM(0));
            }
        }

        fn apply(&mut self, action: Action) {
            match action {
                Action::Redraw => self.rebuild(),
                Action::SlotsChanged(slots) => self.update_slots(slots),
                Action::FocusSearch => self.focus_edit(macro_tab::search_id()),
                Action::ClearSearch => self.clear_edit(macro_tab::search_id()),
            }
        }

        fn apply_build(&mut self, action: build_tab::Action) {
            match action {
                build_tab::Action::Redraw => self.rebuild(),
                build_tab::Action::Setting(change) => self.apply_change(change),
                build_tab::Action::SlotsChanged(slots) => self.update_slots(slots),
                build_tab::Action::LoadoutsChanged => self.save_loadouts(),
                build_tab::Action::Saved => {
                    self.save_loadouts();
                    // O campo de nome é esvaziado depois de salvar, como na v1.
                    self.clear_edit(build_tab::name_id());
                }
                build_tab::Action::FocusEdit(id) => self.focus_edit(id),
                build_tab::Action::ClearEdit(id) => self.clear_edit(id),
            }
        }

        /// Grava as builds salvas e avisa o overlay, que também as mostra.
        fn save_loadouts(&mut self) {
            loadouts::save_loadouts(self.build_tab.loadouts());
            self.shared.send_overlay(OverlayCmd::LoadoutsChanged);
            self.rebuild();
        }

        fn apply_settings(&mut self, action: settings_tab::Action) {
            self.sync_recording();
            match action {
                settings_tab::Action::Redraw => self.rebuild(),
                settings_tab::Action::Setting(change) => self.apply_change(change),
                settings_tab::Action::ExportBackup => self.request_backup(BackupRequest::Export),
                settings_tab::Action::ImportBackup => self.request_backup(BackupRequest::Import),
            }
        }

        // --- Backup ---

        /// Agenda o diálogo em vez de abri-lo aqui.
        ///
        /// Um diálogo do shell roda o **próprio** loop de mensagens, que reentra
        /// neste `WndProc` — e neste ponto o empréstimo do `App` está vivo, no
        /// meio do tratamento do clique. O pedido fica guardado e é executado
        /// quando a mensagem chegar, com o empréstimo já solto.
        fn request_backup(&mut self, request: BackupRequest) {
            self.pending_backup = Some(request);
            // SAFETY: `PostMessageW` é assíncrono; a mensagem cai na fila desta
            // própria janela.
            unsafe {
                let _ = PostMessageW(Some(self.hwnd), WM_APP_BACKUP, WPARAM(0), LPARAM(0));
            }
            self.rebuild();
        }

        /// Aplica o que o diálogo deixou pronto.
        fn finish_backup(&mut self, outcome: BackupOutcome) {
            if outcome.imported {
                self.language = self.shared.settings_snapshot().language;
                // O arquivo pode ter trazido builds salvas, gravadas por fora da
                // aba: ela precisa reler o que está no disco agora.
                self.build_tab.reload_loadouts();
                match outcome.slots {
                    // `update_slots` grava, refaz a tabela de atalhos e avisa o
                    // overlay; sem slots no arquivo, a tabela ainda precisa dos
                    // atalhos que vieram nas preferências.
                    Some(slots) => self.update_slots(slots),
                    None => hooks::rebuild_bindings(),
                }
            }
            if let Some(status) = outcome.status {
                self.settings_tab.set_backup_status(status);
                // O diálogo segurou a janela parada, e o relógio do toolkit
                // parou com ela: sem acertá-lo o aviso nasceria vencido.
                self.ui.set_now(tick_ms());
                self.ui.flash(
                    settings_tab::backup_flash_id(),
                    settings_tab::BACKUP_STATUS_MS,
                );
            }
            self.rebuild();
        }

        /// Grava a preferência nova e espalha o que ela afeta: tabela de
        /// atalhos, overlay e idioma da interface.
        fn apply_change(&mut self, change: Change) {
            let previous = self.shared.settings_snapshot();
            let mut settings = previous.clone();
            change.apply(&mut settings);
            if settings == previous {
                self.rebuild();
                return;
            }

            self.shared.set_settings(settings.clone());
            if let Err(err) = settings.save() {
                log::warn!("configurações não foram salvas: {err:#}");
            }
            hooks::rebuild_bindings();
            self.language = settings.language;
            overlay_effects(&self.shared, self.game_focused, &previous, &settings);
            self.rebuild();
        }

        /// Liga o modo de gravação enquanto a aba espera uma tecla: com ele o
        /// hook repassa tudo em vez de disparar macros.
        fn sync_recording(&mut self) {
            let capturing = self.settings_tab.capturing().is_some();
            if capturing == self.recording {
                return;
            }
            self.recording = capturing;
            self.shared.set_recording(capturing);
            if capturing {
                // O `EDIT` da busca pode estar com o teclado; sem trazer o foco
                // de volta, a tecla capturada nunca chegaria ao `WndProc`. A
                // chamada é adiada porque o `EN_KILLFOCUS` dela reentra aqui.
                self.defer_edit_op(EditOp::FocusMain);
            }
        }

        /// Tecla recebida pela janela. `true` quando a aba da frente a consumiu,
        /// e ela não deve seguir para o tratamento padrão.
        fn on_key(&mut self, vk: u16) -> bool {
            match self.tab {
                1 => match self.build_tab.on_key(vk) {
                    Some(action) => {
                        self.apply_build(action);
                        true
                    }
                    None => false,
                },
                2 => match self.settings_tab.on_key(vk) {
                    Some(action) => {
                        self.apply_settings(action);
                        true
                    }
                    None => false,
                },
                _ => false,
            }
        }

        /// Um `EDIT` de busca só existe quando tem foco ou texto; o clique na
        /// moldura é o que o traz à tona.
        fn focus_edit(&mut self, id: Id) {
            self.focused_edit = Some(id);
            self.rebuild();
            self.defer_edit_op(EditOp::Focus(id));
        }

        /// Esvazia um campo de busca: o estado da aba agora, o filho nativo na
        /// mensagem adiada. Quando o `EN_CHANGE` da escrita chega, a aba já
        /// está com o mesmo texto vazio e o aviso não muda nada.
        fn clear_edit(&mut self, id: Id) {
            self.defer_edit_op(EditOp::Clear(id));
            self.set_edit_text(id, String::new());
            self.rebuild();
        }

        /// Agenda uma chamada nativa de `EDIT` para rodar sem `&mut App` vivo
        /// (ver [`EditOp`]); quem executa é o `WM_APP_EDIT`.
        fn defer_edit_op(&mut self, op: EditOp) {
            self.pending_edit_ops.push(op);
            // SAFETY: `PostMessageW` é assíncrono; a mensagem cai na fila desta
            // própria janela.
            unsafe {
                let _ = PostMessageW(Some(self.hwnd), WM_APP_EDIT, WPARAM(0), LPARAM(0));
            }
        }

        /// Entrega o texto novo à aba dona do campo.
        fn set_edit_text(&mut self, id: Id, text: String) {
            if id == macro_tab::search_id() {
                self.macro_tab.set_search(text);
            } else if id == build_tab::search_id() {
                self.build_tab.set_search(text);
            } else if id == build_tab::name_id() {
                self.build_tab.set_name(text);
            }
        }

        // --- Filhos nativos ---

        /// Troca a fonte dos campos, guardando a anterior para destruir depois.
        fn retire_font(&mut self, font: HFONT) {
            if let Some(previous) = self.retired_font.replace(self.edit_font) {
                // SAFETY: essa já passou por um ciclo inteiro de `sync_edits`,
                // então nenhum controle ainda a usa.
                unsafe {
                    let _ = DeleteObject(previous.into());
                }
            }
            self.edit_font = font;
        }

        /// Cria, move e esconde os `EDIT` conforme a lista de desenho.
        fn sync_edits(&mut self) {
            let hosts = self.ui.frame().edits().to_vec();

            for host in &hosts {
                let rect = self.px_rect(host.rect);
                let index = match self.edits.iter().position(|edit| edit.id == host.id) {
                    Some(index) => index,
                    None => {
                        let Some(child) = self.create_edit(host.id) else {
                            continue;
                        };
                        self.edits.push(child);
                        self.edits.len() - 1
                    }
                };
                let child = &mut self.edits[index];
                // Só o que mudou: durante uma animação isto roda a cada quadro,
                // e mensagem repetida para o filho nativo é repintura à toa.
                // SAFETY: filho vivo; posição em pixels de cliente.
                unsafe {
                    if child.rect != rect {
                        let _ = SetWindowPos(
                            child.hwnd,
                            None,
                            rect.left,
                            rect.top,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                            SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                        child.rect = rect;
                    }
                    if !child.visible {
                        let _ = ShowWindow(child.hwnd, SW_SHOW);
                    }
                    if child.font != self.edit_font {
                        SendMessageW(
                            child.hwnd,
                            WM_SETFONT,
                            Some(WPARAM(self.edit_font.0 as usize)),
                            Some(LPARAM(1)),
                        );
                        child.font = self.edit_font;
                    }
                }
                child.visible = true;
            }

            let mut hidden_had_focus = false;
            for child in &mut self.edits {
                if hosts.iter().any(|host| host.id == child.id) {
                    continue;
                }
                if child.visible {
                    // SAFETY: filho vivo.
                    unsafe {
                        let _ = ShowWindow(child.hwnd, SW_HIDE);
                    }
                    child.visible = false;
                    hidden_had_focus |= self.focused_edit == Some(child.id);
                }
            }
            if hidden_had_focus {
                // Esconder não move o teclado: sem isto o `EDIT` invisível
                // continuaria comendo as teclas (a busca "fantasma" mudaria e o
                // Esc nunca chegaria à janela). O `EN_KILLFOCUS` da chamada
                // adiada é quem limpa o `focused_edit`.
                self.defer_edit_op(EditOp::FocusMain);
            }

            if let Some(retired) = self.retired_font.take() {
                // SAFETY: todos os campos acabaram de receber a fonte nova.
                unsafe {
                    let _ = DeleteObject(retired.into());
                }
            }
        }

        fn create_edit(&self, id: Id) -> Option<EditChild> {
            let ctrl = FIRST_EDIT_CTRL + self.edits.len();
            // SAFETY: classe nativa `EDIT`; o filho morre com a janela pai.
            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("EDIT"),
                    PCWSTR::null(),
                    WS_CHILD | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                    0,
                    0,
                    0,
                    0,
                    Some(self.hwnd),
                    Some(HMENU(ctrl as *mut _)),
                    None,
                    None,
                )
            };
            match hwnd {
                Ok(hwnd) => Some(EditChild {
                    id,
                    hwnd,
                    visible: false,
                    rect: RECT::default(),
                    font: HFONT::default(),
                }),
                Err(err) => {
                    log::warn!("campo de texto não pôde ser criado: {err}");
                    None
                }
            }
        }

        fn edit_text(&self, hwnd: HWND) -> String {
            // SAFETY: janela viva; o buffer tem o tamanho declarado.
            let length = unsafe { GetWindowTextLengthW(hwnd) };
            if length <= 0 {
                return String::new();
            }
            let mut buffer = vec![0u16; length as usize + 1];
            // SAFETY: buffer do tamanho informado.
            let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
            String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
        }

        fn on_edit_changed(&mut self, ctrl_hwnd: HWND) {
            let Some(id) = self
                .edits
                .iter()
                .find(|edit| edit.hwnd == ctrl_hwnd)
                .map(|edit| edit.id)
            else {
                return;
            };
            let text = self.edit_text(ctrl_hwnd);
            self.set_edit_text(id, text);
            self.rebuild();
        }

        fn px_rect(&self, rect: Rect) -> RECT {
            RECT {
                left: self.scale.px(rect.x),
                top: self.scale.px(rect.y),
                right: self.scale.px(rect.right()),
                bottom: self.scale.px(rect.bottom()),
            }
        }

        fn dip(&self, x: i32, y: i32) -> (f32, f32) {
            (self.scale.dip(x as f32), self.scale.dip(y as f32))
        }
    }

    impl Drop for App {
        fn drop(&mut self) {
            // SAFETY: objetos GDI criados por esta janela, soltos uma vez só.
            unsafe {
                if let Some(retired) = self.retired_font.take() {
                    let _ = DeleteObject(retired.into());
                }
                let _ = DeleteObject(self.edit_font.into());
                let _ = DeleteObject(self.edit_brush.into());
            }
        }
    }

    fn create_edit_font(dpi: u32) -> HFONT {
        let height = -(font::SIZE_BODY * dpi as f32 / theme::BASE_DPI as f32).round() as i32;
        let face = wide(font::FAMILY);
        // SAFETY: o nome da família vive durante a chamada; a fonte é destruída
        // no `Drop` do `App`.
        unsafe {
            CreateFontW(
                height,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                (VARIABLE_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(face.as_ptr()),
            )
        }
    }

    fn colorref(color: Color) -> COLORREF {
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
        COLORREF(channel(color.r) | channel(color.g) << 8 | channel(color.b) << 16)
    }

    fn tick_ms() -> u64 {
        // SAFETY: leitura do relógio monotônico do sistema.
        unsafe { GetTickCount64() }
    }

    fn loword(value: u32) -> u16 {
        (value & 0xFFFF) as u16
    }

    fn hiword(value: u32) -> u16 {
        ((value >> 16) & 0xFFFF) as u16
    }

    /// Coordenadas de mouse que vêm no `lParam` (16 bits com sinal).
    fn mouse_point(lparam: LPARAM) -> (i32, i32) {
        let value = lparam.0 as u32;
        (loword(value) as i16 as i32, hiword(value) as i16 as i32)
    }

    // --- Construção da tela ---

    impl App {
        fn build(&mut self) {
            let now = tick_ms();
            self.ui.begin(now);
            let settings = self.shared.settings_snapshot();

            let mut body = Rect::new(0.0, 0.0, self.size.0, self.size.1);
            let header = body.cut_top(TAB_BAR_HEIGHT);
            let footer = body.cut_bottom(FOOTER_HEIGHT);

            let tr = i18n::tr(self.language);
            let tabs = [
                Tab {
                    label: tr.tabs.macro_tab,
                    accent: theme::CYAN,
                },
                Tab {
                    label: tr.tabs.build,
                    accent: theme::CYAN,
                },
                Tab {
                    label: tr.tabs.settings,
                    accent: theme::YELLOW,
                },
            ];
            widgets::tab_bar(&mut self.ui, &mut self.text, header, &tabs, self.tab);

            if self.fullscreen_warning && self.game_focused {
                let banner = body.cut_top(WARNING_HEIGHT);
                self.warning(banner, tr.overlay.fullscreen_warning);
            }

            match self.tab {
                0 => {
                    let ctx = macro_tab::Ctx {
                        data: &self.data,
                        settings: &settings,
                        slots: self.shared.slots(),
                        focused_edit: self.focused_edit,
                    };
                    self.macro_tab
                        .build(&mut self.ui, &mut self.text, body, &ctx);
                }
                1 => {
                    let ctx = build_tab::Ctx {
                        data: &self.data,
                        settings: &settings,
                        slots: self.shared.slots(),
                        focused_edit: self.focused_edit,
                    };
                    self.build_tab
                        .build(&mut self.ui, &mut self.text, body, &ctx);
                }
                _ => {
                    let ctx = settings_tab::Ctx {
                        settings: &settings,
                    };
                    self.settings_tab
                        .build(&mut self.ui, &mut self.text, body, &ctx);
                }
            }
            self.footer(footer);

            // Por último, sobre tudo: enquanto o modal está aberto, o véu é
            // quem responde a qualquer clique fora do cartão.
            if let UpdateStatus::Ready { version } = self.update.clone() {
                let area = Rect::new(0.0, 0.0, self.size.0, self.size.1);
                self.update_modal(area, &version);
            }
            self.ui.end();
        }

        fn warning(&mut self, rect: Rect, message: &str) {
            let rect = rect.inset_xy(PAGE_PADDING, 6.0);
            self.ui
                .fill(rect, theme::RADIUS_BUTTON, theme::YELLOW.alpha(0.12));
            self.ui.stroke(
                rect,
                theme::RADIUS_BUTTON,
                theme::HAIRLINE_WIDTH,
                theme::YELLOW.alpha(0.5),
            );
            self.ui.text(
                rect.inset_xy(16.0, 8.0),
                message,
                TextStyle::new(font::SIZE_TINY, Weight::Regular).wrap(),
                theme::YELLOW,
            );
        }

        /// Rodapé: versão e estado do jogo à esquerda, updater à direita — o
        /// mesmo agrupamento do rodapé da v1.
        fn footer(&mut self, rect: Rect) {
            self.ui.fill(rect, 0.0, theme::BG_DEEP);
            self.ui
                .fill(rect.with_h(theme::HAIRLINE_WIDTH), 0.0, theme::HAIRLINE);

            let mut row = rect.inset_xy(PAGE_PADDING, 0.0);
            self.update_footer(&mut row);
            self.status_footer(&mut row);
        }

        /// Versão do app e a bolinha de "jogo detectado".
        fn status_footer(&mut self, row: &mut Rect) {
            let tr = i18n::tr(self.language);
            let style =
                TextStyle::new(font::SIZE_TINY, Weight::Black).tracking(font::TRACKING_LABEL);

            let version = format!(
                "{} v{}",
                tr.settings.version.to_uppercase(),
                env!("CARGO_PKG_VERSION")
            );
            let width = self.text.text_size(&version, style, f32::INFINITY).0;
            let label = row.cut_left(width).middle_row(16.0);
            self.ui.text(label, version, style, theme::TEXT_DIM);
            row.cut_left(FOOTER_GAP);

            let divider = row.cut_left(theme::HAIRLINE_WIDTH).middle_row(12.0);
            self.ui.fill(divider, 0.0, theme::BORDER);
            row.cut_left(FOOTER_GAP);

            let dot = row.cut_left(8.0).middle_row(8.0);
            widgets::status_dot(&mut self.ui, dot, self.game_focused);
            row.cut_left(6.0);

            let status = if self.game_focused {
                tr.settings.game_active
            } else {
                tr.settings.game_inactive
            }
            .to_uppercase();
            let width = self.text.text_size(&status, style, f32::INFINITY).0;
            let label = row.cut_left(width).middle_row(16.0);
            self.ui.text(
                label,
                status,
                style,
                if self.game_focused {
                    theme::GREEN
                } else {
                    theme::TEXT_DIM
                },
            );
        }

        /// Canto direito do rodapé: o andamento do updater e, quando há o que
        /// fazer, o botão que baixa ou instala. Montado da direita para a
        /// esquerda, então o botão vem antes do texto.
        fn update_footer(&mut self, row: &mut Rect) {
            let tr = i18n::tr(self.language);

            // Pronto para instalar: só o botão, pulsando como na v1.
            if let UpdateStatus::Ready { .. } = self.update {
                let rect = row
                    .cut_right(self.button_width(tr.settings.update_ready))
                    .middle_row(FOOTER_BUTTON_H);
                widgets::button(
                    &mut self.ui,
                    update_action_id(),
                    rect,
                    tr.settings.update_ready,
                    ButtonVariant::Primary,
                    theme::YELLOW,
                );
                return;
            }

            // O download deixou de ser automático para não puxar o instalador
            // no meio de uma partida: quem manda é o botão.
            if let UpdateStatus::Available { .. } = self.update {
                let rect = row
                    .cut_right(self.button_width(tr.settings.update_download))
                    .middle_row(FOOTER_BUTTON_H);
                widgets::button(
                    &mut self.ui,
                    update_action_id(),
                    rect,
                    tr.settings.update_download,
                    ButtonVariant::Secondary,
                    theme::YELLOW,
                );
                row.cut_right(FOOTER_GAP);
            }

            let style =
                TextStyle::new(font::SIZE_TINY, Weight::Black).tracking(font::TRACKING_LABEL);
            let text = update_label(&self.update, tr);
            let width = self.text.text_size(&text, style, f32::INFINITY).0;
            let label = row.cut_right(width).middle_row(16.0);
            self.ui.text(label, text, style, theme::TEXT_DIM);
            row.cut_right(6.0);

            let color = match self.update {
                UpdateStatus::Error { .. } => theme::RED,
                _ => theme::YELLOW,
            };
            let dot = row.cut_right(6.0).middle_row(6.0);
            self.ui.ellipse(dot, color);
            self.ui.glow(dot, dot.w / 2.0, color);
        }

        fn button_width(&mut self, label: &str) -> f32 {
            let style =
                TextStyle::new(font::SIZE_LABEL, Weight::Black).tracking(font::TRACKING_WIDE);
            let text = self
                .text
                .text_size(&label.to_uppercase(), style, f32::INFINITY)
                .0;
            text + FOOTER_BUTTON_PADDING
        }

        /// Modal de "atualização baixada" (porte do modal da v1).
        fn update_modal(&mut self, area: Rect, version: &str) {
            let tr = i18n::tr(self.language);
            let subtitle = format!("v{version}");
            let modal = Modal {
                title: tr.update.title,
                subtitle: &subtitle,
                body: tr.update.body,
                primary: tr.update.restart_now,
                secondary: tr.update.later,
                accent: theme::YELLOW,
            };
            modal::show(&mut self.ui, &mut self.text, area, &modal);
        }
    }

    /// Botão do updater no rodapé: baixa quando há novidade, instala quando o
    /// arquivo já está no disco.
    fn update_action_id() -> Id {
        id("footer.update")
    }

    /// Texto do andamento, com os mesmos estados da v1.
    fn update_label(status: &UpdateStatus, tr: &i18n::Tr) -> String {
        let text = &tr.settings;
        match status {
            UpdateStatus::Checking => text.update_checking.to_uppercase(),
            UpdateStatus::Available { .. } => text.update_available.to_uppercase(),
            UpdateStatus::Downloading { percent } => format!(
                "{} {}%",
                text.update_downloading.to_uppercase(),
                percent.round()
            ),
            UpdateStatus::UpToDate => text.update_up_to_date.to_uppercase(),
            UpdateStatus::Error { .. } => text.update_error.to_uppercase(),
            // "Atualizado" é o texto de repouso da v1, e também o que sobra
            // depois de o usuário adiar a instalação.
            UpdateStatus::Idle | UpdateStatus::Ready { .. } => text.updated.to_uppercase(),
        }
    }

    // --- Backup (fora do `App`) ---

    /// Qual diálogo o usuário pediu.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum BackupRequest {
        Export,
        Import,
    }

    /// O que sobra para a janela fazer depois que o diálogo fecha.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct BackupOutcome {
        /// Aviso a mostrar; `None` quando o usuário apenas cancelou.
        status: Option<BackupStatus>,
        /// Slots restaurados, quando a importação trouxe a lista.
        slots: Option<Slots>,
        /// Uma importação mexeu nas preferências.
        imported: bool,
    }

    /// Escreve o backup no arquivo escolhido. Cancelar não é falha e não vira
    /// aviso — a v1 separava os dois casos do mesmo jeito.
    fn export_backup(shared: &Shared, language: Language) -> BackupOutcome {
        let title = i18n::tr(language).settings.backup_export;
        let write = || -> Result<bool> {
            let Some(path) =
                util::save_dialog(title, loadouts::BACKUP_FILE_NAME, util::JSON_FILTER)?
            else {
                return Ok(false);
            };
            loadouts::Backup::new(
                &shared.settings_snapshot(),
                &loadouts::load_loadouts(),
                shared.slots(),
            )?
            .write(&path)?;
            Ok(true)
        };

        let status = match write() {
            Ok(true) => Some(BackupStatus::Exported),
            Ok(false) => None,
            Err(err) => {
                log::warn!("backup não foi exportado: {err:#}");
                Some(BackupStatus::Failed)
            }
        };
        BackupOutcome {
            status,
            ..BackupOutcome::default()
        }
    }

    /// Restaura o que o arquivo trouxer — o que ele não trouxer fica como está
    /// (porte de `legacy/src/renderer/App.jsx` ~199–241).
    fn import_backup(
        shared: &Shared,
        data: &GameData,
        game_focused: bool,
        language: Language,
    ) -> BackupOutcome {
        let title = i18n::tr(language).settings.backup_import;
        let read = || -> Result<Option<Option<Slots>>> {
            let Some(path) = util::open_dialog(title, util::JSON_FILTER)? else {
                return Ok(None);
            };
            let backup = loadouts::Backup::read(&path)?;

            let previous = shared.settings_snapshot();
            let settings = backup.merged_settings(&previous)?;
            shared.set_settings(settings.clone());
            if let Err(err) = settings.save() {
                log::warn!("configurações do backup não foram salvas: {err:#}");
            }
            overlay_effects(shared, game_focused, &previous, &settings);

            if let Some(saved) = &backup.loadouts {
                loadouts::save_loadouts(saved);
                shared.send_overlay(OverlayCmd::LoadoutsChanged);
            }
            Ok(Some(backup.slots(data)))
        };

        match read() {
            Ok(Some(slots)) => BackupOutcome {
                status: Some(BackupStatus::Imported),
                slots,
                imported: true,
            },
            Ok(None) => BackupOutcome::default(),
            Err(err) => {
                log::warn!("backup não foi importado: {err:#}");
                BackupOutcome {
                    status: Some(BackupStatus::Failed),
                    ..BackupOutcome::default()
                }
            }
        }
    }

    /// O que uma mudança de preferência faz com o overlay (portado de
    /// `legacy/src/main/index.js` ~737–746).
    ///
    /// Desligar o recurso derruba a thread inteira, com as duas janelas: é o que
    /// zera de verdade o custo do overlay, e não só o que ele desenha.
    fn overlay_effects(
        shared: &Shared,
        game_focused: bool,
        previous: &Settings,
        settings: &Settings,
    ) {
        if settings.enable_overlay != previous.enable_overlay {
            overlay::set_enabled(settings.enable_overlay);
        }
        if !settings.enable_overlay {
            return;
        }
        // Atalhos e idioma aparecem no strip e no painel.
        shared.send_overlay(OverlayCmd::SettingsChanged);

        // Com o jogo na frente, o HUD persistente aparece e some na hora —
        // menos com o painel aberto, que manda no estado.
        let hud_changed =
            settings.always_show_slots != previous.always_show_slots || !previous.enable_overlay;
        if hud_changed && game_focused && shared.overlay_state() != OverlayState::Panel {
            let state = if settings.always_show_slots {
                OverlayState::Minimal
            } else {
                OverlayState::Hidden
            };
            shared.send_overlay(OverlayCmd::SetState(state));
        }
    }

    /// Executa as chamadas nativas de `EDIT` agendadas, **sem** empréstimo do
    /// `App` vivo: o `EN_*` que elas provocam chega por `SendMessage` síncrono
    /// e reentra no `WndProc`.
    fn run_pending_edit_ops(hwnd: HWND) {
        enum Native {
            SetText(HWND),
            Focus(HWND),
        }

        // SAFETY: empréstimo curto — tira os pedidos e resolve os alvos; solto
        // antes das chamadas que reentram.
        let actions: Vec<Native> = {
            let Some(app) = (unsafe { app_mut(hwnd) }) else {
                return;
            };
            let ops = std::mem::take(&mut app.pending_edit_ops);
            ops.into_iter()
                .filter_map(|op| {
                    let child = |id| {
                        app.edits
                            .iter()
                            .find(|edit| edit.id == id)
                            .map(|edit| edit.hwnd)
                    };
                    match op {
                        EditOp::Clear(id) => child(id).map(Native::SetText),
                        EditOp::Focus(id) => child(id).map(Native::Focus),
                        EditOp::FocusMain => Some(Native::Focus(hwnd)),
                    }
                })
                .collect()
        };

        for action in actions {
            // SAFETY: janelas vivas (os filhos morrem com a janela pai), e
            // nenhuma referência ao `App` está viva quando o `EN_*` reentra.
            unsafe {
                match action {
                    Native::SetText(child) => {
                        let _ = SetWindowTextW(child, w!(""));
                    }
                    Native::Focus(target) => {
                        let _ = SetFocus(Some(target));
                    }
                }
            }
        }
    }

    /// Abre o diálogo pedido **sem** nenhum empréstimo do `App` vivo: o loop de
    /// mensagens do próprio diálogo reentra no `WndProc`, e dois `&mut App` ao
    /// mesmo tempo seriam UB.
    fn run_pending_backup(hwnd: HWND) {
        // SAFETY: empréstimo curto, só para tirar o pedido e copiar o que o
        // diálogo precisa; solto antes de qualquer chamada bloqueante.
        let taken = unsafe {
            app_mut(hwnd).and_then(|app| {
                app.pending_backup.take().map(|request| {
                    (
                        request,
                        Arc::clone(&app.shared),
                        Arc::clone(&app.data),
                        app.game_focused,
                        app.language,
                    )
                })
            })
        };
        let Some((request, shared, data, game_focused, language)) = taken else {
            return;
        };

        let outcome = match request {
            BackupRequest::Export => export_backup(&shared, language),
            BackupRequest::Import => import_backup(&shared, &data, game_focused, language),
        };

        // SAFETY: o diálogo já fechou; nenhum outro empréstimo está vivo.
        if let Some(app) = unsafe { app_mut(hwnd) } {
            app.finish_backup(outcome);
        }
    }

    // --- Bandeja (fora do `App`) ---

    /// Traz a janela de volta: da bandeja, da barra de tarefas ou de trás de
    /// outra janela.
    fn restore_window(hwnd: HWND) {
        // SAFETY: janela viva; no pior caso as chamadas devolvem erro.
        unsafe {
            let command = if IsIconic(hwnd).as_bool() {
                SW_RESTORE
            } else {
                SW_SHOW
            };
            let _ = ShowWindow(hwnd, command);
            let _ = SetForegroundWindow(hwnd);
        }
    }

    /// Recolhe para a bandeja. O processo continua de pé com os hooks e o
    /// motor, que é o ponto do app.
    fn hide_window(hwnd: HWND) {
        // A posição é gravada antes de esconder: escondida, a janela reporta
        // coordenadas que não servem para a próxima abertura.
        save_bounds(hwnd);
        // SAFETY: janela viva.
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }

    /// Abre o menu do ícone e executa a escolha.
    ///
    /// Como o diálogo de backup, o menu roda o **próprio** loop de mensagens e
    /// reentra neste `WndProc`: nenhum empréstimo do `App` pode estar vivo
    /// enquanto ele está aberto.
    fn run_tray_menu(hwnd: HWND) {
        // SAFETY: empréstimo curto, só para copiar o idioma, e solto aqui.
        let Some(language) = (unsafe { app_mut(hwnd).map(|app| app.language) }) else {
            return;
        };
        let tr = i18n::tr(language);
        let labels = tray::MenuLabels {
            open: tr.tray.open,
            exit: tr.tray.exit,
        };
        match tray::show_menu(hwnd, labels) {
            Some(tray::Command::Open) => restore_window(hwnd),
            Some(tray::Command::Exit) => quit_app(hwnd),
            None => {}
        }
    }

    /// Encerra o app de verdade: destrói a janela, o que solta o ícone da
    /// bandeja (no `Drop` do `App`) e fecha o loop de mensagens.
    fn quit_app(hwnd: HWND) {
        // SAFETY: empréstimo curto, solto antes do `DestroyWindow` — que chama
        // `WM_DESTROY` e `WM_NCDESTROY` de dentro dele mesmo.
        unsafe {
            if let Some(app) = app_mut(hwnd) {
                app.quitting = true;
            }
        }
        // SAFETY: janela viva, na própria thread dela.
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }

    // --- WndProc ---

    /// Empresta o estado da janela. Cada handler cria o empréstimo no menor
    /// escopo possível: chamadas como `SetWindowPos` reentram no `WndProc`, e
    /// duas referências mutáveis vivas ao mesmo tempo seriam UB.
    unsafe fn app_mut<'a>(hwnd: HWND) -> Option<&'a mut App> {
        // SAFETY: o ponteiro é o `Box<App>` guardado no `WM_NCCREATE` e só é
        // liberado no `WM_NCDESTROY`.
        let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
        (raw != 0).then(|| unsafe { &mut *(raw as *mut App) })
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        // SAFETY: todo o corpo roda na thread dona da janela, com os ponteiros
        // que o próprio Windows entrega.
        unsafe {
            match message {
                WM_NCCREATE => {
                    let create = &*(lparam.0 as *const CREATESTRUCTW);
                    let boot = Box::from_raw(create.lpCreateParams as *mut Boot);
                    match App::new(hwnd, *boot) {
                        Ok(app) => {
                            SetWindowLongPtrW(
                                hwnd,
                                GWLP_USERDATA,
                                Box::into_raw(Box::new(app)) as isize,
                            );
                        }
                        Err(err) => {
                            log::error!("janela principal não pôde iniciar: {err:#}");
                            return LRESULT(0);
                        }
                    }
                    DefWindowProcW(hwnd, message, wparam, lparam)
                }
                WM_PAINT => {
                    let mut paint = PAINTSTRUCT::default();
                    BeginPaint(hwnd, &mut paint);
                    if let Some(app) = app_mut(hwnd) {
                        app.paint();
                    }
                    let _ = EndPaint(hwnd, &paint);
                    LRESULT(0)
                }
                // O Direct2D pinta cada pixel do cliente; deixar o Windows
                // limpar antes só produziria tremulação.
                WM_ERASEBKGND => LRESULT(1),
                WM_SIZE => {
                    let (width, height) = (loword(lparam.0 as u32), hiword(lparam.0 as u32));
                    if let Some(app) = app_mut(hwnd) {
                        app.on_resize(width as u32, height as u32);
                    }
                    schedule_bounds_save(hwnd);
                    LRESULT(0)
                }
                WM_MOVE => {
                    schedule_bounds_save(hwnd);
                    LRESULT(0)
                }
                WM_GETMINMAXINFO => {
                    let info = &mut *(lparam.0 as *mut MINMAXINFO);
                    let scale = Scale::from_dpi(window_dpi(hwnd));
                    info.ptMinTrackSize = POINT {
                        x: scale.px(MIN_WIDTH),
                        y: scale.px(MIN_HEIGHT),
                    };
                    LRESULT(0)
                }
                WM_DPICHANGED => {
                    let dpi = loword(wparam.0 as u32) as u32;
                    if let Some(app) = app_mut(hwnd) {
                        app.on_dpi_changed(dpi);
                    }
                    // O Windows sugere o retângulo já convertido; aplicá-lo
                    // reentra em `WM_SIZE`, então nenhum empréstimo pode estar vivo.
                    let suggested = &*(lparam.0 as *const RECT);
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        suggested.left,
                        suggested.top,
                        suggested.right - suggested.left,
                        suggested.bottom - suggested.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    LRESULT(0)
                }
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    // Só a captura de atalho consome tecla; o resto segue para
                    // o tratamento padrão da janela.
                    let captured = app_mut(hwnd).is_some_and(|app| app.on_key(wparam.0 as u16));
                    if captured {
                        LRESULT(0)
                    } else {
                        DefWindowProcW(hwnd, message, wparam, lparam)
                    }
                }
                WM_MOUSEMOVE => {
                    if let Some(app) = app_mut(hwnd) {
                        if !app.tracking_mouse {
                            let mut track = TRACKMOUSEEVENT {
                                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                                dwFlags: TME_LEAVE,
                                hwndTrack: hwnd,
                                dwHoverTime: 0,
                            };
                            let _ = TrackMouseEvent(&mut track);
                            app.tracking_mouse = true;
                        }
                        let (x, y) = mouse_point(lparam);
                        let (x, y) = app.dip(x, y);
                        app.on_mouse(Input::Move { x, y });
                    }
                    LRESULT(0)
                }
                WM_MOUSELEAVE => {
                    if let Some(app) = app_mut(hwnd) {
                        app.tracking_mouse = false;
                        app.on_mouse(Input::Leave);
                    }
                    LRESULT(0)
                }
                WM_LBUTTONDOWN => {
                    SetCapture(hwnd);
                    if let Some(app) = app_mut(hwnd) {
                        let (x, y) = mouse_point(lparam);
                        let (x, y) = app.dip(x, y);
                        app.on_mouse(Input::Down { x, y });
                    }
                    LRESULT(0)
                }
                WM_LBUTTONUP => {
                    let _ = ReleaseCapture();
                    if let Some(app) = app_mut(hwnd) {
                        let (x, y) = mouse_point(lparam);
                        let (x, y) = app.dip(x, y);
                        app.on_mouse(Input::Up { x, y });
                    }
                    LRESULT(0)
                }
                WM_MOUSEWHEEL => {
                    if let Some(app) = app_mut(hwnd) {
                        // A roda chega em coordenadas de tela.
                        let (x, y) = mouse_point(lparam);
                        let mut point = POINT { x, y };
                        let _ = ScreenToClient(hwnd, &mut point);
                        let (x, y) = app.dip(point.x, point.y);
                        let delta = hiword(wparam.0 as u32) as i16 as f32 / WHEEL_DELTA as f32;
                        app.on_mouse(Input::Wheel { x, y, delta });
                    }
                    LRESULT(0)
                }
                WM_SETCURSOR => {
                    let over_client = loword(lparam.0 as u32) as u32 == HTCLIENT;
                    let interactive = app_mut(hwnd).is_some_and(|app| app.ui.hot().is_some());
                    if over_client && interactive {
                        if let Ok(cursor) = LoadCursorW(None, IDC_HAND) {
                            SetCursor(Some(cursor));
                        }
                        return LRESULT(1);
                    }
                    DefWindowProcW(hwnd, message, wparam, lparam)
                }
                WM_COMMAND => {
                    let notification = hiword(wparam.0 as u32) as u32;
                    let control = HWND(lparam.0 as *mut _);
                    if let Some(app) = app_mut(hwnd) {
                        match notification {
                            EN_CHANGE => app.on_edit_changed(control),
                            EN_SETFOCUS => {
                                let focused = app
                                    .edits
                                    .iter()
                                    .find(|edit| edit.hwnd == control)
                                    .map(|edit| edit.id);
                                if app.focused_edit != focused {
                                    app.focused_edit = focused;
                                    app.rebuild();
                                }
                            }
                            EN_KILLFOCUS => {
                                // Só o dono atual do foco limpa o estado: numa
                                // troca de campo o `EN_SETFOCUS` do novo pode
                                // já ter passado por aqui.
                                let leaving = app
                                    .edits
                                    .iter()
                                    .find(|edit| edit.hwnd == control)
                                    .map(|edit| edit.id);
                                if leaving.is_some() && app.focused_edit == leaving {
                                    app.focused_edit = None;
                                    app.rebuild();
                                }
                            }
                            _ => {}
                        }
                    }
                    LRESULT(0)
                }
                WM_CTLCOLOREDIT => {
                    let dc = HDC(wparam.0 as *mut _);
                    SetTextColor(dc, colorref(theme::TEXT));
                    let background = theme::SURFACE.over(theme::BG_DEEP);
                    SetBkColor(dc, colorref(background));
                    match app_mut(hwnd) {
                        Some(app) => LRESULT(app.edit_brush.0 as isize),
                        None => DefWindowProcW(hwnd, message, wparam, lparam),
                    }
                }
                WM_TIMER => match wparam.0 {
                    TIMER_BOUNDS => {
                        let _ = KillTimer(Some(hwnd), TIMER_BOUNDS);
                        save_bounds(hwnd);
                        LRESULT(0)
                    }
                    TIMER_ANIM => {
                        if let Some(app) = app_mut(hwnd) {
                            app.rebuild();
                        }
                        LRESULT(0)
                    }
                    TIMER_UPDATE => {
                        let _ = KillTimer(Some(hwnd), TIMER_UPDATE);
                        // Com o jogo em foco a checagem não acontece agora: ela
                        // fica adiada para a primeira perda de foco (Fase 3).
                        if let Some(app) = app_mut(hwnd) {
                            updater::auto_check(&app.shared);
                        }
                        LRESULT(0)
                    }
                    _ => DefWindowProcW(hwnd, message, wparam, lparam),
                },
                WM_APP_BACKUP => {
                    run_pending_backup(hwnd);
                    LRESULT(0)
                }
                WM_APP_EDIT => {
                    run_pending_edit_ops(hwnd);
                    LRESULT(0)
                }
                WM_SHOWWINDOW => {
                    // A volta da bandeja: as animações congeladas terminam e o
                    // timer é rearmado (ver `sync_anim_timer`).
                    if wparam.0 != 0 {
                        if let Some(app) = app_mut(hwnd) {
                            app.rebuild();
                        }
                    }
                    DefWindowProcW(hwnd, message, wparam, lparam)
                }
                // Sem `NIM_SETVERSION`, o evento de mouse do ícone vem na parte
                // baixa do `lParam` — o formato clássico da bandeja.
                WM_APP_TRAY => {
                    match loword(lparam.0 as u32) as u32 {
                        WM_LBUTTONUP | WM_LBUTTONDBLCLK => restore_window(hwnd),
                        WM_RBUTTONUP | WM_CONTEXTMENU => run_tray_menu(hwnd),
                        _ => {}
                    }
                    LRESULT(0)
                }
                WM_SYSCOMMAND => {
                    // Minimizar recolhe pra bandeja: o app segue rodando os
                    // macros em segundo plano (regra da v1).
                    let minimizing = wparam.0 & 0xFFF0 == SC_MINIMIZE as usize;
                    let to_tray = minimizing && app_mut(hwnd).is_some_and(|app| app.tray.is_some());
                    if to_tray {
                        hide_window(hwnd);
                        LRESULT(0)
                    } else {
                        DefWindowProcW(hwnd, message, wparam, lparam)
                    }
                }
                WM_CLOSE => {
                    // Fechar também esconde — menos quando o pedido veio do
                    // "Sair" do menu, ou quando não há ícone na bandeja: aí a
                    // janela escondida seria irrecuperável.
                    let to_tray =
                        app_mut(hwnd).is_some_and(|app| !app.quitting && app.tray.is_some());
                    if to_tray {
                        hide_window(hwnd);
                        LRESULT(0)
                    } else {
                        DefWindowProcW(hwnd, message, wparam, lparam)
                    }
                }
                WM_APP_UI_EVENT => {
                    if let Some(app) = app_mut(hwnd) {
                        app.drain_events();
                    }
                    LRESULT(0)
                }
                WM_APP_QUIT => {
                    quit_app(hwnd);
                    LRESULT(0)
                }
                WM_DESTROY => {
                    let _ = KillTimer(Some(hwnd), TIMER_BOUNDS);
                    save_bounds(hwnd);
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                WM_NCDESTROY => {
                    let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    if raw != 0 {
                        drop(Box::from_raw(raw as *mut App));
                    }
                    DefWindowProcW(hwnd, message, wparam, lparam)
                }
                _ => DefWindowProcW(hwnd, message, wparam, lparam),
            }
        }
    }

    fn schedule_bounds_save(hwnd: HWND) {
        // SAFETY: timer da própria janela; reagendar reinicia a contagem, que é
        // exatamente o efeito de debounce desejado.
        unsafe {
            SetTimer(Some(hwnd), TIMER_BOUNDS, BOUNDS_DEBOUNCE_MS, None);
        }
    }

    fn save_bounds(hwnd: HWND) {
        let mut rect = RECT::default();
        // SAFETY: retângulo próprio, vivo durante a chamada.
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
            return;
        }
        // Janela minimizada reporta coordenadas fora da tela; guardar isso faria
        // a próxima abertura cair no padrão.
        // SAFETY: leitura de estado da janela.
        if unsafe { IsIconic(hwnd) }.as_bool() {
            return;
        }
        Bounds {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
        .save();
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn the_registered_class_matches_the_name_a_second_instance_looks_for() {
            // SAFETY: literal estático terminado em nulo.
            let registered = unsafe { super::CLASS.to_string() }.expect("classe em UTF-16");
            assert_eq!(registered, crate::ui::window::CLASS_NAME);
        }
    }
}

/// Fora do Windows não há janela: o binário serve para `check` e `test`.
#[cfg(not(windows))]
pub fn run(
    _shared: std::sync::Arc<crate::shared::Shared>,
    _data: std::sync::Arc<crate::data::GameData>,
    _ui_rx: crossbeam_channel::Receiver<crate::shared::UiEvent>,
) -> anyhow::Result<()> {
    anyhow::bail!("a janela principal só existe no Windows")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARY: Bounds = Bounds {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    };
    const SECOND: Bounds = Bounds {
        x: 1920,
        y: 0,
        width: 1280,
        height: 1000,
    };

    #[test]
    fn bounds_inside_a_monitor_are_kept() {
        let bounds = Bounds {
            x: 100,
            y: 80,
            width: 820,
            height: 640,
        };
        assert!(bounds.fits_in(&[PRIMARY]));
        assert!(bounds.fits_in(&[PRIMARY, SECOND]));
    }

    #[test]
    fn bounds_on_a_monitor_that_went_away_are_discarded() {
        let on_second = Bounds {
            x: 2000,
            y: 100,
            width: 820,
            height: 640,
        };
        assert!(on_second.fits_in(&[PRIMARY, SECOND]));
        assert!(
            !on_second.fits_in(&[PRIMARY]),
            "sem o segundo monitor a janela abriria fora da tela"
        );
    }

    #[test]
    fn bounds_that_only_half_fit_are_discarded() {
        // A v1 exigia a janela inteira dentro de um único monitor, e não
        // espalhada entre dois.
        let straddling = Bounds {
            x: 1800,
            y: 100,
            width: 820,
            height: 640,
        };
        assert!(!straddling.fits_in(&[PRIMARY, SECOND]));

        let too_tall = Bounds {
            x: 0,
            y: 900,
            width: 820,
            height: 640,
        };
        assert!(!too_tall.fits_in(&[PRIMARY]));
    }

    #[test]
    fn degenerate_bounds_never_pass() {
        let empty = Bounds {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        assert!(!empty.fits_in(&[PRIMARY]));
        assert!(!PRIMARY.fits_in(&[]));
    }

    #[test]
    fn bounds_round_trip_through_the_v1_json_shape() {
        let json = br#"{"x":10,"y":20,"width":820,"height":640}"#;
        let bounds: Bounds = serde_json::from_slice(json).unwrap();
        assert_eq!(bounds.width, 820);
        assert_eq!(
            serde_json::to_string(&bounds).unwrap(),
            r#"{"x":10,"y":20,"width":820,"height":640}"#
        );
    }
}
