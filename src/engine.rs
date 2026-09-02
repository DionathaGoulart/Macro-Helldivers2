//! Motor de macro: a thread que transforma um atalho em teclas dentro do jogo.
//!
//! É a parte mais sensível do app. Três regras mandam aqui:
//!
//! 1. **Input primeiro, aviso depois.** O press do modificador acontece antes de
//!    qualquer notificação para a UI — feedback visual atrasado ninguém percebe,
//!    input atrasado sim (mesma ordem da v1, `legacy/src/main/index.js` ~336-342).
//! 2. **Fila de tamanho zero.** Um segundo atalho durante a execução é rejeitado
//!    com aviso de bloqueio, nunca enfileirado: uma sequência que chega tarde no
//!    jogo é pior do que uma que não chega.
//! 3. **Nada fica preso.** Toda tecla pressionada é registrada num guard RAII, e
//!    o `Drop` solta tudo em ordem inversa — vale para retorno normal, aborto por
//!    perda de foco e panic (em debug, onde há unwind).
//!
//! O envio usa `SendInput` com scancode: o jogo lê o teclado por scancode, e um
//! evento por virtual-key simplesmente não chega nele.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::Receiver;
use rand::RngExt;
use spin_sleep::SpinSleeper;

use crate::data::Dir;
use crate::keys::{self, Scan};
use crate::settings::Speed;
use crate::shared::{EngineCmd, FlashKind, OverlayCmd, Shared, UiEvent};

/// O jogo lê o teclado uma vez por frame: 16,7ms a 60fps, 33,3ms a 30fps. Uma
/// tecla que desce e sobe entre dois polls não existe para ele, e a macro falha de
/// forma intermitente. Por isso o `hold` de cada perfil é medido em frames e a
/// velocidade vem de encurtar o `gap`, que não tem esse limite.
pub const MIN_HOLD_MS: u32 = 20;

/// Piso das esperas que não são `hold`. Herdado da v1: só impede espera zero.
const MIN_WAIT_MS: u32 = 1;

/// Amplitude do jitter humanizado, para cada lado, em ms.
pub const JITTER_MS: f64 = 5.0;

/// Margem que damos ao sono nativo: o `spin_sleep` dorme `duração - margem` no
/// relógio do SO e gira o resto. Margem maior custa mais CPU dentro da sequência
/// (só durante ela) e compra precisão.
///
/// Este é o primeiro botão a girar se o `timing_bench` num Windows real não
/// fechar a meta de p99 < 1ms: com `timeBeginPeriod(1)` o sono nativo acorda com
/// granularidade de ~1ms, e uma margem menor que o erro dele não sobra giro para
/// compensar.
const NATIVE_SLEEP_ACCURACY_NS: u32 = 500_000;

/// Teclas simultaneamente seguradas numa sequência: modificador + uma direção.
/// A folga é só defensiva.
const MAX_HELD: usize = 4;

/// Intervalos de um perfil de velocidade, em milissegundos.
///
/// Os números são os da v1 (`legacy/src/macro/stratagemRunner.js`). O campo
/// `auto` de lá era o atraso interno do nut-js por press/release e morreu junto
/// com ele: o `SendInput` não tem atraso próprio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// Tecla de direção segurada.
    pub hold: u32,
    /// Pausa entre uma direção e a próxima.
    pub gap: u32,
    /// Pausa depois de segurar o modificador, antes da primeira direção.
    pub lead: u32,
    /// Pausa depois da última direção, antes de soltar o modificador.
    pub tail: u32,
}

impl Profile {
    /// ~1 frame a 30fps.
    pub const NORMAL: Profile = Profile {
        hold: 34,
        gap: 20,
        lead: 100,
        tail: 50,
    };
    /// ~1,5 frame a 60fps.
    pub const FAST: Profile = Profile {
        hold: 24,
        gap: 12,
        lead: 70,
        tail: 40,
    };
    /// ~1,2 frame a 60fps.
    pub const TURBO: Profile = Profile {
        hold: 20,
        gap: 6,
        lead: 50,
        tail: 30,
    };

    pub fn of(speed: Speed) -> Profile {
        match speed {
            Speed::Normal => Profile::NORMAL,
            Speed::Fast => Profile::FAST,
            Speed::Turbo => Profile::TURBO,
        }
    }
}

/// Em que ponto da sequência uma espera acontece. O sink de produção ignora; as
/// bancadas usam para separar o desvio de `hold` do de `gap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Lead,
    Hold,
    Gap,
    Tail,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Lead => "lead",
            Phase::Hold => "hold",
            Phase::Gap => "gap",
            Phase::Tail => "tail",
        }
    }
}

/// Um evento de teclado: uma metade do ciclo de uma tecla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub scan: Scan,
    pub up: bool,
}

/// Para onde o engine manda teclas e como ele espera.
///
/// Em produção é o teclado do Windows; nos testes, um [`Recorder`] que guarda a
/// sequência inteira sem dormir de verdade; nas bancadas, um decorador que
/// carimba o relógio a cada envio.
pub trait InputSink {
    fn send(&mut self, event: KeyEvent);
    fn wait(&mut self, phase: Phase, duration: Duration);
}

/// Jitter humanizado, desligável para o teste conseguir prever cada intervalo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jitter {
    Humanized,
    Off,
}

impl Jitter {
    fn offset_ms(self) -> f64 {
        match self {
            Jitter::Off => 0.0,
            Jitter::Humanized => rand::rng().random_range(-JITTER_MS..JITTER_MS),
        }
    }
}

/// Aplica jitter e piso a um intervalo. Puro de propósito: é aqui que mora o
/// risco de o turbo cair abaixo de um frame, e o teste cobre a faixa inteira.
pub fn jittered_ms(base_ms: u32, floor_ms: u32, offset_ms: f64) -> f64 {
    (f64::from(base_ms) + offset_ms).max(f64::from(floor_ms))
}

/// O que executar. Tudo já resolvido pelo remetente — o engine não consulta
/// settings no meio de uma sequência.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    /// Vem pronto do hook, compartilhado com a tabela de bindings.
    pub codex: Arc<[Dir]>,
    pub modifier: Scan,
    pub use_arrows: bool,
    pub speed: Speed,
}

/// Ganchos da sequência com o resto do app, para `run_sequence` continuar
/// testável sem `Shared`.
pub trait Hooks {
    /// Consultado antes de cada direção. `true` interrompe na hora.
    fn aborted(&mut self) -> bool {
        false
    }

    /// Chamado logo depois do press do modificador, nunca antes.
    fn started(&mut self) {}
}

/// Sequência sem ganchos: roda inteira e não avisa ninguém.
impl Hooks for () {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    Aborted,
}

/// Teclas seguradas no momento. O `Drop` é a garantia de que nenhuma fica presa
/// no jogo quando a sequência termina de qualquer jeito que não seja o normal.
struct Held<'a, S: InputSink> {
    sink: &'a mut S,
    down: [Option<Scan>; MAX_HELD],
}

impl<'a, S: InputSink> Held<'a, S> {
    fn new(sink: &'a mut S) -> Self {
        Held {
            sink,
            down: [None; MAX_HELD],
        }
    }

    fn press(&mut self, scan: Scan) {
        self.sink.send(KeyEvent { scan, up: false });
        match self.down.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => *slot = Some(scan),
            // Inalcançável: a sequência nunca segura mais que duas teclas.
            None => debug_assert!(false, "mais de {MAX_HELD} teclas seguradas"),
        }
    }

    fn release(&mut self, scan: Scan) {
        self.sink.send(KeyEvent { scan, up: true });
        for slot in self.down.iter_mut() {
            if *slot == Some(scan) {
                *slot = None;
            }
        }
    }

    fn wait(&mut self, phase: Phase, base_ms: u32, floor_ms: u32, jitter: Jitter) {
        let ms = jittered_ms(base_ms, floor_ms, jitter.offset_ms());
        self.sink.wait(phase, Duration::from_secs_f64(ms / 1000.0));
    }
}

impl<S: InputSink> Drop for Held<'_, S> {
    fn drop(&mut self) {
        // Ordem inversa: primeiro a última tecla presa (a direção), depois o
        // modificador — do jeito que a mão soltaria.
        for slot in self.down.iter_mut().rev() {
            if let Some(scan) = slot.take() {
                self.sink.send(KeyEvent { scan, up: true });
            }
        }
    }
}

/// Executa uma sequência inteira: modificador, lead, cada direção com hold e
/// gap, tail e a soltura do modificador.
pub fn run_sequence<S: InputSink>(
    sink: &mut S,
    sequence: &Sequence,
    jitter: Jitter,
    hooks: &mut dyn Hooks,
) -> Outcome {
    let profile = Profile::of(sequence.speed);
    let _timer = TimerResolution::acquire();
    let mut held = Held::new(sink);

    held.press(sequence.modifier);
    hooks.started();
    held.wait(Phase::Lead, profile.lead, MIN_WAIT_MS, jitter);

    for dir in sequence.codex.iter() {
        // Perdeu o foco no meio: para agora; o guard solta modificador e direção.
        if hooks.aborted() {
            return Outcome::Aborted;
        }
        let scan = keys::direction_scan(*dir, sequence.use_arrows);
        held.press(scan);
        held.wait(Phase::Hold, profile.hold, MIN_HOLD_MS, jitter);
        held.release(scan);
        held.wait(Phase::Gap, profile.gap, MIN_WAIT_MS, jitter);
    }

    held.wait(Phase::Tail, profile.tail, MIN_WAIT_MS, jitter);
    held.release(sequence.modifier);
    Outcome::Completed
}

/// Solta todas as teclas que uma sequência pode estar segurando agora.
///
/// É a rede de segurança do panic hook: em release o perfil usa `panic =
/// "abort"`, que mata o processo sem rodar o `Drop` do guard — e um panic em
/// qualquer thread no meio de uma sequência deixaria o modificador logicamente
/// preso no sistema até o usuário apertar a tecla física, no meio da partida.
/// Key-up de tecla que não está pressionada é inofensivo, então soltamos o
/// conjunto inteiro em vez de rastrear o que está de fato seguro.
pub fn emergency_release(shared: &Shared) {
    if !shared.macro_running.load(Ordering::Acquire) {
        return;
    }
    for scan in keys::WASD.iter().chain(keys::ARROWS.iter()) {
        send_scan(*scan, true);
    }
    for name in keys::MODIFIER_KEYS {
        send_scan(keys::modifier_scan(name), true);
    }
}

/// Sobe a thread do engine. Ela vive enquanto o canal existir.
pub fn spawn(shared: Arc<Shared>, rx: Receiver<EngineCmd>) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("engine".to_string())
        .spawn(move || {
            raise_priority();
            let mut sink = SystemInput::new();
            run_loop(&shared, &rx, &mut sink);
        })
}

fn run_loop<S: InputSink>(shared: &Shared, rx: &Receiver<EngineCmd>, sink: &mut S) {
    while let Ok(cmd) = rx.recv() {
        handle(shared, rx, sink, cmd);
    }
}

/// Marca o engine como ocupado e libera a marca aconteça o que acontecer.
struct Busy<'a>(&'a Shared);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.macro_running.store(false, Ordering::Release);
    }
}

fn handle<S: InputSink>(
    shared: &Shared,
    rx: &Receiver<EngineCmd>,
    sink: &mut S,
    cmd: EngineCmd,
) -> Outcome {
    let EngineCmd::Run {
        codex,
        modifier,
        use_arrows,
        speed,
        slot,
        support,
    } = cmd;

    if shared
        .macro_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        reject(shared, slot, support);
        return Outcome::Aborted;
    }
    let _busy = Busy(shared);

    // O foco pode ter caído entre o hook enfileirar e o engine acordar. A v1
    // também desistia em silêncio nesse caso.
    if !shared.is_game_focused() {
        return Outcome::Aborted;
    }

    let sequence = Sequence {
        codex,
        modifier,
        use_arrows,
        speed,
    };
    let mut hooks = EngineHooks {
        shared,
        slot,
        support,
    };
    let outcome = run_sequence(sink, &sequence, Jitter::Humanized, &mut hooks);
    shared.send_ui(UiEvent::MacroStatus {
        slot,
        support,
        running: false,
    });

    // Atalhos apertados durante a execução esperam no canal; rejeitamos todos
    // antes de liberar a marca, para que nenhum dispare fora de hora.
    while let Ok(EngineCmd::Run { slot, support, .. }) = rx.try_recv() {
        reject(shared, slot, support);
    }

    outcome
}

fn reject(shared: &Shared, slot: usize, support: bool) {
    shared.send_ui(UiEvent::MacroBlocked { slot, support });
    shared.send_overlay(OverlayCmd::Flash {
        slot,
        support,
        kind: FlashKind::Blocked,
    });
}

/// Liga a sequência ao estado compartilhado: foco para abortar, canais para avisar.
struct EngineHooks<'a> {
    shared: &'a Shared,
    slot: usize,
    support: bool,
}

impl Hooks for EngineHooks<'_> {
    fn aborted(&mut self) -> bool {
        !self.shared.is_game_focused()
    }

    fn started(&mut self) {
        let (slot, support) = (self.slot, self.support);
        self.shared
            .send_ui(UiEvent::MacroTriggered { slot, support });
        self.shared.send_ui(UiEvent::MacroStatus {
            slot,
            support,
            running: true,
        });
        self.shared.send_overlay(OverlayCmd::Flash {
            slot,
            support,
            kind: FlashKind::Triggered,
        });
    }
}

/// Sink de produção: teclado do sistema e sono preciso.
pub struct SystemInput {
    sleeper: SpinSleeper,
}

impl SystemInput {
    pub fn new() -> SystemInput {
        SystemInput {
            sleeper: SpinSleeper::new(NATIVE_SLEEP_ACCURACY_NS),
        }
    }
}

impl Default for SystemInput {
    fn default() -> SystemInput {
        SystemInput::new()
    }
}

impl InputSink for SystemInput {
    fn send(&mut self, event: KeyEvent) {
        send_scan(event.scan, event.up);
    }

    fn wait(&mut self, _phase: Phase, duration: Duration) {
        self.sleeper.sleep(duration);
    }
}

/// Sink de teste: guarda a sequência inteira e avança um relógio virtual, então
/// os testes verificam ordem e intervalos sem esperar de verdade.
#[derive(Debug, Default)]
pub struct Recorder {
    /// Cada evento com o instante virtual em que foi enviado.
    pub events: Vec<(Duration, KeyEvent)>,
    /// Cada espera pedida, na ordem.
    pub waits: Vec<(Phase, Duration)>,
    elapsed: Duration,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder::default()
    }

    /// Só os eventos, para comparar a ordem da sequência de uma vez.
    pub fn key_events(&self) -> Vec<KeyEvent> {
        self.events.iter().map(|(_, event)| *event).collect()
    }

    /// Esperas em milissegundos, arredondadas — sem jitter elas são exatas.
    pub fn wait_ms(&self) -> Vec<(Phase, u64)> {
        self.waits
            .iter()
            .map(|(phase, duration)| (*phase, duration.as_millis() as u64))
            .collect()
    }
}

impl InputSink for Recorder {
    fn send(&mut self, event: KeyEvent) {
        self.events.push((self.elapsed, event));
    }

    fn wait(&mut self, phase: Phase, duration: Duration) {
        self.waits.push((phase, duration));
        self.elapsed += duration;
    }
}

#[cfg(windows)]
fn send_scan(scan: Scan, up: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, VIRTUAL_KEY,
    };

    let mut flags = KEYEVENTF_SCANCODE;
    if scan.extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                // Scancode puro: é o que o jogo lê. O virtual-key fica zerado.
                wVk: VIRTUAL_KEY(0),
                wScan: scan.code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    // SAFETY: um `INPUT` válido, vivo durante a chamada, com o tamanho que a API pede.
    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if sent != 1 {
        // Acontece quando outro processo mais privilegiado está em foco: o
        // Windows bloqueia a injeção (UIPI) em vez de falhar visivelmente.
        log::warn!("SendInput recusado (scancode {:#04X})", scan.code);
    }
}

#[cfg(not(windows))]
fn send_scan(scan: Scan, up: bool) {
    // No host de desenvolvimento não há teclado para onde mandar; o resto do
    // motor continua exercitável pelos testes e pelas bancadas em modo seco.
    log::trace!(
        "send_scan ignorado fora do Windows: {:#04X} {}",
        scan.code,
        if up { "up" } else { "down" }
    );
}

#[cfg(windows)]
fn raise_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
    };

    // SAFETY: pseudo-handle da própria thread, sempre válido e sem dono.
    match unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) } {
        Ok(()) => log::debug!("thread do engine em THREAD_PRIORITY_HIGHEST"),
        // Sem prioridade alta o motor ainda funciona, só fica mais sujeito ao
        // escalonador; não é motivo para derrubar o app.
        Err(err) => log::warn!("não foi possível elevar a prioridade do engine: {err}"),
    }
}

#[cfg(not(windows))]
fn raise_priority() {}

/// Resolução do timer do sistema em 1ms enquanto a sequência roda.
///
/// Sem isso o sono nativo acorda em múltiplos de ~15,6ms e o `spin_sleep` teria
/// que girar a CPU o tempo todo para compensar.
struct TimerResolution {
    #[cfg(windows)]
    acquired: bool,
}

impl TimerResolution {
    #[cfg(windows)]
    fn acquire() -> TimerResolution {
        use windows::Win32::Media::timeBeginPeriod;

        // SAFETY: chamada documentada, sem ponteiros; o `Drop` faz o par.
        let acquired = unsafe { timeBeginPeriod(1) } == 0;
        if !acquired {
            log::warn!("timeBeginPeriod(1) recusado; o sono pode ficar granuloso");
        }
        TimerResolution { acquired }
    }

    #[cfg(not(windows))]
    fn acquire() -> TimerResolution {
        TimerResolution {}
    }
}

#[cfg(windows)]
impl Drop for TimerResolution {
    fn drop(&mut self) {
        use windows::Win32::Media::timeEndPeriod;

        if self.acquired {
            // SAFETY: par exato do `timeBeginPeriod(1)` feito no `acquire`.
            unsafe { timeEndPeriod(1) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use crate::shared::{Receivers, Slots};

    const CTRL: Scan = Scan {
        code: 0x1D,
        extended: false,
    };

    fn sequence(codex: &[Dir], speed: Speed, use_arrows: bool) -> Sequence {
        Sequence {
            codex: codex.into(),
            modifier: CTRL,
            use_arrows,
            speed,
        }
    }

    fn down(code: u16) -> KeyEvent {
        KeyEvent {
            scan: Scan {
                code,
                extended: false,
            },
            up: false,
        }
    }

    fn up(code: u16) -> KeyEvent {
        KeyEvent {
            up: true,
            ..down(code)
        }
    }

    #[test]
    fn speed_profiles_match_the_reference_table() {
        assert_eq!(Profile::of(Speed::Normal), Profile::NORMAL);
        assert_eq!(Profile::of(Speed::Fast), Profile::FAST);
        assert_eq!(Profile::of(Speed::Turbo), Profile::TURBO);
        // O turbo é o perfil no limite: o hold já está no piso de um frame.
        assert_eq!(Profile::TURBO.hold, MIN_HOLD_MS);
    }

    #[test]
    fn sequence_follows_the_reference_order() {
        let mut sink = Recorder::new();
        let seq = sequence(&[Dir::Up, Dir::Left], Speed::Normal, false);

        assert_eq!(
            run_sequence(&mut sink, &seq, Jitter::Off, &mut ()),
            Outcome::Completed
        );

        // Modificador segurado do começo ao fim, uma direção por vez dentro dele.
        assert_eq!(
            sink.key_events(),
            vec![
                down(0x1D), // LCtrl
                down(0x11), // W
                up(0x11),
                down(0x1E), // A
                up(0x1E),
                up(0x1D),
            ]
        );
        assert_eq!(
            sink.wait_ms(),
            vec![
                (Phase::Lead, 100),
                (Phase::Hold, 34),
                (Phase::Gap, 20),
                (Phase::Hold, 34),
                (Phase::Gap, 20),
                (Phase::Tail, 50),
            ]
        );
    }

    #[test]
    fn arrow_mode_sends_the_extended_scancodes() {
        let mut sink = Recorder::new();
        let seq = sequence(
            &[Dir::Up, Dir::Down, Dir::Left, Dir::Right],
            Speed::Turbo,
            true,
        );
        run_sequence(&mut sink, &seq, Jitter::Off, &mut ());

        let presses: Vec<Scan> = sink
            .key_events()
            .into_iter()
            .filter(|event| !event.up && event.scan != CTRL)
            .map(|event| event.scan)
            .collect();
        assert_eq!(
            presses,
            vec![
                keys::ARROWS[0],
                keys::ARROWS[1],
                keys::ARROWS[2],
                keys::ARROWS[3]
            ]
        );
        assert!(presses.iter().all(|scan| scan.extended));
    }

    #[test]
    fn turbo_waits_come_from_the_turbo_profile() {
        let mut sink = Recorder::new();
        run_sequence(
            &mut sink,
            &sequence(&[Dir::Right], Speed::Turbo, false),
            Jitter::Off,
            &mut (),
        );
        assert_eq!(
            sink.wait_ms(),
            vec![
                (Phase::Lead, 50),
                (Phase::Hold, 20),
                (Phase::Gap, 6),
                (Phase::Tail, 30),
            ]
        );
    }

    /// Aborta na direção de índice `at`.
    struct AbortAfter {
        at: usize,
        seen: usize,
        started: bool,
    }

    impl Hooks for AbortAfter {
        fn aborted(&mut self) -> bool {
            let abort = self.seen >= self.at;
            self.seen += 1;
            abort
        }

        fn started(&mut self) {
            self.started = true;
        }
    }

    #[test]
    fn losing_focus_stops_the_sequence_and_releases_every_key() {
        let mut sink = Recorder::new();
        let mut hooks = AbortAfter {
            at: 1,
            seen: 0,
            started: false,
        };
        let seq = sequence(&[Dir::Up, Dir::Down, Dir::Left], Speed::Fast, false);

        assert_eq!(
            run_sequence(&mut sink, &seq, Jitter::Off, &mut hooks),
            Outcome::Aborted
        );
        assert!(
            hooks.started,
            "o aviso de disparo sai antes do primeiro passo"
        );

        // A segunda direção nunca foi pressionada e o modificador saiu solto.
        assert_eq!(
            sink.key_events(),
            vec![down(0x1D), down(0x11), up(0x11), up(0x1D)]
        );
    }

    #[test]
    fn a_key_held_when_the_sequence_dies_is_released_in_reverse_order() {
        let mut sink = Recorder::new();

        {
            let mut held = Held::new(&mut sink);
            held.press(CTRL);
            held.press(keys::WASD[Dir::Up.index()]);
            // Sem release explícito: só o guard desfaz.
        }

        assert_eq!(
            sink.key_events(),
            vec![down(0x1D), down(0x11), up(0x11), up(0x1D)]
        );
    }

    #[test]
    fn hold_never_falls_below_a_frame_even_with_the_worst_jitter() {
        let mut offset = -JITTER_MS;
        while offset <= JITTER_MS {
            for profile in [Profile::NORMAL, Profile::FAST, Profile::TURBO] {
                let hold = jittered_ms(profile.hold, MIN_HOLD_MS, offset);
                assert!(
                    hold >= f64::from(MIN_HOLD_MS),
                    "hold {hold} com offset {offset}"
                );
            }
            offset += 0.25;
        }
    }

    #[test]
    fn jitter_only_moves_an_interval_five_milliseconds_each_way() {
        // O piso não interfere quando o valor base já está bem acima dele.
        assert_eq!(jittered_ms(34, MIN_HOLD_MS, -JITTER_MS), 29.0);
        assert_eq!(jittered_ms(34, MIN_HOLD_MS, JITTER_MS), 39.0);
        assert_eq!(jittered_ms(34, MIN_HOLD_MS, 0.0), 34.0);
        // E esperas curtas nunca chegam a zero.
        assert_eq!(jittered_ms(6, MIN_WAIT_MS, -JITTER_MS), 1.0);

        let drawn = Jitter::Humanized.offset_ms();
        assert!((-JITTER_MS..JITTER_MS).contains(&drawn), "offset {drawn}");
        assert_eq!(Jitter::Off.offset_ms(), 0.0);
    }

    fn engine_shared() -> (Arc<Shared>, Receivers) {
        let (shared, receivers) = Shared::new(Settings::default(), Slots::default());
        shared.set_game_focused(true);
        (shared, receivers)
    }

    fn run_cmd(slot: usize) -> EngineCmd {
        EngineCmd::Run {
            codex: [Dir::Up, Dir::Down].into(),
            modifier: CTRL,
            use_arrows: false,
            speed: Speed::Turbo,
            slot,
            support: false,
        }
    }

    #[test]
    fn a_run_notifies_the_ui_only_after_the_modifier_is_down() {
        let (shared, rx) = engine_shared();
        let mut sink = Recorder::new();

        let outcome = handle(&shared, &rx.engine, &mut sink, run_cmd(2));

        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(
            rx.ui.try_recv().unwrap(),
            UiEvent::MacroTriggered {
                slot: 2,
                support: false
            }
        );
        assert_eq!(
            rx.ui.try_recv().unwrap(),
            UiEvent::MacroStatus {
                slot: 2,
                support: false,
                running: true
            }
        );
        assert_eq!(
            rx.ui.try_recv().unwrap(),
            UiEvent::MacroStatus {
                slot: 2,
                support: false,
                running: false
            }
        );
        // O primeiro evento de teclado precede o primeiro aviso: o instante
        // virtual do press do modificador é zero.
        assert_eq!(sink.events[0].0, Duration::ZERO);
        assert!(!shared.macro_running.load(Ordering::Acquire));
    }

    #[test]
    fn a_second_shortcut_while_running_is_rejected_not_queued() {
        let (shared, rx) = engine_shared();
        let mut sink = Recorder::new();
        shared.macro_running.store(true, Ordering::Release);

        let outcome = handle(&shared, &rx.engine, &mut sink, run_cmd(1));

        assert_eq!(outcome, Outcome::Aborted);
        assert!(
            sink.events.is_empty(),
            "nenhuma tecla sai com o motor ocupado"
        );
        assert_eq!(
            rx.ui.try_recv().unwrap(),
            UiEvent::MacroBlocked {
                slot: 1,
                support: false
            }
        );
        assert_eq!(
            rx.overlay.try_recv().unwrap(),
            OverlayCmd::Flash {
                slot: 1,
                support: false,
                kind: FlashKind::Blocked
            }
        );
    }

    #[test]
    fn shortcuts_that_arrived_during_a_run_are_dropped_with_feedback() {
        let (shared, rx) = engine_shared();
        let mut sink = Recorder::new();

        // Chegaram enquanto a sequência do slot 0 rodava.
        shared.send_engine(run_cmd(1));
        shared.send_engine(run_cmd(3));
        handle(&shared, &rx.engine, &mut sink, run_cmd(0));

        assert!(rx.engine.is_empty(), "a fila não sobrevive à execução");
        let blocked: Vec<UiEvent> = rx
            .ui
            .try_iter()
            .filter(|event| matches!(event, UiEvent::MacroBlocked { .. }))
            .collect();
        assert_eq!(
            blocked,
            vec![
                UiEvent::MacroBlocked {
                    slot: 1,
                    support: false
                },
                UiEvent::MacroBlocked {
                    slot: 3,
                    support: false
                }
            ]
        );
    }

    #[test]
    fn focus_lost_before_the_engine_wakes_up_fires_nothing() {
        let (shared, rx) = engine_shared();
        let mut sink = Recorder::new();
        shared.set_game_focused(false);

        let outcome = handle(&shared, &rx.engine, &mut sink, run_cmd(0));

        assert_eq!(outcome, Outcome::Aborted);
        assert!(sink.events.is_empty());
        assert!(rx.ui.is_empty(), "desistir em silêncio, como a v1");
        assert!(!shared.macro_running.load(Ordering::Acquire));
    }
}
