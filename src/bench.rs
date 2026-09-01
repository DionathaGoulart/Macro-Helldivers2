//! Instrumentação das bancadas de timing (`src/bin/timing_bench.rs` e
//! `src/bin/soak.rs`).
//!
//! Mora na biblioteca porque os dois binários precisam do mesmo par: um sink que
//! carimba o relógio a cada tecla enviada e a estatística que transforma os
//! carimbos em desvio por percentil.
//!
//! O que se mede é o intervalo real entre dois `SendInput` consecutivos contra o
//! que a sequência pediu de espera entre eles. É o número que importa: inclui o
//! custo da própria chamada de envio, que é o que o jogo enxerga.

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, Instant};

use crate::engine::{InputSink, KeyEvent, Phase};

/// Um passo da sequência, na ordem em que aconteceu.
enum Step {
    /// Instante em que uma tecla foi despachada.
    Sent(Instant),
    Waited(Phase, Duration),
}

/// Envolve um sink real e guarda quando cada tecla saiu e quanto se pediu de espera.
pub struct Trace<S: InputSink> {
    inner: S,
    steps: Vec<Step>,
}

impl<S: InputSink> Trace<S> {
    pub fn new(inner: S) -> Trace<S> {
        Trace {
            inner,
            steps: Vec::new(),
        }
    }

    /// Desvio de cada trecho entre duas teclas, em milissegundos: positivo dormiu
    /// demais, negativo de menos.
    ///
    /// Quase sempre há exatamente uma espera entre duas teclas. A exceção é o fim
    /// da sequência, onde o `gap` da última direção e o `tail` se somam sem tecla
    /// no meio: o par vira uma medida só, cobrada contra a soma dos dois e
    /// rotulada pela fase que a fecha.
    pub fn deviations(&self) -> Vec<(Phase, f64)> {
        let mut deviations = Vec::new();
        let mut opened: Option<Instant> = None;
        let mut pending: Option<(Phase, Duration)> = None;

        for step in &self.steps {
            match step {
                Step::Waited(phase, duration) => {
                    // A fase que fecha o trecho é a que o nomeia.
                    let total = pending.map_or(Duration::ZERO, |(_, total)| total) + *duration;
                    pending = Some((*phase, total));
                }
                Step::Sent(at) => {
                    if let (Some(from), Some((phase, requested))) = (opened, pending.take()) {
                        deviations.push((phase, ms(at.duration_since(from)) - ms(requested)));
                    }
                    opened = Some(*at);
                }
            }
        }
        deviations
    }

    /// Zera a linha do tempo entre execuções sem descartar o sink.
    pub fn clear(&mut self) {
        self.steps.clear();
    }
}

impl<S: InputSink> InputSink for Trace<S> {
    fn send(&mut self, event: KeyEvent) {
        // Carimbo antes do envio: é o instante em que a tecla é despachada.
        self.steps.push(Step::Sent(Instant::now()));
        self.inner.send(event);
    }

    fn wait(&mut self, phase: Phase, duration: Duration) {
        self.steps.push(Step::Waited(phase, duration));
        self.inner.wait(phase, duration);
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

/// Resumo de uma amostra de desvios, em milissegundos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub count: usize,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

impl Stats {
    /// Ordena a amostra no lugar e resume. `None` para amostra vazia.
    pub fn of(sample: &mut [f64]) -> Option<Stats> {
        if sample.is_empty() {
            return None;
        }
        sample.sort_by(|a, b| a.total_cmp(b));
        Some(Stats {
            count: sample.len(),
            mean: sample.iter().sum::<f64>() / sample.len() as f64,
            p50: percentile(sample, 50.0),
            p95: percentile(sample, 95.0),
            p99: percentile(sample, 99.0),
            max: sample[sample.len() - 1],
        })
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "n={:<6} média {:>6.3}ms · p50 {:>6.3}ms · p95 {:>6.3}ms · p99 {:>6.3}ms · máx {:>6.3}ms",
            self.count, self.mean, self.p50, self.p95, self.p99, self.max
        )
    }
}

/// Percentil por posto mais próximo, sobre uma amostra já ordenada.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let last = sorted.len() - 1;
    let index = (p / 100.0 * last as f64).round() as usize;
    sorted[index.min(last)]
}

/// Junta desvios de várias execuções e separa por fase da sequência.
#[derive(Debug, Default)]
pub struct Collector {
    per_phase: BTreeMap<&'static str, Vec<f64>>,
    all: Vec<f64>,
}

impl Collector {
    pub fn new() -> Collector {
        Collector::default()
    }

    pub fn add(&mut self, deviations: &[(Phase, f64)]) {
        for (phase, deviation) in deviations {
            self.per_phase
                .entry(phase.as_str())
                .or_default()
                .push(*deviation);
            self.all.push(*deviation);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    /// Quantos desvios passaram do limite, com a fase de cada um.
    pub fn violations(&self, limit_ms: f64) -> Vec<(&'static str, f64)> {
        let mut over: Vec<(&'static str, f64)> = self
            .per_phase
            .iter()
            .flat_map(|(phase, values)| {
                values
                    .iter()
                    .filter(move |value| value.abs() > limit_ms)
                    .map(move |value| (*phase, *value))
            })
            .collect();
        over.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
        over
    }

    /// Resumo geral seguido de um por fase, na ordem `gap`, `hold`, `lead`, `tail`.
    pub fn report(&mut self) -> Vec<(&'static str, Stats)> {
        let mut rows = Vec::new();
        if let Some(stats) = Stats::of(&mut self.all) {
            rows.push(("todos", stats));
        }
        for (phase, values) in self.per_phase.iter_mut() {
            if let Some(stats) = Stats::of(values) {
                rows.push((phase, stats));
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_land_on_the_expected_ranks() {
        let sorted: Vec<f64> = (0..=100).map(f64::from).collect();
        assert_eq!(percentile(&sorted, 50.0), 50.0);
        assert_eq!(percentile(&sorted, 95.0), 95.0);
        assert_eq!(percentile(&sorted, 99.0), 99.0);
        assert_eq!(percentile(&sorted, 100.0), 100.0);
    }

    #[test]
    fn stats_summarize_an_unsorted_sample() {
        let mut sample = vec![3.0, 1.0, 2.0, 10.0];
        let stats = Stats::of(&mut sample).unwrap();
        assert_eq!(stats.count, 4);
        assert_eq!(stats.max, 10.0);
        // Posto mais próximo numa amostra par arredonda para cima: 1,5 → índice 2.
        assert_eq!(stats.p50, 3.0);
        assert_eq!(Stats::of(&mut []), None);
    }

    /// Sink que não dorme: cada espera pedida vira tempo real zero, então o
    /// desvio observado é o negativo do que foi pedido — previsível o bastante
    /// para conferir o pareamento.
    struct Instant0;

    impl InputSink for Instant0 {
        fn send(&mut self, _event: KeyEvent) {}
        fn wait(&mut self, _phase: Phase, _duration: Duration) {}
    }

    #[test]
    fn the_last_gap_and_the_tail_are_measured_as_one_stretch() {
        use crate::data::Dir;
        use crate::engine::{run_sequence, Jitter, Sequence};
        use crate::settings::Speed;

        let mut trace = Trace::new(Instant0);
        run_sequence(
            &mut trace,
            &Sequence {
                codex: vec![Dir::Up],
                modifier: crate::keys::modifier_scan("LeftControl"),
                use_arrows: false,
                speed: Speed::Turbo,
            },
            Jitter::Off,
            &mut (),
        );

        let deviations = trace.deviations();
        let phases: Vec<Phase> = deviations.iter().map(|(phase, _)| *phase).collect();
        // Uma direção: lead, hold, e o gap+tail fechados pelo tail.
        assert_eq!(phases, vec![Phase::Lead, Phase::Hold, Phase::Tail]);
        // Sem dormir de verdade, o desvio é o que se pediu, com o sinal trocado —
        // e o último trecho cobra os 6ms de gap somados aos 30ms de tail.
        assert!(
            (deviations[2].1 + 36.0).abs() < 1.0,
            "trecho final: {}",
            deviations[2].1
        );
    }

    #[test]
    fn collector_splits_by_phase_and_flags_violations() {
        let mut collector = Collector::new();
        collector.add(&[(Phase::Hold, 0.2), (Phase::Gap, -7.5), (Phase::Hold, 0.4)]);

        assert_eq!(collector.violations(5.0), vec![("gap", -7.5)]);
        let report = collector.report();
        assert_eq!(report[0].0, "todos");
        assert_eq!(report[0].1.count, 3);
        let hold = report.iter().find(|(phase, _)| *phase == "hold").unwrap();
        assert_eq!(hold.1.count, 2);
    }
}
