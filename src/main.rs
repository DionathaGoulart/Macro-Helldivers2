// Ponto de entrada do binário nativo. A fundação lógica (settings, dados,
// traduções, teclas e estado compartilhado) e a thread do motor de macro já
// sobem aqui; o resto do bootstrap — instância única, hook de teclado, janela
// principal e message loop — chega nas fases seguintes do plano de reescrita.
//
// O app só é útil no Windows. No host de desenvolvimento (macOS/Linux) o crate
// compila, roda os testes de lógica e este resumo de sanidade.

use std::sync::Arc;

use anyhow::Result;

use macro_helldivers2::data::GameData;
use macro_helldivers2::settings::Settings;
use macro_helldivers2::shared::{Shared, Slots};
use macro_helldivers2::{engine, i18n, util};

fn main() -> Result<()> {
    util::init_logging();

    let settings = Settings::load();
    let text = i18n::tr(settings.language);
    let data = GameData::load()?;

    let (shared, receivers) = Shared::new(settings, Slots::default());
    let settings = shared.settings_snapshot();

    // O motor fica bloqueado no canal até um atalho chegar — o que só acontece
    // quando o hook de teclado entrar, na Fase 3. Ele segura uma referência ao
    // `Shared`, e com ela o próprio remetente, então nunca se desliga sozinho: a
    // thread morre junto com o processo, como as demais do app.
    engine::spawn(Arc::clone(&shared), receivers.engine)?;

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
