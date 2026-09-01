// Ponto de entrada do binário nativo. A ordem do boot é a que o plano define:
// instância única, estado compartilhado, threads de motor e hooks e, por
// último, a janela — que a partir daqui é quem segura o processo de pé.
//
// O app só é útil no Windows. No host de desenvolvimento (macOS/Linux) o crate
// compila, roda os testes de lógica e este resumo de sanidade.

use std::sync::Arc;

use anyhow::Result;

use macro_helldivers2::data::GameData;
use macro_helldivers2::settings::Settings;
use macro_helldivers2::shared::{Shared, Slots};
use macro_helldivers2::{engine, hooks, i18n, ui, util};

fn main() -> Result<()> {
    util::init_logging();

    // O guard vive até o fim do `main`: enquanto o app roda, uma segunda
    // execução encontra o mutex. (Trazer a janela da primeira para a frente é
    // da Fase 10.)
    #[cfg(windows)]
    let _instance = {
        let lock = util::InstanceLock::acquire();
        if lock.as_ref().is_some_and(|lock| lock.already_running) {
            log::warn!("já existe uma instância do Macro Helldivers 2 em execução");
        }
        lock
    };

    let settings = Settings::load();
    let text = i18n::tr(settings.language);
    let data = Arc::new(GameData::load()?);

    // Slots ainda saem vazios: `slots.json` chega com a aba de macros (Fase 5).
    // Até lá só os atalhos de apoio fixo, configurados à mão no settings.json,
    // resolvem para um binding.
    let (shared, receivers) = Shared::new(settings, Slots::default());
    let settings = shared.settings_snapshot();

    // O motor fica bloqueado no canal até um atalho chegar. Ele segura uma
    // referência ao `Shared`, e com ela o próprio remetente, então nunca se
    // desliga sozinho: a thread morre junto com o processo, como as demais.
    engine::spawn(Arc::clone(&shared), receivers.engine)?;

    hooks::init(Arc::clone(&shared), Arc::clone(&data));
    let _hooks = hooks::spawn()?;

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
