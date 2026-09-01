// Ponto de entrada do binário nativo. A fundação lógica (settings, dados,
// traduções, teclas e estado compartilhado) já sobe aqui; o bootstrap completo
// — instância única, threads engine/hooks, janela principal e message loop —
// chega junto com a janela, na Fase 4 do plano de reescrita.
//
// O app só é útil no Windows. No host de desenvolvimento (macOS/Linux) o crate
// compila, roda os testes de lógica e este resumo de sanidade.

use anyhow::Result;

use macro_helldivers2::data::GameData;
use macro_helldivers2::settings::Settings;
use macro_helldivers2::shared::{Shared, Slots};
use macro_helldivers2::{i18n, util};

fn main() -> Result<()> {
    util::init_logging();

    let settings = Settings::load();
    let text = i18n::tr(settings.language);
    let data = GameData::load()?;

    // Os canais ficam vivos com o `Shared`; as threads que os consomem entram
    // nas próximas fases.
    let (shared, _receivers) = Shared::new(settings, Slots::default());
    let settings = shared.settings_snapshot();

    log::info!(
        "config em {} · {} estratagemas carregados",
        util::config_dir().display(),
        data.all().len()
    );

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

    if !cfg!(windows) {
        println!("Este binário só é funcional no Windows; aqui ele serve para check/test.");
    }

    Ok(())
}
