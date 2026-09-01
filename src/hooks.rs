//! Atalhos globais: hook de teclado de baixo nível e a tabela que ele consulta.
//!
//! A v1 usava `globalShortcut` do Electron, que registra e desregistra
//! acelerador a cada mudança de foco. Aqui o hook fica instalado o tempo todo e
//! quem decide é uma tabela pré-resolvida: o callback compara um virtual-key,
//! encontra um comando pronto e o manda pro engine. Nada de consultar settings,
//! resolver estratagema ou alocar no caminho quente — o Windows derruba um
//! `WH_KEYBOARD_LL` que passe do `LowLevelHooksTimeout` (~300ms por padrão), e
//! um hook derrubado é um macro que deixa de funcionar até o app reiniciar.
//!
//! O hook roda na thread que o instalou, então tudo o que ele faz sai da fila de
//! mensagens desta thread: comparar, mandar num canal, voltar.
//!
//! ## Teste manual (Windows)
//!
//! Sem janela ainda (Fase 4) e sem `slots.json` (Fase 5), o caminho testável é o
//! atalho de apoio fixo. Edite `%APPDATA%\Macro Helldivers 2\settings.json`:
//!
//! ```json
//! { "supportShortcuts": ["F5", null, null] }
//! ```
//!
//! 1. Abra o Notepad e renomeie o arquivo para `HELLDIVERS 2 test.txt`, de modo
//!    que o título da janela contenha "HELLDIVERS" (R10 classifica por título).
//! 2. Rode `cargo run` num terminal e volte pro Notepad.
//! 3. `F5` digita a sequência do Reforço (Ctrl + ↑↓→←↑ em WASD) e **não** deixa
//!    o `F5` chegar no Notepad.
//! 4. `Shift+F5` e `Ctrl+F5` disparam igual — qualquer modificador seguro serve,
//!    que é o motivo de o "modificador de sprint" da v1 ter sido removido.
//! 5. Segurar `F5` dispara uma vez só: o auto-repeat do teclado é engolido.
//! 6. Alt-tab pro terminal e `F5` não dispara mais nada (foco perdido).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread::JoinHandle;

use crate::data::{Dir, GameData, SUPPORT_STRATS};
use crate::keys::{self, Scan, Vk};
use crate::settings::{Settings, Speed};
use crate::shared::{EngineCmd, OverlayCmd, Shared, Slots};

/// Virtual-key do atalho do overlay (`Ctrl+H`, o mesmo da v1). O Ctrl vem do
/// estado real do teclado, não da tabela.
const OVERLAY_HOTKEY_VK: Vk = 0x48;

/// Um atalho já resolvido: do virtual-key até a sequência que ele executa.
///
/// Tudo o que o engine precisa é montado aqui, na reconstrução da tabela, para
/// que o callback só copie campos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub vk: Vk,
    /// Índice do slot de macro ou do apoio fixo, conforme `support`.
    pub slot: usize,
    pub support: bool,
    pub codex: Arc<[Dir]>,
    pub modifier: Scan,
    pub use_arrows: bool,
    pub speed: Speed,
}

impl Binding {
    /// Comando pronto pro engine. Só clona um `Arc` e campos `Copy`.
    pub fn command(&self) -> EngineCmd {
        EngineCmd::Run {
            codex: Arc::clone(&self.codex),
            modifier: self.modifier,
            use_arrows: self.use_arrows,
            speed: self.speed,
            slot: self.slot,
            support: self.support,
        }
    }
}

/// Tabela consultada pelo callback: os atalhos e as decisões que não dependem
/// deles.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bindings {
    entries: Vec<Binding>,
    /// `enableOverlay` congelado aqui para o callback não abrir os settings.
    overlay_hotkey: bool,
}

impl Bindings {
    /// Resolve settings + slots numa tabela. Slots vazios, atalhos em branco e
    /// nomes que não existem em [`keys`] simplesmente não viram entrada.
    pub fn build(settings: &Settings, slots: &Slots, data: &GameData) -> Bindings {
        let modifier = keys::modifier_scan(&settings.modifier_key);
        let mut entries: Vec<Binding> = Vec::new();

        let mut push = |vk: Vk, slot: usize, support: bool, codex: Arc<[Dir]>| {
            // Mesmo virtual-key em dois atalhos: o primeiro fica. A v1 chegava
            // no mesmo lugar por outro caminho — o segundo `globalShortcut`
            // com o mesmo acelerador falhava no registro.
            if entries.iter().any(|entry| entry.vk == vk) {
                log::warn!(
                    "atalho duplicado ignorado (vk {vk:#04X}) no {} {slot}",
                    if support { "apoio" } else { "slot" }
                );
                return;
            }
            entries.push(Binding {
                vk,
                slot,
                support,
                codex,
                modifier,
                use_arrows: settings.use_arrows,
                speed: settings.macro_speed,
            });
        };

        for (slot, id) in slots.iter().enumerate() {
            let Some(vk) = settings.shortcut(slot).and_then(keys::vk_from_name) else {
                continue;
            };
            let Some(strat) = id.and_then(|id| data.by_id(id)) else {
                continue;
            };
            push(vk, slot, false, strat.codex.as_slice().into());
        }

        for (slot, support) in SUPPORT_STRATS.iter().enumerate() {
            let Some(vk) = settings.support_shortcut(slot).and_then(keys::vk_from_name) else {
                continue;
            };
            push(vk, slot, true, support.codex.into());
        }

        Bindings {
            entries,
            overlay_hotkey: settings.enable_overlay,
        }
    }

    /// Busca linear: são no máximo sete entradas, e um `HashMap` custaria hash e
    /// indireção justamente onde o orçamento é apertado.
    pub fn find(&self, vk: Vk) -> Option<&Binding> {
        self.entries.iter().find(|entry| entry.vk == vk)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// O que fazer com uma tecla que acabou de descer.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Dispara a sequência e engole a tecla.
    Run(EngineCmd),
    /// Abre/fecha o painel do overlay e engole a tecla.
    ToggleOverlay,
    /// Não é nossa: segue para o app em foco.
    PassThrough,
}

/// Decide o destino de uma tecla. Puro, para o teste cobrir a mesma lógica que
/// roda dentro do hook.
///
/// `armed` é `jogo em foco && não estamos capturando atalho`: fora disso o app
/// não interfere no teclado de ninguém.
pub fn decide(bindings: &Bindings, vk: Vk, ctrl_down: bool, armed: bool) -> Decision {
    if !armed {
        return Decision::PassThrough;
    }
    // Ctrl+H antes dos slots, como na v1: lá o acelerador do overlay era
    // registrado antes dos atalhos de macro.
    if vk == OVERLAY_HOTKEY_VK && ctrl_down && bindings.overlay_hotkey {
        return Decision::ToggleOverlay;
    }
    match bindings.find(vk) {
        Some(binding) => Decision::Run(binding.command()),
        None => Decision::PassThrough,
    }
}

/// Conjunto de virtual-keys (0..=255) em quatro palavras atômicas.
///
/// Guarda quais teclas foram engolidas na descida, para engolir também a subida
/// — o app em foco nunca vê meio evento — e para reconhecer o auto-repeat.
struct KeySet([AtomicU64; 4]);

impl KeySet {
    const fn new() -> KeySet {
        KeySet([
            AtomicU64::new(0),
            AtomicU64::new(0),
            AtomicU64::new(0),
            AtomicU64::new(0),
        ])
    }

    fn bit(vk: Vk) -> Option<(usize, u64)> {
        (vk < 256).then(|| ((vk / 64) as usize, 1u64 << (vk % 64)))
    }

    /// Marca a tecla e devolve `true` se ela já estava marcada (auto-repeat).
    fn insert(&self, vk: Vk) -> bool {
        match KeySet::bit(vk) {
            Some((word, bit)) => self.0[word].fetch_or(bit, Ordering::Relaxed) & bit != 0,
            None => false,
        }
    }

    /// Desmarca a tecla e devolve `true` se ela estava marcada.
    fn remove(&self, vk: Vk) -> bool {
        match KeySet::bit(vk) {
            Some((word, bit)) => self.0[word].fetch_and(!bit, Ordering::Relaxed) & bit != 0,
            None => false,
        }
    }
}

/// O que o callback do hook enxerga. Global porque um `HOOKPROC` é uma função
/// crua, sem estado próprio.
struct Runtime {
    shared: Arc<Shared>,
    data: Arc<GameData>,
    /// Lida a cada tecla, escrita só quando settings ou slots mudam (clique do
    /// usuário). Com escrita tão rara, o `read()` nunca espera na prática, e um
    /// `RwLock` sai mais barato que trocar um `Arc` inteiro por atalho.
    bindings: RwLock<Bindings>,
    swallowed: KeySet,
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Prepara a tabela de atalhos. Chamar uma vez, antes de [`spawn`].
pub fn init(shared: Arc<Shared>, data: Arc<GameData>) {
    let bindings = Bindings::build(&shared.settings_snapshot(), &shared.slots(), &data);
    log::debug!("{} atalho(s) resolvido(s)", bindings.len());

    let runtime = Runtime {
        shared,
        data,
        bindings: RwLock::new(bindings),
        swallowed: KeySet::new(),
    };
    if RUNTIME.set(runtime).is_err() {
        log::warn!("hooks::init chamado mais de uma vez; a tabela original continua valendo");
    }
}

/// Reconstrói a tabela a partir do estado atual. Chamar depois de toda mudança
/// de settings ou de slots.
pub fn rebuild_bindings() {
    let Some(runtime) = RUNTIME.get() else {
        log::warn!("rebuild_bindings antes de hooks::init");
        return;
    };
    let bindings = Bindings::build(
        &runtime.shared.settings_snapshot(),
        &runtime.shared.slots(),
        &runtime.data,
    );
    log::debug!(
        "tabela de atalhos reconstruída: {} entrada(s)",
        bindings.len()
    );
    *lock_write(&runtime.bindings) = bindings;
}

/// Escrita que sobrevive a um leitor que entrou em panic: a tabela é dado puro,
/// e ficar sem atalhos seria pior que seguir com a cópia anterior.
fn lock_write(lock: &RwLock<Bindings>) -> std::sync::RwLockWriteGuard<'_, Bindings> {
    lock.write().unwrap_or_else(|err| err.into_inner())
}

/// Sobe a thread dos hooks. Ela vive até o processo terminar.
pub fn spawn() -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("hooks".to_string())
        .spawn(run)
}

#[cfg(not(windows))]
fn run() {
    // Fora do Windows não há hook para instalar; a tabela de atalhos continua
    // sendo construída e testada normalmente.
    log::info!("thread de hooks inativa fora do Windows");
}

#[cfg(windows)]
fn run() {
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, WH_KEYBOARD_LL,
    };

    if RUNTIME.get().is_none() {
        log::error!("thread de hooks sem hooks::init; atalhos desligados");
        return;
    }

    // SAFETY: `None` pede o módulo do próprio processo, que sempre existe.
    let module = match unsafe { GetModuleHandleW(None) } {
        Ok(module) => HINSTANCE::from(module),
        Err(err) => {
            log::error!("GetModuleHandleW falhou ({err}); atalhos desligados");
            return;
        }
    };

    // SAFETY: o callback é uma `extern "system"` do próprio módulo e o hook é
    // removido pelo guard antes de a thread morrer.
    let hook =
        match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(module), 0) } {
            Ok(hook) => KeyboardHook(hook),
            Err(err) => {
                log::error!("SetWindowsHookExW recusado ({err}); atalhos desligados");
                return;
            }
        };
    log::info!("hook de teclado instalado");

    pump();
    drop(hook);

    /// Desinstala o hook aconteça o que acontecer com a thread.
    struct KeyboardHook(HHOOK);

    impl Drop for KeyboardHook {
        fn drop(&mut self) {
            // SAFETY: handle devolvido pelo `SetWindowsHookExW` desta thread.
            if let Err(err) = unsafe { UnhookWindowsHookEx(self.0) } {
                log::warn!("UnhookWindowsHookEx falhou: {err}");
            }
        }
    }
}

/// Fila de mensagens da thread: é ela que entrega as chamadas do hook.
#[cfg(windows)]
fn pump() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };

    let mut msg = MSG::default();
    loop {
        // SAFETY: `msg` é nosso e vive durante a chamada.
        let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        match result.0 {
            0 => break,
            -1 => {
                log::error!("GetMessageW falhou na thread de hooks; encerrando o pump");
                break;
            }
            _ => {
                // SAFETY: mensagem recém-preenchida pelo `GetMessageW`.
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
}

/// Callback do `WH_KEYBOARD_LL`. Roda em toda tecla do sistema: compara, manda
/// num canal e sai. Devolver 1 engole a tecla; qualquer outra coisa a repassa.
#[cfg(windows)]
unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    if code == HC_ACTION as i32 {
        if let Some(runtime) = RUNTIME.get() {
            // SAFETY: em `HC_ACTION` o `lParam` é um `KBDLLHOOKSTRUCT` válido
            // durante toda a chamada, por contrato da API.
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

            // Nossos próprios `SendInput` passam por aqui: sem este filtro, um
            // slot mapeado em W dispararia a si mesmo.
            if !info.flags.contains(LLKHF_INJECTED) {
                let vk = info.vkCode as Vk;
                let handled = match wparam.0 as u32 {
                    WM_KEYDOWN | WM_SYSKEYDOWN => on_key_down(runtime, vk),
                    // A subida de uma tecla engolida também não pode vazar, ou o
                    // app em foco vê um KEYUP sem KEYDOWN.
                    WM_KEYUP | WM_SYSKEYUP => runtime.swallowed.remove(vk),
                    _ => false,
                };
                if handled {
                    return LRESULT(1);
                }
            }
        }
    }

    // SAFETY: repassa os mesmos parâmetros adiante na cadeia, como manda a API.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// `true` quando a tecla é nossa e deve ser engolida.
#[cfg(windows)]
fn on_key_down(runtime: &Runtime, vk: Vk) -> bool {
    let armed = runtime.shared.is_game_focused() && !runtime.shared.is_recording();
    let decision = {
        // Leitor que sobrevive a escritor em panic, pelo mesmo motivo de `lock_write`.
        let bindings = runtime
            .bindings
            .read()
            .unwrap_or_else(|err| err.into_inner());
        decide(&bindings, vk, ctrl_down(), armed)
    };

    match decision {
        Decision::PassThrough => false,
        // Segurar a tecla repete o KEYDOWN dezenas de vezes por segundo. Uma
        // pressão física = uma sequência; as repetições só somem do caminho.
        Decision::Run(cmd) => {
            if !runtime.swallowed.insert(vk) {
                runtime.shared.send_engine(cmd);
            }
            true
        }
        Decision::ToggleOverlay => {
            if !runtime.swallowed.insert(vk) {
                runtime.shared.send_overlay(OverlayCmd::Toggle);
            }
            true
        }
    }
}

#[cfg(windows)]
fn ctrl_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};

    // SAFETY: leitura de estado global do teclado, sem ponteiros.
    // O bit alto marca "pressionada agora", e ele é o bit de sinal do i16.
    unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Language, SLOT_COUNT};

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    /// Ids dos primeiros estratagemas do arquivo, para preencher slots.
    fn first_ids(data: &GameData, n: usize) -> Vec<u32> {
        data.all().iter().take(n).map(|strat| strat.id).collect()
    }

    fn settings_with(shortcuts: [Option<&str>; SLOT_COUNT]) -> Settings {
        Settings {
            shortcuts: shortcuts.map(|name| name.map(str::to_string)),
            ..Settings::default()
        }
    }

    #[test]
    fn slots_become_bindings_with_everything_resolved() {
        let data = data();
        let ids = first_ids(&data, 2);
        let settings = Settings {
            macro_speed: Speed::Turbo,
            use_arrows: true,
            modifier_key: "LeftAlt".to_string(),
            ..Settings::default()
        };
        let slots: Slots = [Some(ids[0]), None, Some(ids[1]), None];

        let bindings = Bindings::build(&settings, &slots, &data);

        assert_eq!(bindings.len(), 2);
        let first = bindings.find(keys::vk_from_name("F1").unwrap()).unwrap();
        assert_eq!(first.slot, 0);
        assert!(!first.support);
        assert_eq!(&*first.codex, data.by_id(ids[0]).unwrap().codex.as_slice());
        assert_eq!(first.modifier, keys::modifier_scan("LeftAlt"));
        assert_eq!(first.speed, Speed::Turbo);
        assert!(first.use_arrows);

        // O slot 2 responde por F3, e o slot vazio não entra na tabela.
        assert_eq!(
            bindings
                .find(keys::vk_from_name("F3").unwrap())
                .map(|binding| binding.slot),
            Some(2)
        );
        assert!(bindings.find(keys::vk_from_name("F2").unwrap()).is_none());
    }

    #[test]
    fn support_shortcuts_carry_the_fixed_codexes() {
        let data = data();
        let settings = Settings {
            support_shortcuts: [Some("F5".into()), None, Some("F7".into())],
            ..settings_with([None, None, None, None])
        };

        let bindings = Bindings::build(&settings, &Slots::default(), &data);

        let reinforce = bindings.find(keys::vk_from_name("F5").unwrap()).unwrap();
        assert!(reinforce.support);
        assert_eq!(reinforce.slot, 0);
        assert_eq!(&*reinforce.codex, SUPPORT_STRATS[0].codex);

        let rearm = bindings.find(keys::vk_from_name("F7").unwrap()).unwrap();
        assert_eq!(rearm.slot, 2);
        assert_eq!(&*rearm.codex, SUPPORT_STRATS[2].codex);
        // O apoio do meio ficou sem atalho.
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn unknown_names_and_missing_stratagems_are_skipped() {
        let data = data();
        let settings = settings_with([Some("Ctrl+Shift+Q"), Some("F2"), None, Some("F4")]);
        // Slot 1 aponta pra um id que não existe mais no JSON.
        let slots: Slots = [Some(0), Some(9_999), Some(0), None];

        let bindings = Bindings::build(&settings, &slots, &data);

        assert_eq!(bindings.len(), 0);
    }

    #[test]
    fn the_first_shortcut_wins_when_two_share_a_key() {
        let data = data();
        let id = first_ids(&data, 1)[0];
        let settings = Settings {
            support_shortcuts: [Some("F1".into()), None, None],
            ..settings_with([Some("F1"), None, None, None])
        };

        let bindings = Bindings::build(&settings, &[Some(id), None, None, None], &data);

        assert_eq!(bindings.len(), 1);
        assert!(
            !bindings
                .find(keys::vk_from_name("F1").unwrap())
                .unwrap()
                .support
        );
    }

    fn armed_bindings() -> (GameData, Bindings) {
        let data = data();
        let id = data.all()[0].id;
        let bindings = Bindings::build(&Settings::default(), &[Some(id), None, None, None], &data);
        (data, bindings)
    }

    #[test]
    fn a_bound_key_becomes_a_ready_engine_command() {
        let (data, bindings) = armed_bindings();
        let f1 = keys::vk_from_name("F1").unwrap();

        let Decision::Run(EngineCmd::Run {
            codex,
            slot,
            support,
            ..
        }) = decide(&bindings, f1, false, true)
        else {
            panic!("F1 deveria disparar o slot 0");
        };
        assert_eq!(slot, 0);
        assert!(!support);
        assert_eq!(&*codex, data.all()[0].codex.as_slice());
    }

    #[test]
    fn nothing_is_swallowed_while_disarmed() {
        let (_data, bindings) = armed_bindings();
        let f1 = keys::vk_from_name("F1").unwrap();

        // Fora do jogo, ou capturando um atalho na tela de configurações.
        assert_eq!(decide(&bindings, f1, false, false), Decision::PassThrough);
        assert_eq!(
            decide(&bindings, OVERLAY_HOTKEY_VK, true, false),
            Decision::PassThrough
        );
    }

    #[test]
    fn ctrl_h_toggles_the_overlay_only_when_it_is_enabled() {
        let data = data();
        let enabled = Bindings::build(&Settings::default(), &Slots::default(), &data);
        assert_eq!(
            decide(&enabled, OVERLAY_HOTKEY_VK, true, true),
            Decision::ToggleOverlay
        );
        // Sem Ctrl é um H comum, e vai pro jogo.
        assert_eq!(
            decide(&enabled, OVERLAY_HOTKEY_VK, false, true),
            Decision::PassThrough
        );

        let off = Bindings::build(
            &Settings {
                enable_overlay: false,
                ..Settings::default()
            },
            &Slots::default(),
            &data,
        );
        assert_eq!(
            decide(&off, OVERLAY_HOTKEY_VK, true, true),
            Decision::PassThrough
        );
    }

    #[test]
    fn the_overlay_hotkey_is_the_h_key() {
        assert_eq!(keys::vk_from_name("H"), Some(OVERLAY_HOTKEY_VK));
    }

    #[test]
    fn unrelated_keys_pass_through() {
        let (_data, bindings) = armed_bindings();
        let f9 = keys::vk_from_name("F9").unwrap();
        assert_eq!(decide(&bindings, f9, false, true), Decision::PassThrough);
        assert_eq!(decide(&bindings, f9, true, true), Decision::PassThrough);
    }

    #[test]
    fn a_language_change_keeps_the_bindings_intact() {
        let data = data();
        let id = data.all()[0].id;
        let slots: Slots = [Some(id), None, None, None];
        let pt = Bindings::build(&Settings::default(), &slots, &data);
        let en = Bindings::build(
            &Settings {
                language: Language::En,
                ..Settings::default()
            },
            &slots,
            &data,
        );
        assert_eq!(pt, en);
    }

    #[test]
    fn the_key_set_tracks_repeats_and_releases() {
        let set = KeySet::new();
        let f1 = keys::vk_from_name("F1").unwrap();

        assert!(!set.insert(f1), "primeira descida");
        assert!(set.insert(f1), "auto-repeat");
        assert!(set.remove(f1), "a subida fecha o par");
        assert!(!set.remove(f1), "e não engole a próxima");

        // Teclas em palavras diferentes do bitmap não se atrapalham.
        let numpad0 = keys::vk_from_name("Numpad0").unwrap();
        assert!(!set.insert(numpad0));
        assert!(!set.insert(f1));
        assert!(set.remove(numpad0));
        assert!(set.insert(f1));

        // Virtual-key fora da faixa não é rastreável, e não pode entrar em pânico.
        assert!(!set.insert(300));
        assert!(!set.remove(300));
    }
}
