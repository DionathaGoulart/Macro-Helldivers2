//! Teste de resistência do motor de macro: mil sequências em série.
//!
//! O `timing_bench` responde "o relógio é bom?". Este responde "o jogo aceita
//! todas?", que é a pergunta que fecha a Fase 2 do plano.
//!
//! PROTOCOLO IN-GAME (o que vale como aprovado):
//!
//! 1. Windows, Helldivers 2 aberto em **borderless**, dentro de uma missão, com
//!    o personagem parado e em segurança.
//! 2. Equipe um estratagema cujo codex seja o mesmo passado aqui (`--codex`), ou
//!    use o padrão `UP DOWN RIGHT LEFT UP` (Reforço) numa partida em que ele
//!    esteja disponível.
//! 3. Deixe o jogo em foco e rode: `soak.exe -n 1000 --speed turbo`.
//! 4. Repita com o jogo travado em **60fps** e depois em **30fps** — é o cap de
//!    30 que expõe um `hold` curto demais.
//! 5. Aprovado = 1.000 chamadas do estratagema no jogo, zero falhas, e o
//!    relatório sem violações de desvio.
//!
//! Fora do Windows nenhuma tecla sai; o binário ainda roda para conferir a
//! contabilidade do relatório.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use macro_helldivers2::bench::{Collector, Trace};
use macro_helldivers2::data::Dir;
use macro_helldivers2::engine::{self, Jitter, Sequence, SystemInput};
use macro_helldivers2::keys;
use macro_helldivers2::settings::Speed;

/// Codex do Reforço: cinco passos, o mais longo dos apoios fixos.
const CODEX: [Dir; 5] = [Dir::Up, Dir::Down, Dir::Right, Dir::Left, Dir::Up];

/// Acima disto o desvio deixa de ser ruído de escalonador e vira risco de o jogo
/// perder a tecla.
const VIOLATION_MS: f64 = 5.0;

const HELP: &str = "\
soak — mil sequências em série contra o jogo

USO:
    soak [opções]

OPÇÕES:
    -n <N>              sequências a executar (padrão: 1000)
    --speed <perfil>    normal | fast | turbo (padrão: turbo)
    --arrows            usa as setas em vez de WASD
    --gap <ms>          pausa entre sequências (padrão: 300)
    --codex <passos>    ex.: UP,DOWN,RIGHT,LEFT,UP (padrão: Reforço)
    -h, --help          mostra esta ajuda

O protocolo in-game está no cabeçalho do arquivo: borderless, dentro de uma
missão, jogo em foco, uma rodada a 60fps e outra a 30fps, zero estratagema
falhado.
";

struct Args {
    runs: usize,
    speed: Speed,
    use_arrows: bool,
    gap: Duration,
    codex: Vec<Dir>,
}

impl Default for Args {
    fn default() -> Args {
        Args {
            runs: 1_000,
            speed: Speed::Turbo,
            use_arrows: false,
            gap: Duration::from_millis(300),
            codex: CODEX.to_vec(),
        }
    }
}

fn parse_codex(raw: &str) -> Result<Vec<Dir>, String> {
    raw.split(',')
        .map(|step| match step.trim().to_ascii_uppercase().as_str() {
            "UP" => Ok(Dir::Up),
            "DOWN" => Ok(Dir::Down),
            "LEFT" => Ok(Dir::Left),
            "RIGHT" => Ok(Dir::Right),
            other => Err(format!("passo inválido no codex: {other}")),
        })
        .collect()
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args::default();
    let mut argv = std::env::args().skip(1);

    while let Some(flag) = argv.next() {
        let mut value = || {
            argv.next()
                .ok_or_else(|| format!("faltou o valor de {flag}"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "-n" => {
                args.runs = value()?
                    .parse()
                    .map_err(|_| "-n espera um inteiro".to_string())?
            }
            "--speed" => args.speed = Speed::from_str_or_default(&value()?),
            "--arrows" => args.use_arrows = true,
            "--gap" => {
                let ms: u64 = value()?
                    .parse()
                    .map_err(|_| "--gap espera um inteiro".to_string())?;
                args.gap = Duration::from_millis(ms);
            }
            "--codex" => args.codex = parse_codex(&value()?)?,
            other => return Err(format!("opção desconhecida: {other}")),
        }
    }
    Ok(Some(args))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Some(args)) => args,
        Ok(None) => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("{err}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    };

    if !cfg!(windows) {
        println!("AVISO: fora do Windows nenhuma tecla é enviada.\n");
    }

    let sequence = Sequence {
        codex: args.codex.as_slice().into(),
        modifier: keys::modifier_scan("LeftControl"),
        use_arrows: args.use_arrows,
        speed: args.speed,
    };

    let codex: Vec<&str> = args
        .codex
        .iter()
        .map(|dir| match dir {
            Dir::Up => "UP",
            Dir::Down => "DOWN",
            Dir::Left => "LEFT",
            Dir::Right => "RIGHT",
        })
        .collect();
    println!(
        "{} sequências · perfil {} · codex {} · pausa {}ms",
        args.runs,
        args.speed,
        codex.join(" "),
        args.gap.as_millis()
    );
    println!("deixe o jogo em foco agora; conte os estratagemas chamados no fim.");
    countdown(5);

    let started = Instant::now();
    let mut sink = Trace::new(SystemInput::new());
    let mut collector = Collector::new();
    for run in 1..=args.runs {
        sink.clear();
        engine::run_sequence(&mut sink, &sequence, Jitter::Humanized, &mut ());
        collector.add(&sink.deviations());
        if run % 100 == 0 {
            println!("  {run}/{} ...", args.runs);
        }
        std::thread::sleep(args.gap);
    }

    println!("\nconcluído em {:.1}s", started.elapsed().as_secs_f64());
    println!("desvio observado (positivo = dormiu além do pedido)");
    for (label, stats) in collector.report() {
        println!("  {label:<6} {stats}");
    }

    let violations = collector.violations(VIOLATION_MS);
    if violations.is_empty() {
        println!("\nOK: nenhuma espera passou de {VIOLATION_MS:.0}ms de desvio.");
        println!("Confirme no jogo: {} estratagemas chamados.", args.runs);
        ExitCode::SUCCESS
    } else {
        println!(
            "\n{} violações acima de {VIOLATION_MS:.0}ms:",
            violations.len()
        );
        for (phase, deviation) in violations.iter().take(10) {
            println!("  {phase:<6} {deviation:+.3}ms");
        }
        ExitCode::FAILURE
    }
}

fn countdown(seconds: u32) {
    for left in (1..=seconds).rev() {
        println!("começando em {left}...");
        std::thread::sleep(Duration::from_secs(1));
    }
}
