//! Teste de digitação: o engine digita sequências de verdade na janela do
//! próprio app, e a janela confere se cada tecla chegou, na ordem, e por quanto
//! tempo ficou segurada.
//!
//! Separa as duas metades do problema. Se o teste passa e o jogo falha, o
//! caminho até o Windows está bom e a causa mora no jogo (FPS, timing, tecla do
//! menu). Se o teste falha, algo no PC come ou reordena as teclas antes de
//! qualquer jogo: outro programa com hook, antivírus, software de periférico.
//!
//! Esta parte é lógica pura. Quem manda o engine digitar e entrega as teclas
//! recebidas é a janela principal.

use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;

use super::{RunOutcome, Spread, Stamp};
use crate::data::{Dir, SUPPORT_STRATS};
use crate::keys::Scan;
use crate::settings::Speed;

/// Rodadas por teste: metade com cada codex.
pub const RUNS: usize = 10;

/// Os dois codex do teste: o Reforço, que tem cinco direções diferentes em
/// sequência, e o Rearmar Eagle, que começa com duas iguais (↑↑). Direção
/// repetida é o caso em que um intervalo curto pode fundir duas teclas em uma.
fn plan() -> Vec<Arc<[Dir]>> {
    let reinforce: Arc<[Dir]> = SUPPORT_STRATS[0].codex.into();
    let rearm: Arc<[Dir]> = SUPPORT_STRATS[2].codex.into();
    (0..RUNS)
        .map(|run| {
            if run % 2 == 0 {
                Arc::clone(&reinforce)
            } else {
                Arc::clone(&rearm)
            }
        })
        .collect()
}

/// Uma tecla que chegou na janela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Received {
    pub key: &'static str,
    pub up: bool,
    pub at: Instant,
}

/// O veredito de uma rodada.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCheck {
    pub codex: Vec<Dir>,
    pub outcome: RunOutcome,
    /// Eventos que o engine mandou (descidas e subidas).
    pub sent: usize,
    /// Mandados que chegaram, na mesma ordem.
    pub matched: usize,
    pub missing: usize,
    /// Chegaram sem ter sido mandados: tecla física no meio do teste, ou
    /// repetição inventada por outro programa.
    pub extra: usize,
    /// Quanto cada direção ficou segurada, medido na chegada. É aproximado:
    /// inclui o atraso da fila de mensagens da janela.
    #[serde(serialize_with = "super::all_one_decimal")]
    pub received_holds_ms: Vec<f32>,
    #[serde(serialize_with = "super::all_one_decimal")]
    pub sent_holds_ms: Vec<f32>,
}

impl RunCheck {
    pub fn passed(&self) -> bool {
        self.outcome == RunOutcome::Completed && self.missing == 0 && self.extra == 0
    }
}

/// Compara o que foi mandado com o que chegou.
///
/// Casamento guloso na ordem: cada evento mandado procura o próximo recebido
/// igual a partir de onde o anterior parou. O que sobra dos dois lados é falta
/// ou excesso.
pub fn check(
    codex: &[Dir],
    outcome: RunOutcome,
    sent: &[Stamp],
    received: &[Received],
) -> RunCheck {
    let mut cursor = 0;
    let mut matched = 0;
    let mut used = vec![false; received.len()];
    for stamp in sent {
        if let Some(offset) = received[cursor..]
            .iter()
            .position(|event| event.key == stamp.key && event.up == stamp.up)
        {
            used[cursor + offset] = true;
            cursor += offset + 1;
            matched += 1;
        }
    }

    // Hold na chegada: cada descida até a próxima subida da mesma tecla.
    let mut received_holds_ms = Vec::new();
    let modifier = sent.first().map(|stamp| stamp.key);
    for (index, event) in received.iter().enumerate() {
        if event.up || Some(event.key) == modifier {
            continue;
        }
        if let Some(release) = received[index + 1..]
            .iter()
            .find(|later| later.key == event.key && later.up)
        {
            received_holds_ms.push(release.at.duration_since(event.at).as_secs_f32() * 1_000.0);
        }
    }

    RunCheck {
        codex: codex.to_vec(),
        outcome,
        sent: sent.len(),
        matched,
        missing: sent.len() - matched,
        extra: used.iter().filter(|used| !**used).count(),
        received_holds_ms,
        sent_holds_ms: modifier
            .map(|modifier| super::Timing::of(sent, modifier).holds_ms)
            .unwrap_or_default(),
    }
}

/// Resumo do teste inteiro, para o painel e o registro.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub runs: usize,
    pub passed: usize,
    pub missing_keys: usize,
    pub extra_keys: usize,
    /// Rodadas que o engine interrompeu: o foco saiu da janela no meio.
    pub interrupted: usize,
    /// Rodadas recusadas porque um atalho de verdade disparou durante o teste.
    pub blocked: usize,
    pub sent_hold: Spread,
    pub received_hold: Spread,
}

impl Summary {
    pub fn of(checks: &[RunCheck]) -> Summary {
        let mut summary = Summary {
            runs: checks.len(),
            ..Summary::default()
        };
        for check in checks {
            summary.passed += usize::from(check.passed());
            summary.missing_keys += check.missing;
            summary.extra_keys += check.extra;
            match check.outcome {
                RunOutcome::Completed => {}
                RunOutcome::Blocked => summary.blocked += 1,
                RunOutcome::Aborted | RunOutcome::Unfocused => summary.interrupted += 1,
            }
            for hold in &check.sent_holds_ms {
                summary.sent_hold.add(*hold);
            }
            for hold in &check.received_holds_ms {
                summary.received_hold.add(*hold);
            }
        }
        summary
    }
}

/// Um teste em andamento ou terminado, do ponto de vista da janela.
///
/// As preferências ficam congeladas no começo: mudar a velocidade no meio não
/// mistura dois perfis no mesmo resultado.
#[derive(Debug, Clone)]
pub struct TypingTest {
    pub speed: Speed,
    pub arrows: bool,
    pub modifier: Scan,
    plan: Vec<Arc<[Dir]>>,
    received: Vec<Received>,
    /// O que o engine disse ter mandado na rodada atual, à espera de as
    /// últimas teclas chegarem.
    sent: Option<(RunOutcome, Vec<Stamp>)>,
    checks: Vec<RunCheck>,
}

impl TypingTest {
    pub fn new(speed: Speed, arrows: bool, modifier: Scan) -> TypingTest {
        TypingTest {
            speed,
            arrows,
            modifier,
            plan: plan(),
            received: Vec::new(),
            sent: None,
            checks: Vec::new(),
        }
    }

    /// Codex da rodada a digitar agora, ou `None` com o teste terminado.
    pub fn current(&self) -> Option<Arc<[Dir]>> {
        self.plan.get(self.checks.len()).cloned()
    }

    pub fn done(&self) -> bool {
        self.checks.len() >= self.plan.len()
    }

    /// Rodadas conferidas e o total, para o "3/10" do painel.
    pub fn progress(&self) -> (usize, usize) {
        (self.checks.len(), self.plan.len())
    }

    pub fn on_key(&mut self, key: &'static str, up: bool, at: Instant) {
        if !self.done() {
            self.received.push(Received { key, up, at });
        }
    }

    /// O engine terminou a rodada. A conferência espera a janela drenar as
    /// teclas que ainda estão na fila: o aviso do engine é mensagem postada,
    /// que o Windows entrega antes da entrada de teclado pendente.
    pub fn on_sent(&mut self, outcome: RunOutcome, stamps: Vec<Stamp>) {
        self.sent = Some((outcome, stamps));
    }

    /// Confere a rodada atual e passa para a próxima. `false` quando não havia
    /// rodada esperando conferência.
    pub fn settle(&mut self) -> bool {
        let (Some((outcome, stamps)), Some(codex)) = (self.sent.take(), self.current()) else {
            return false;
        };
        let received = std::mem::take(&mut self.received);
        self.checks.push(check(&codex, outcome, &stamps, &received));
        true
    }

    pub fn checks(&self) -> &[RunCheck] {
        &self.checks
    }

    pub fn summary(&self) -> Summary {
        Summary::of(&self.checks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn stamp(key: &'static str, up: bool) -> Stamp {
        Stamp {
            at_ms: 0.0,
            key,
            up,
            ok: true,
        }
    }

    /// Mandado: CTRL, W, S, CTRL (descida e subida de cada um).
    fn sent() -> Vec<Stamp> {
        vec![
            stamp("CTRL", false),
            stamp("W", false),
            stamp("W", true),
            stamp("S", false),
            stamp("S", true),
            stamp("CTRL", true),
        ]
    }

    fn received(events: &[(&'static str, bool, u64)]) -> Vec<Received> {
        let origin = Instant::now();
        events
            .iter()
            .map(|(key, up, ms)| Received {
                key,
                up: *up,
                at: origin + Duration::from_millis(*ms),
            })
            .collect()
    }

    #[test]
    fn everything_arriving_in_order_passes() {
        let got = received(&[
            ("CTRL", false, 0),
            ("W", false, 100),
            ("W", true, 134),
            ("S", false, 154),
            ("S", true, 190),
            ("CTRL", true, 240),
        ]);
        let result = check(&[Dir::Up, Dir::Down], RunOutcome::Completed, &sent(), &got);
        assert!(result.passed());
        assert_eq!((result.matched, result.missing, result.extra), (6, 0, 0));
        assert_eq!(result.received_holds_ms, vec![34.0, 36.0]);
    }

    #[test]
    fn a_lost_key_and_a_stray_key_are_both_counted() {
        // O S nunca chegou; um A que ninguém mandou apareceu.
        let got = received(&[
            ("CTRL", false, 0),
            ("W", false, 100),
            ("W", true, 134),
            ("A", false, 150),
            ("A", true, 160),
            ("CTRL", true, 240),
        ]);
        let result = check(&[Dir::Up, Dir::Down], RunOutcome::Completed, &sent(), &got);
        assert!(!result.passed());
        assert_eq!((result.missing, result.extra), (2, 2));
    }

    #[test]
    fn keys_out_of_order_do_not_match() {
        let got = received(&[
            ("CTRL", false, 0),
            ("S", false, 100),
            ("S", true, 134),
            ("W", false, 154),
            ("W", true, 190),
            ("CTRL", true, 240),
        ]);
        let result = check(&[Dir::Up, Dir::Down], RunOutcome::Completed, &sent(), &got);
        assert!(!result.passed());
        assert!(result.missing > 0);
    }

    #[test]
    fn the_test_walks_its_plan_and_summarizes() {
        let mut test =
            TypingTest::new(Speed::Low, false, crate::keys::modifier_scan("LeftControl"));
        assert_eq!(test.progress(), (0, RUNS));
        assert_eq!(&*test.current().unwrap(), SUPPORT_STRATS[0].codex);

        // Sem aviso do engine, não há o que conferir.
        assert!(!test.settle());

        for run in 0..RUNS {
            let origin = Instant::now();
            test.on_key("CTRL", false, origin);
            test.on_key("CTRL", true, origin);
            let outcome = match run {
                3 => RunOutcome::Aborted,
                5 => RunOutcome::Blocked,
                _ => RunOutcome::Completed,
            };
            test.on_sent(outcome, vec![stamp("CTRL", false), stamp("CTRL", true)]);
            assert!(test.settle());
        }
        assert!(test.done());
        assert_eq!(test.current(), None);

        let summary = test.summary();
        assert_eq!(summary.runs, RUNS);
        assert_eq!(summary.interrupted, 1);
        assert_eq!(summary.blocked, 1, "bloqueio não é perda de foco");
        assert_eq!(summary.passed, RUNS - 2);

        // Terminado, tecla nova não entra em rodada nenhuma.
        test.on_key("W", false, Instant::now());
        assert!(!test.settle());
    }

    #[test]
    fn the_plan_alternates_a_plain_codex_and_one_with_a_repeat() {
        let plan = plan();
        assert_eq!(plan.len(), RUNS);
        assert!(
            plan[1].windows(2).any(|pair| pair[0] == pair[1]),
            "↑↑ do Rearmar"
        );
        assert!(plan[0].windows(2).all(|pair| pair[0] != pair[1]));
    }
}
