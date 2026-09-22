//! Modo debug: o que o app viu e fez em cada disparo, gravado para quem for
//! investigar uma falha num PC que não é o nosso.
//!
//! O app sabe que mandou as teclas, mas não enxerga o jogo: se o estratagema
//! saiu, só quem joga vê. O que dá para registrar é tudo o que cerca o envio: o
//! tempo real de cada tecla contra o planejado, o que o jogador segurava na
//! hora, quem estava em foco, se o Windows recusou alguma tecla e o retrato do
//! PC. Cruzando isso com o relato de quem testou, a causa aparece.
//!
//! Três regras:
//!
//! 1. **Desligado não custa nada.** O caminho quente só lê uma flag atômica.
//! 2. **Ligado, mexe o mínimo no timing que mede.** O engine carimba instantes
//!    e avisa o painel do overlay depois de cada `SendInput`: microssegundos que
//!    só alongam a espera seguinte, nunca a encurtam. Montar o registro vem
//!    depois da última tecla, e serializar e gravar em disco é trabalho da
//!    thread `diag`.
//! 3. **Não é keylogger.** Entram as teclas que o próprio macro manda, o atalho
//!    que disparou e as teclas de movimento e modificadores seguradas no
//!    instante do disparo. Título de janela, só a do jogo ou a do app; das
//!    outras, o executável.
//!
//! O registro vai para `debug.jsonl` na pasta de configuração, uma linha JSON
//! por evento, e o botão "Exportar relatório" junta tudo num arquivo só
//! ([`report`]).

pub mod env;
pub mod report;
pub mod typing;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::Serialize;

use crate::data::{Dir, GameData, SUPPORT_STRATS};
use crate::engine::{InputSink, KeyEvent, Phase, Sequence};
use crate::keys::{self, Scan};
use crate::settings::{Language, Speed, SLOT_COUNT, SUPPORT_COUNT};
use crate::shared::{DebugKeys, OverlayCmd, Shared, UiEvent};
use crate::util;

/// Registro corrente e o anterior, para onde ele gira ao passar do limite.
pub const LOG_FILE: &str = "debug.jsonl";
pub const OLD_LOG_FILE: &str = "debug.old.jsonl";
/// Uns 4 MB são milhares de disparos: sessões de jogo inteiras, sem encher o
/// disco de quem esqueceu o modo ligado.
const LOG_LIMIT: u64 = 4 * 1024 * 1024;

/// Id anônimo do PC, para agrupar relatórios da mesma pessoa.
const ID_FILE: &str = "debug-id.txt";

// --- Registros ---

/// Janela em primeiro plano, do jeito que o registro a guarda.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WindowInfo {
    /// `game`, `app`, `overlay` ou `other`: a classificação que arma os atalhos.
    pub class: &'static str,
    /// Executável dono da janela, sem caminho.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// Só da janela do jogo ou do app: título de terceiros não sai do PC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl WindowInfo {
    /// O executável é o do jogo, classificado ou não como tal.
    pub fn is_game_process(&self) -> bool {
        self.exe
            .as_deref()
            .is_some_and(|exe| exe.eq_ignore_ascii_case(env::GAME_EXE))
    }
}

/// Uma tecla que o engine mandou.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Stamp {
    /// Milissegundos desde o press do modificador.
    #[serde(rename = "t", serialize_with = "one_decimal")]
    pub at_ms: f32,
    #[serde(rename = "k")]
    pub key: &'static str,
    pub up: bool,
    /// `false` é o Windows recusando a injeção.
    pub ok: bool,
}

/// Tempos reais de uma sequência, tirados dos carimbos.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    #[serde(serialize_with = "opt_one_decimal")]
    pub lead_ms: Option<f32>,
    /// Quanto cada direção ficou segurada.
    #[serde(serialize_with = "all_one_decimal")]
    pub holds_ms: Vec<f32>,
    /// Entre soltar uma direção e apertar a seguinte.
    #[serde(serialize_with = "all_one_decimal")]
    pub gaps_ms: Vec<f32>,
    /// Da última direção solta até soltar o modificador.
    #[serde(serialize_with = "opt_one_decimal")]
    pub tail_ms: Option<f32>,
}

impl Timing {
    /// Lê os intervalos de uma sequência: modificador, pares de direção e
    /// modificador de novo. Uma sequência abortada rende só os pares completos.
    pub fn of(stamps: &[Stamp], modifier: &str) -> Timing {
        let mut timing = Timing::default();
        let modifier_down = stamps
            .iter()
            .find(|stamp| stamp.key == modifier && !stamp.up)
            .map(|stamp| stamp.at_ms);
        let (mut pressed, mut released): (Option<f32>, Option<f32>) = (None, None);

        for stamp in stamps {
            if stamp.key == modifier {
                if stamp.up {
                    timing.tail_ms = released.map(|at| stamp.at_ms - at);
                }
                continue;
            }
            if !stamp.up {
                match released {
                    Some(at) => timing.gaps_ms.push(stamp.at_ms - at),
                    None => timing.lead_ms = modifier_down.map(|at| stamp.at_ms - at),
                }
                pressed = Some(stamp.at_ms);
            } else if let Some(at) = pressed.take() {
                timing.holds_ms.push(stamp.at_ms - at);
                released = Some(stamp.at_ms);
            }
        }
        timing
    }
}

/// Como uma chamada de atalho terminou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    /// Todas as teclas saíram.
    Completed,
    /// O foco saiu do jogo no meio; o guard soltou o que estava preso.
    Aborted,
    /// Outra sequência ainda rodava: descartada com a piscada vermelha.
    Blocked,
    /// O foco caiu entre o atalho e o engine acordar: nada saiu.
    Unfocused,
}

/// Uma chamada de atalho, do jeito que o registro a guarda.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub at: String,
    pub slot: usize,
    pub support: bool,
    /// Preenchido pela thread `diag`, que tem os dados do jogo.
    pub stratagem: Option<String>,
    pub codex: Vec<Dir>,
    pub speed: Speed,
    pub arrows: bool,
    pub modifier: &'static str,
    pub outcome: RunOutcome,
    /// Teclas de movimento e modificadores que o jogador segurava no instante
    /// do disparo. Um W físico segurado briga com o W que o macro manda.
    pub held: Vec<&'static str>,
    pub window: WindowInfo,
    pub timing: Timing,
    /// Teclas que o Windows recusou.
    pub rejected: usize,
    pub keys: Vec<Stamp>,
}

impl Run {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot: usize,
        support: bool,
        codex: &[Dir],
        speed: Speed,
        arrows: bool,
        modifier: Scan,
        outcome: RunOutcome,
        held: Vec<&'static str>,
        window: WindowInfo,
        keys: Vec<Stamp>,
    ) -> Run {
        let modifier = keys::scan_label(modifier);
        Run {
            at: util::iso8601_now(),
            slot,
            support,
            stratagem: None,
            codex: codex.to_vec(),
            speed,
            arrows,
            modifier,
            outcome,
            held,
            window,
            timing: Timing::of(&keys, modifier),
            rejected: keys.iter().filter(|stamp| !stamp.ok).count(),
            keys,
        }
    }
}

/// Um atalho apertado com o jogo na frente, mas com os macros desarmados. É o
/// "apertei e não veio nada" que nenhum outro registro pegaria.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ignored {
    pub at: String,
    pub key: &'static str,
    pub slot: usize,
    pub support: bool,
    /// `unfocused` (o app não reconheceu a janela como o jogo) ou `recording`
    /// (a aba de configurações esperava um atalho).
    pub reason: &'static str,
    pub window: WindowInfo,
}

/// Troca de foco que arma ou desarma os atalhos.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Focus {
    pub at: String,
    pub armed: bool,
    pub window: WindowInfo,
}

/// Um teste de digitação terminado.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingRecord {
    pub at: String,
    pub speed: Speed,
    pub arrows: bool,
    pub summary: typing::Summary,
    pub runs: Vec<typing::RunCheck>,
}

/// Abertura de sessão: retrato do PC e das preferências.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub at: String,
    pub id: String,
    pub environment: env::Environment,
    pub settings: SettingsSummary,
}

/// As preferências que mexem no macro, com os slots já pelo nome.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSummary {
    pub speed: Speed,
    pub modifier: String,
    pub arrows: bool,
    pub overlay: bool,
    pub persistent_hud: bool,
    pub language: Language,
    pub shortcuts: [Option<String>; SLOT_COUNT],
    pub support_shortcuts: [Option<String>; SUPPORT_COUNT],
    pub slots: Vec<Option<String>>,
}

impl SettingsSummary {
    pub fn of(shared: &Shared, data: &GameData) -> SettingsSummary {
        let settings = shared.settings_snapshot();
        SettingsSummary {
            speed: settings.macro_speed,
            modifier: settings.modifier_key.clone(),
            arrows: settings.use_arrows,
            overlay: settings.enable_overlay,
            persistent_hud: settings.always_show_slots,
            language: settings.language,
            shortcuts: settings.shortcuts.clone(),
            support_shortcuts: settings.support_shortcuts.clone(),
            slots: shared
                .slots()
                .iter()
                .map(|id| id.and_then(|id| data.by_id(id)).map(|s| s.nome.clone()))
                .collect(),
        }
    }
}

/// O que as outras threads mandam para a thread `diag`.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Abre uma sessão: a thread monta o retrato, que é lento demais para
    /// quem pede.
    Session,
    Run(Box<Run>),
    Ignored(Ignored),
    Focus(Focus),
    Typing(Box<TypingRecord>),
}

/// Uma linha do `debug.jsonl`.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Entry<'a> {
    Session(&'a Session),
    Run(&'a Run),
    Ignored(&'a Ignored),
    Focus(&'a Focus),
    Typing(&'a TypingRecord),
}

// --- Estatísticas ---

/// Mínimo, média e máximo de uma medida, acumulados sem guardar a amostra.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Spread {
    pub count: u32,
    #[serde(serialize_with = "one_decimal")]
    pub min: f32,
    #[serde(serialize_with = "one_decimal")]
    pub mean: f32,
    #[serde(serialize_with = "one_decimal")]
    pub max: f32,
}

impl Spread {
    pub fn add(&mut self, value: f32) {
        if self.count == 0 {
            self.min = value;
            self.max = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
        }
        self.mean += (value - self.mean) / (self.count + 1) as f32;
        self.count += 1;
    }
}

/// Resumo da sessão de debug, para o painel e o relatório.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Quando o modo foi ligado.
    pub since: Option<String>,
    pub runs: u32,
    pub completed: u32,
    pub aborted: u32,
    pub blocked: u32,
    pub unfocused: u32,
    pub ignored: u32,
    pub rejected_keys: u32,
    pub hold: Spread,
    pub gap: Spread,
    /// Chamadas com tecla de movimento segurada no disparo.
    pub with_held_keys: u32,
    /// A última chamada, para o painel mostrar sem reler o arquivo.
    pub last: Option<LastRun>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    pub stratagem: Option<String>,
    pub outcome: RunOutcome,
    pub keys: usize,
    #[serde(serialize_with = "opt_one_decimal")]
    pub min_hold_ms: Option<f32>,
}

impl Stats {
    pub fn add_run(&mut self, run: &Run) {
        self.runs += 1;
        match run.outcome {
            RunOutcome::Completed => self.completed += 1,
            RunOutcome::Aborted => self.aborted += 1,
            RunOutcome::Blocked => self.blocked += 1,
            RunOutcome::Unfocused => self.unfocused += 1,
        }
        self.rejected_keys += run.rejected as u32;
        for hold in &run.timing.holds_ms {
            self.hold.add(*hold);
        }
        for gap in &run.timing.gaps_ms {
            self.gap.add(*gap);
        }
        if run.held.iter().any(|key| MOVEMENT_KEYS.contains(key)) {
            self.with_held_keys += 1;
        }
        self.last = Some(LastRun {
            stratagem: run.stratagem.clone(),
            outcome: run.outcome,
            keys: run.timing.holds_ms.len(),
            min_hold_ms: run.timing.holds_ms.iter().copied().reduce(f32::min),
        });
    }
}

/// Saúde do hook de teclado: se ele parou de ver teclas com o jogador digitando,
/// o Windows o derrubou.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookHealth {
    /// Teclas físicas vistas desde que o modo foi ligado (só a contagem).
    pub keys: u64,
    /// Segundos desde a última tecla física vista.
    #[serde(serialize_with = "opt_one_decimal")]
    pub last_key_age_s: Option<f32>,
    /// A chamada mais lenta, medida dentro do callback. É um piso do que o
    /// Windows cronometra contra o `hooksTimeoutMs`: a espera da tecla na fila
    /// da thread de hooks, ocupada com outra coisa, fica de fora.
    pub max_callback_us: u32,
}

// --- Estado compartilhado ---

/// O lado do modo debug que vive no [`Shared`].
pub struct Diag {
    enabled: AtomicBool,
    tx: Sender<Event>,
    stats: Mutex<Stats>,
    /// Última janela em primeiro plano, escrita pela thread de hooks.
    window: Mutex<WindowInfo>,
    hook_keys: AtomicU64,
    hook_last_ms: AtomicU64,
    hook_max_us: AtomicU32,
}

impl Diag {
    pub fn new(enabled: bool) -> (Diag, Receiver<Event>) {
        let (tx, rx) = unbounded();
        let diag = Diag {
            enabled: AtomicBool::new(enabled),
            tx,
            stats: Mutex::new(Stats::default()),
            window: Mutex::new(WindowInfo::default()),
            hook_keys: AtomicU64::new(0),
            hook_last_ms: AtomicU64::new(0),
            hook_max_us: AtomicU32::new(0),
        };
        (diag, rx)
    }

    /// É a única leitura que o caminho quente faz com o modo desligado.
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Liga ou desliga. Ligar abre uma sessão nova.
    pub fn set_enabled(&self, on: bool) {
        let was = self.enabled.swap(on, Ordering::Relaxed);
        if on && !was {
            self.begin();
        }
    }

    /// Zera as estatísticas e pede o retrato de abertura. Chamado ao ligar o
    /// modo e no boot, quando ele já vem ligado.
    pub fn begin(&self) {
        *lock(&self.stats) = Stats {
            since: Some(util::iso8601_now()),
            ..Stats::default()
        };
        self.hook_keys.store(0, Ordering::Relaxed);
        self.hook_last_ms.store(0, Ordering::Relaxed);
        self.hook_max_us.store(0, Ordering::Relaxed);
        let _ = self.tx.send(Event::Session);
    }

    /// Manda um evento para o registro. Com o modo desligado, some.
    pub fn record(&self, event: Event) {
        if self.enabled() {
            let _ = self.tx.send(event);
        }
    }

    pub fn stats(&self) -> Stats {
        lock(&self.stats).clone()
    }

    fn update_stats(&self, update: impl FnOnce(&mut Stats)) {
        update(&mut lock(&self.stats));
    }

    pub fn window(&self) -> WindowInfo {
        lock(&self.window).clone()
    }

    pub fn set_window(&self, window: WindowInfo) {
        *lock(&self.window) = window;
    }

    /// Conta uma tecla física vista pelo hook e quanto o callback levou.
    pub fn hook_key(&self, took: Duration) {
        self.hook_keys.fetch_add(1, Ordering::Relaxed);
        self.hook_last_ms
            .store(util::epoch_millis(), Ordering::Relaxed);
        let micros = took.as_micros().min(u128::from(u32::MAX)) as u32;
        self.hook_max_us.fetch_max(micros, Ordering::Relaxed);
    }

    pub fn hook_health(&self) -> HookHealth {
        let last = self.hook_last_ms.load(Ordering::Relaxed);
        HookHealth {
            keys: self.hook_keys.load(Ordering::Relaxed),
            last_key_age_s: (last != 0)
                .then(|| util::epoch_millis().saturating_sub(last) as f32 / 1_000.0),
            max_callback_us: self.hook_max_us.load(Ordering::Relaxed),
        }
    }
}

/// Os locks guardam dado puro: um panic com o lock na mão não invalida a cópia.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

// --- Medição dentro do engine ---

/// Sink do modo debug: carimba cada tecla enviada e repassa tudo ao sink real.
///
/// O carimbo sai antes do envio, como na bancada de timing: é o instante em que
/// a tecla é despachada, e o intervalo entre dois carimbos é o intervalo real
/// entre dois `SendInput`. Quanto disso chega ao jogo é o que o registro vem
/// ajudar a descobrir.
pub struct Probe<'a, S: InputSink> {
    inner: &'a mut S,
    origin: Option<Instant>,
    stamps: Vec<Stamp>,
    live: Option<Live<'a>>,
}

/// O painel de teclas do overlay, acompanhando a sequência ao vivo.
struct Live<'a> {
    shared: &'a Shared,
    modifier: Scan,
    /// Direção atual: o passo que o painel acende.
    step: usize,
    /// O aviso de início, guardado até a primeira tecla sair.
    start: Option<DebugKeys>,
}

impl<'a, S: InputSink> Probe<'a, S> {
    pub fn new(inner: &'a mut S) -> Probe<'a, S> {
        Probe {
            inner,
            origin: None,
            // Modificador e até oito direções, ida e volta: sem realocar no meio.
            stamps: Vec::with_capacity(20),
            live: None,
        }
    }

    /// Acende o painel de teclas do overlay a cada tecla. Cada aviso sai
    /// depois do `SendInput` dela: o painel pode atrasar, a tecla não.
    pub fn live(
        mut self,
        shared: &'a Shared,
        sequence: &Sequence,
        slot: usize,
        support: bool,
    ) -> Probe<'a, S> {
        self.live = Some(Live {
            shared,
            modifier: sequence.modifier,
            step: 0,
            start: Some(DebugKeys::Start {
                slot,
                support,
                modifier: keys::scan_label(sequence.modifier),
                codex: Arc::clone(&sequence.codex),
                speed: sequence.speed,
            }),
        });
        self
    }

    /// Fecha a medição: avisa o painel de como terminou e devolve os carimbos.
    pub fn finish(self, outcome: RunOutcome) -> Vec<Stamp> {
        if let Some(live) = &self.live {
            let modifier = keys::scan_label(live.modifier);
            live.shared
                .send_overlay(OverlayCmd::DebugKeys(DebugKeys::End {
                    outcome,
                    holds_ms: Timing::of(&self.stamps, modifier).holds_ms,
                }));
        }
        self.stamps
    }
}

impl<S: InputSink> InputSink for Probe<'_, S> {
    fn send(&mut self, event: KeyEvent) -> bool {
        let now = Instant::now();
        let origin = *self.origin.get_or_insert(now);
        let ok = self.inner.send(event);
        self.stamps.push(Stamp {
            at_ms: now.duration_since(origin).as_secs_f32() * 1_000.0,
            key: keys::scan_label(event.scan),
            up: event.up,
            ok,
        });
        if let Some(live) = &mut self.live {
            if let Some(start) = live.start.take() {
                live.shared.send_overlay(OverlayCmd::DebugKeys(start));
            }
            let step = if event.scan == live.modifier {
                0
            } else {
                if !event.up {
                    live.step += 1;
                }
                live.step
            };
            live.shared
                .send_overlay(OverlayCmd::DebugKeys(DebugKeys::Key {
                    step,
                    up: event.up,
                    ok,
                }));
        }
        ok
    }

    fn wait(&mut self, phase: Phase, duration: Duration) {
        self.inner.wait(phase, duration);
    }
}

/// Teclas que o registro olha no disparo: movimento, setas, os modificadores
/// (Shift de correr, e as teclas de menu de estratagema) e o espaço.
const WATCHED_KEYS: [(&str, u16); 12] = [
    ("W", 0x57),
    ("A", 0x41),
    ("S", 0x53),
    ("D", 0x44),
    ("UP", 0x26),
    ("DOWN", 0x28),
    ("LEFT", 0x25),
    ("RIGHT", 0x27),
    ("SHIFT", 0x10),
    ("CTRL", 0x11),
    ("ALT", 0x12),
    ("SPACE", 0x20),
];

/// As que andam com o personagem: são as que brigam com as do macro.
const MOVEMENT_KEYS: [&str; 8] = ["W", "A", "S", "D", "UP", "DOWN", "LEFT", "RIGHT"];

/// Quais de [`WATCHED_KEYS`] estão pressionadas agora. Doze leituras de estado
/// do teclado, microssegundos no total: o engine chama antes do modificador,
/// quando o que está seguro ainda é só do jogador.
#[cfg(windows)]
pub fn held_keys() -> Vec<&'static str> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    WATCHED_KEYS
        .iter()
        // SAFETY: leitura de estado global do teclado, sem ponteiros. O bit
        // alto (sinal do i16) marca "pressionada agora".
        .filter(|(_, vk)| unsafe { GetAsyncKeyState(i32::from(*vk)) } < 0)
        .map(|(name, _)| *name)
        .collect()
}

#[cfg(not(windows))]
pub fn held_keys() -> Vec<&'static str> {
    let _ = WATCHED_KEYS;
    Vec::new()
}

// --- Thread de gravação ---

/// Sobe a thread que monta os registros e grava o `debug.jsonl`. Ela vive
/// enquanto o processo viver, parada no canal quando o modo está desligado.
pub fn spawn(
    shared: Arc<Shared>,
    data: Arc<GameData>,
    rx: Receiver<Event>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("diag".to_string())
        .spawn(move || {
            let mut log = Log::new(util::config_path(LOG_FILE), util::config_path(OLD_LOG_FILE));
            while let Ok(event) = rx.recv() {
                write_event(&shared, &data, &mut log, event);
            }
        })
}

fn write_event(shared: &Shared, data: &GameData, log: &mut Log, event: Event) {
    let line = match event {
        Event::Session => {
            let session = session(shared, data);
            serialize(&Entry::Session(&session))
        }
        Event::Run(mut run) => {
            run.stratagem = stratagem_name(shared, data, run.slot, run.support);
            shared.diag.update_stats(|stats| stats.add_run(&run));
            serialize(&Entry::Run(&run))
        }
        Event::Ignored(ignored) => {
            shared.diag.update_stats(|stats| stats.ignored += 1);
            serialize(&Entry::Ignored(&ignored))
        }
        Event::Focus(focus) => serialize(&Entry::Focus(&focus)),
        Event::Typing(typing) => serialize(&Entry::Typing(&typing)),
    };
    if let Some(line) = line {
        if let Err(err) = log.append(&line) {
            log::warn!("registro de debug não foi gravado: {err}");
        }
    }
    shared.send_ui(UiEvent::DiagUpdated);
}

/// Retrato atual do PC e das preferências.
pub fn session(shared: &Shared, data: &GameData) -> Session {
    Session {
        at: util::iso8601_now(),
        id: machine_id(),
        environment: env::snapshot(),
        settings: SettingsSummary::of(shared, data),
    }
}

fn stratagem_name(shared: &Shared, data: &GameData, slot: usize, support: bool) -> Option<String> {
    if support {
        return SUPPORT_STRATS.get(slot).map(|strat| strat.nome.to_string());
    }
    let id = shared.slots().get(slot).copied().flatten()?;
    data.by_id(id).map(|strat| strat.nome.clone())
}

fn serialize(entry: &Entry) -> Option<Vec<u8>> {
    match serde_json::to_vec(entry) {
        Ok(mut line) => {
            line.push(b'\n');
            Some(line)
        }
        Err(err) => {
            log::warn!("registro de debug não serializou: {err}");
            None
        }
    }
}

/// Id anônimo e estável deste PC: doze dígitos hexadecimais sorteados na
/// primeira vez. Não sai de nada do usuário; só agrupa relatórios.
pub fn machine_id() -> String {
    let path = util::config_path(ID_FILE);
    if let Ok(text) = fs::read_to_string(&path) {
        let id = text.trim();
        if id.len() == 12 && id.chars().all(|c| c.is_ascii_hexdigit()) {
            return id.to_string();
        }
    }
    let id = format!("{:012x}", rand::random::<u64>() & 0xFFFF_FFFF_FFFF);
    if let Err(err) = util::write_atomic(&path, id.as_bytes()) {
        log::warn!("id do debug não foi gravado: {err:#}");
    }
    id
}

/// O `debug.jsonl` aberto para acréscimo, girando para o `.old` no limite.
struct Log {
    path: PathBuf,
    old: PathBuf,
    file: Option<fs::File>,
    size: u64,
}

impl Log {
    fn new(path: PathBuf, old: PathBuf) -> Log {
        Log {
            path,
            old,
            file: None,
            size: 0,
        }
    }

    fn append(&mut self, line: &[u8]) -> std::io::Result<()> {
        if self.file.is_some() && self.size + line.len() as u64 > LOG_LIMIT {
            self.file = None;
            fs::rename(&self.path, &self.old)?;
        }
        if self.file.is_none() {
            if let Some(dir) = self.path.parent() {
                fs::create_dir_all(dir)?;
            }
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.size = file.metadata()?.len();
            self.file = Some(file);
        }
        if let Some(file) = &mut self.file {
            file.write_all(line)?;
            self.size += line.len() as u64;
        }
        Ok(())
    }
}

// --- Serialização compacta ---

// Microssegundos de float no JSON só engordam o arquivo: um décimo de
// milissegundo é a resolução que interessa para comparar com um quadro.

fn round1(value: f32) -> f64 {
    (f64::from(value) * 10.0).round() / 10.0
}

fn one_decimal<S: serde::Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_f64(round1(*value))
}

fn opt_one_decimal<S: serde::Serializer>(
    value: &Option<f32>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.serialize_some(&round1(*value)),
        None => serializer.serialize_none(),
    }
}

fn all_one_decimal<S: serde::Serializer>(values: &[f32], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(values.iter().map(|value| round1(*value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{run_sequence, Jitter, Recorder, Sequence};

    fn stamp(at_ms: f32, key: &'static str, up: bool) -> Stamp {
        Stamp {
            at_ms,
            key,
            up,
            ok: true,
        }
    }

    #[test]
    fn timing_reads_lead_holds_gaps_and_tail() {
        let stamps = [
            stamp(0.0, "CTRL", false),
            stamp(100.0, "W", false),
            stamp(134.0, "W", true),
            stamp(154.0, "S", false),
            stamp(190.0, "S", true),
            stamp(260.0, "CTRL", true),
        ];
        assert_eq!(
            Timing::of(&stamps, "CTRL"),
            Timing {
                lead_ms: Some(100.0),
                holds_ms: vec![34.0, 36.0],
                gaps_ms: vec![20.0],
                tail_ms: Some(70.0),
            }
        );
    }

    #[test]
    fn an_aborted_sequence_keeps_only_the_complete_pairs() {
        // O guard soltou a direção e o modificador juntos, na ordem inversa.
        let stamps = [
            stamp(0.0, "CTRL", false),
            stamp(50.0, "UP", false),
            stamp(70.0, "UP", true),
            stamp(76.0, "DOWN", false),
            stamp(80.0, "DOWN", true),
            stamp(80.1, "CTRL", true),
        ];
        let timing = Timing::of(&stamps, "CTRL");
        assert_eq!(timing.holds_ms, vec![20.0, 4.0]);
        assert_eq!(timing.gaps_ms, vec![6.0]);
        assert!(timing.tail_ms.is_some());
        assert_eq!(Timing::of(&[], "CTRL"), Timing::default());
    }

    #[test]
    fn the_probe_stamps_every_key_the_engine_sends() {
        let mut recorder = Recorder::new();
        let mut probe = Probe::new(&mut recorder);
        let sequence = Sequence {
            codex: [Dir::Up, Dir::Down, Dir::Down].into(),
            modifier: keys::modifier_scan("LeftControl"),
            use_arrows: true,
            speed: Speed::Low,
        };
        run_sequence(&mut probe, &sequence, Jitter::Off, &mut ());
        let stamps = probe.finish(RunOutcome::Completed);

        let keys: Vec<_> = stamps.iter().map(|s| (s.key, s.up)).collect();
        assert_eq!(
            keys,
            [
                ("CTRL", false),
                ("UP", false),
                ("UP", true),
                ("DOWN", false),
                ("DOWN", true),
                ("DOWN", false),
                ("DOWN", true),
                ("CTRL", true),
            ]
        );
        assert_eq!(stamps[0].at_ms, 0.0, "o relógio começa no modificador");
        assert!(stamps.iter().all(|stamp| stamp.ok));
        // O recorder repassou tudo para o sink de baixo.
        assert_eq!(recorder.events.len(), 8);
    }

    #[test]
    fn the_live_probe_walks_the_overlay_panel_step_by_step() {
        let (shared, rx) = Shared::new(crate::settings::Settings::default(), [None; SLOT_COUNT]);
        let mut recorder = Recorder::new();
        let sequence = Sequence {
            codex: [Dir::Left, Dir::Left].into(),
            modifier: keys::modifier_scan("LeftAlt"),
            use_arrows: false,
            speed: Speed::Fast,
        };
        let mut probe = Probe::new(&mut recorder).live(&shared, &sequence, 2, false);
        run_sequence(&mut probe, &sequence, Jitter::Off, &mut ());
        probe.finish(RunOutcome::Completed);

        let cmds: Vec<DebugKeys> = rx
            .overlay
            .try_iter()
            .filter_map(|cmd| match cmd {
                OverlayCmd::DebugKeys(keys) => Some(keys),
                _ => None,
            })
            .collect();
        assert!(matches!(
            &cmds[0],
            DebugKeys::Start {
                slot: 2,
                modifier: "ALT",
                ..
            }
        ));
        let steps: Vec<(usize, bool)> = cmds
            .iter()
            .filter_map(|cmd| match cmd {
                DebugKeys::Key { step, up, .. } => Some((*step, *up)),
                _ => None,
            })
            .collect();
        // Duas direções iguais seguidas continuam sendo dois passos.
        assert_eq!(
            steps,
            [
                (0, false),
                (1, false),
                (1, true),
                (2, false),
                (2, true),
                (0, true)
            ]
        );
        assert!(matches!(
            cmds.last(),
            Some(DebugKeys::End { outcome: RunOutcome::Completed, holds_ms }) if holds_ms.len() == 2
        ));
    }

    #[test]
    fn a_run_counts_rejected_keys_and_feeds_the_stats() {
        let mut keys = vec![
            stamp(0.0, "CTRL", false),
            stamp(100.0, "W", false),
            stamp(130.0, "W", true),
            stamp(180.0, "CTRL", true),
        ];
        keys[1].ok = false;
        let run = Run::new(
            1,
            false,
            &[Dir::Up],
            Speed::Normal,
            false,
            keys::modifier_scan("LeftControl"),
            RunOutcome::Completed,
            vec!["W", "SHIFT"],
            WindowInfo::default(),
            keys,
        );
        assert_eq!(run.rejected, 1);
        assert_eq!(run.modifier, "CTRL");

        let mut stats = Stats::default();
        stats.add_run(&run);
        assert_eq!(
            (stats.runs, stats.completed, stats.rejected_keys),
            (1, 1, 1)
        );
        assert_eq!(stats.with_held_keys, 1, "W segurado no disparo");
        assert_eq!(stats.hold.count, 1);
        assert_eq!(stats.hold.min, 30.0);
        assert_eq!(stats.last.as_ref().unwrap().min_hold_ms, Some(30.0));
    }

    #[test]
    fn spread_keeps_min_mean_and_max_without_the_sample() {
        let mut spread = Spread::default();
        for value in [30.0, 50.0, 40.0] {
            spread.add(value);
        }
        assert_eq!((spread.count, spread.min, spread.max), (3, 30.0, 50.0));
        assert!((spread.mean - 40.0).abs() < 1e-4);
    }

    #[test]
    fn a_line_is_compact_json_with_its_type() {
        let run = Run::new(
            0,
            true,
            &[Dir::Up, Dir::Down],
            Speed::Turbo,
            true,
            keys::modifier_scan("LeftAlt"),
            RunOutcome::Blocked,
            Vec::new(),
            WindowInfo {
                class: "game",
                exe: Some("helldivers2.exe".into()),
                title: Some("HELLDIVERS™ 2".into()),
            },
            vec![stamp(12.345, "ALT", false)],
        );
        let line = serialize(&Entry::Run(&run)).unwrap();
        let text = String::from_utf8(line).unwrap();
        assert!(text.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["type"], "run");
        assert_eq!(value["outcome"], "blocked");
        assert_eq!(value["codex"], serde_json::json!(["UP", "DOWN"]));
        assert_eq!(value["speed"], "turbo");
        assert_eq!(value["keys"][0]["t"], 12.3);
        assert_eq!(value["window"]["class"], "game");
    }

    #[test]
    fn the_log_rotates_past_the_limit_and_keeps_appending() {
        let dir = std::env::temp_dir().join(format!("mh2-diag-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut log = Log::new(dir.join(LOG_FILE), dir.join(OLD_LOG_FILE));

        log.append(b"a\n").unwrap();
        log.append(b"b\n").unwrap();
        assert_eq!(fs::read_to_string(dir.join(LOG_FILE)).unwrap(), "a\nb\n");

        // Força o giro sem escrever 4 MB.
        log.size = LOG_LIMIT;
        log.append(b"c\n").unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(OLD_LOG_FILE)).unwrap(),
            "a\nb\n"
        );
        assert_eq!(fs::read_to_string(dir.join(LOG_FILE)).unwrap(), "c\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn events_are_dropped_while_disabled() {
        let (diag, rx) = Diag::new(false);
        diag.record(Event::Focus(Focus {
            at: String::new(),
            armed: true,
            window: WindowInfo::default(),
        }));
        assert!(rx.is_empty());

        diag.set_enabled(true);
        assert_eq!(rx.try_recv().unwrap(), Event::Session, "ligar abre sessão");
        assert!(diag.stats().since.is_some());
        diag.record(Event::Focus(Focus {
            at: String::new(),
            armed: false,
            window: WindowInfo::default(),
        }));
        assert!(matches!(rx.try_recv().unwrap(), Event::Focus(_)));

        // Religar com o modo já ligado não abre outra sessão.
        diag.set_enabled(true);
        assert!(rx.is_empty());
    }

    #[test]
    fn the_hook_health_tracks_count_age_and_the_slowest_call() {
        let (diag, _rx) = Diag::new(true);
        assert_eq!(diag.hook_health().last_key_age_s, None);
        diag.hook_key(Duration::from_micros(40));
        diag.hook_key(Duration::from_micros(900));
        diag.hook_key(Duration::from_micros(10));
        let health = diag.hook_health();
        assert_eq!(health.keys, 3);
        assert_eq!(health.max_callback_us, 900);
        assert!(health.last_key_age_s.is_some_and(|age| age < 5.0));
    }
}
