//! Bancada de timing do motor de macro.
//!
//! Mede o desvio entre a espera que a sequência pede e o intervalo real entre os
//! dois `SendInput` que a cercam. A meta do projeto é p99 abaixo de 1ms.
//!
//! **Isto digita de verdade.** Rode num Windows com um editor de texto em foco,
//! nunca com o jogo aberto — o objetivo aqui é o relógio, não o jogo (para o
//! jogo existe o `soak`). Fora do Windows o envio é ignorado e o que sobra é a
//! aferição do sono, útil só para conferir a bancada em si.

use std::process::ExitCode;
use std::time::Duration;

use macro_helldivers2::bench::{Collector, Trace};
use macro_helldivers2::data::Dir;
use macro_helldivers2::engine::{self, Jitter, Sequence, SystemInput};
use macro_helldivers2::keys;
use macro_helldivers2::settings::Speed;

/// Codex de quatro passos que exercita as quatro direções.
const CODEX: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];

/// Meta do plano para o percentil 99 do desvio.
const TARGET_P99_MS: f64 = 1.0;

const HELP: &str = "\
timing_bench — desvio de timing do motor de macro

USO:
    timing_bench [opções]

OPÇÕES:
    -n <N>              sequências a executar (padrão: 200)
    --speed <perfil>    normal | fast | turbo (padrão: turbo)
    --arrows            usa as setas em vez de WASD
    --no-jitter         desliga o jitter humanizado
    --gap <ms>          pausa entre sequências (padrão: 200)
    -h, --help          mostra esta ajuda

COMO RODAR:
    1. Windows, sem o jogo aberto.
    2. Abra um editor de texto vazio e deixe-o em foco — a bancada digita
       Ctrl + WASD de verdade, uma sequência por vez.
    3. Rode e não toque no teclado até o relatório sair.
    4. Feche o que estiver disputando CPU: o que se mede aqui é o escalonador.
";

struct Args {
    runs: usize,
    speed: Speed,
    use_arrows: bool,
    jitter: Jitter,
    gap: Duration,
}

impl Default for Args {
    fn default() -> Args {
        Args {
            runs: 200,
            speed: Speed::Turbo,
            use_arrows: false,
            jitter: Jitter::Humanized,
            gap: Duration::from_millis(200),
        }
    }
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
            "--no-jitter" => args.jitter = Jitter::Off,
            "--gap" => {
                let ms: u64 = value()?
                    .parse()
                    .map_err(|_| "--gap espera um inteiro".to_string())?;
                args.gap = Duration::from_millis(ms);
            }
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
        println!("AVISO: fora do Windows nenhuma tecla é enviada; só o sono é medido.\n");
    }

    let sequence = Sequence {
        codex: CODEX.to_vec(),
        modifier: keys::modifier_scan("LeftControl"),
        use_arrows: args.use_arrows,
        speed: args.speed,
    };

    println!(
        "{} sequências · perfil {} · {} · jitter {}",
        args.runs,
        args.speed,
        if args.use_arrows { "setas" } else { "WASD" },
        if args.jitter == Jitter::Off {
            "off"
        } else {
            "humanizado"
        }
    );
    countdown(3);

    let mut sink = Trace::new(SystemInput::new());
    let mut collector = Collector::new();
    for _ in 0..args.runs {
        sink.clear();
        engine::run_sequence(&mut sink, &sequence, args.jitter, &mut ());
        collector.add(&sink.deviations());
        std::thread::sleep(args.gap);
    }

    let report = collector.report();
    println!("\ndesvio observado (positivo = dormiu além do pedido)");
    for (label, stats) in &report {
        println!("  {label:<6} {stats}");
    }

    // A primeira linha é o agregado de todas as fases.
    let p99 = report
        .first()
        .map(|(_, stats)| stats.p99)
        .unwrap_or_default();
    if !cfg!(windows) {
        // Sem `timeBeginPeriod` e com outro escalonador, o número daqui não diz
        // nada sobre o alvo; serve só para conferir a própria bancada.
        println!("\np99 {p99:.3}ms — veredito só vale num Windows real.");
        return ExitCode::SUCCESS;
    }
    if p99 < TARGET_P99_MS {
        println!("\nOK: p99 {p99:.3}ms < {TARGET_P99_MS:.1}ms");
        ExitCode::SUCCESS
    } else {
        println!("\nFALHOU: p99 {p99:.3}ms >= {TARGET_P99_MS:.1}ms");
        ExitCode::FAILURE
    }
}

fn countdown(seconds: u32) {
    for left in (1..=seconds).rev() {
        println!("começando em {left}...");
        std::thread::sleep(Duration::from_secs(1));
    }
}
