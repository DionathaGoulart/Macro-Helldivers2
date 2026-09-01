// Ponto de entrada do binário nativo. A ordem do boot é a que o plano define:
// instância única, estado compartilhado, threads de motor e hooks e, por
// último, a janela — que a partir daqui é quem segura o processo de pé.
//
// O app só é útil no Windows. No host de desenvolvimento (macOS/Linux) o crate
// compila, roda os testes de lógica e este resumo de sanidade.

// Subsistema "windows" só no build de release: sem ele o Explorer abre um
// console preto junto da janela a cada execução. No build de debug o console
// fica, que é por onde `RUST_LOG=debug cargo run` mostra os logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Result;

use macro_helldivers2::data::GameData;
use macro_helldivers2::settings::Settings;
use macro_helldivers2::shared::Shared;
use macro_helldivers2::{engine, hooks, i18n, loadouts, overlay, ui, util};

fn main() -> ExitCode {
    util::init_logging();

    match boot() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log::error!("falha no boot: {err:#}");
            // Sem console em release, um `Err` devolvido do `main` sumiria: o
            // processo morreria sem janela e sem uma linha de explicação. O
            // caso real é a instalação sem a pasta `assets/`.
            util::fatal_dialog(&format!("{err:#}"));
            ExitCode::FAILURE
        }
    }
}

fn boot() -> Result<()> {
    // O guard vive até o fim do `boot`: enquanto o app roda, uma segunda
    // execução encontra o mutex, devolve o foco para a janela que já existe e
    // sai — abrir dois processos instalaria dois hooks de teclado, e cada
    // atalho dispararia a sequência duas vezes.
    #[cfg(windows)]
    let _instance = {
        let lock = util::InstanceLock::acquire();
        if lock.as_ref().is_some_and(|lock| lock.already_running) {
            log::info!("já existe uma instância em execução; trazendo a janela dela para a frente");
            util::focus_running_instance(ui::window::CLASS_NAME);
            return Ok(());
        }
        lock
    };

    let settings = Settings::load();
    let text = i18n::tr(settings.language);
    let data = Arc::new(GameData::load()?);

    // Os slots são resolvidos contra os dados atuais já na leitura: id que sumiu
    // do jogo e conflito de exclusividade herdado saem antes de virarem atalho.
    let slots = loadouts::load_slots(&data);
    let (shared, receivers) = Shared::new(settings, slots);
    let settings = shared.settings_snapshot();

    // O motor fica bloqueado no canal até um atalho chegar. Ele segura uma
    // referência ao `Shared`, e com ela o próprio remetente, então nunca se
    // desliga sozinho: a thread morre junto com o processo, como as demais.
    engine::spawn(Arc::clone(&shared), receivers.engine)?;

    hooks::init(Arc::clone(&shared), Arc::clone(&data));
    let _hooks = hooks::spawn()?;

    // A thread do overlay só existe enquanto o recurso estiver ligado; a aba de
    // configurações a sobe e derruba pelo mesmo caminho.
    overlay::init(Arc::clone(&shared), Arc::clone(&data), receivers.overlay);
    overlay::set_enabled(settings.enable_overlay);

    log::info!(
        "config em {} · {} estratagemas carregados",
        util::config_dir().display(),
        data.all().len()
    );

    if cfg!(windows) {
        // A janela roda o message loop até o usuário fechar; as threads de
        // motor e hooks vivem enquanto o processo viver.
        ui::window::run(Arc::clone(&shared), Arc::clone(&data), receivers.ui)?;
    } else {
        println!(
            "{} v{} — {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            text.tabs.macro_tab
        );
        println!("idioma: {}", i18n::language_name(settings.language));
        println!("velocidade: {}", text.settings.speed(settings.macro_speed));
        println!("modificador in-game: {}", settings.modifier_key);
        println!("estratagemas: {}", data.all().len());
        println!("config: {}", util::config_dir().display());
        println!("Este binário só é funcional no Windows; aqui ele serve para check/test.");
    }

    Ok(())
}
