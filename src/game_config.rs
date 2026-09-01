//! Modo de vídeo do Helldivers 2, lido do `user_settings.config` do jogo.
//!
//! O HD2 em "Tela Cheia" (DXGI exclusivo) se auto-minimiza quando QUALQUER
//! janela desenha por cima dele — comportamento do jogo, sem relação com foco.
//! Overlay de janela só funciona em "Tela Cheia sem Borda", então o app detecta
//! o modo e avisa em vez de deixar o overlay quebrar a partida.
//!
//! Porte de `legacy/src/main/index.js` (~31–44), inclusive o cache: o arquivo é
//! consultado a cada troca de foco e de estado do overlay, e reler algumas
//! dezenas de KB nesse ritmo não se paga. O `mtime` decide se a leitura vale.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::util;

/// Nome do arquivo de configuração do jogo.
const CONFIG_FILE: &str = "user_settings.config";

/// Caminho de `%APPDATA%\Arrowhead\Helldivers2\user_settings.config`.
pub fn config_path() -> PathBuf {
    util::appdata_dir()
        .join("Arrowhead")
        .join("Helldivers2")
        .join(CONFIG_FILE)
}

/// Última leitura. `mtime` em `None` significa "nada válido em cache" — arquivo
/// ausente (jogo nunca aberto) ou ilegível.
struct Cache {
    mtime: Option<SystemTime>,
    value: bool,
}

static CACHE: Mutex<Cache> = Mutex::new(Cache {
    mtime: None,
    value: false,
});

/// O jogo está configurado em tela cheia exclusiva?
///
/// Sem arquivo, sem permissão ou com conteúdo inesperado a resposta é `false`:
/// a v1 fazia o mesmo, e um aviso falso atrapalharia mais que a ausência dele.
pub fn is_exclusive_fullscreen() -> bool {
    let path = config_path();
    let Ok(mtime) = std::fs::metadata(&path).and_then(|meta| meta.modified()) else {
        forget();
        return false;
    };

    let mut cache = CACHE.lock().unwrap_or_else(|err| err.into_inner());
    if cache.mtime == Some(mtime) {
        return cache.value;
    }

    let Ok(bytes) = std::fs::read(&path) else {
        *cache = Cache {
            mtime: None,
            value: false,
        };
        return false;
    };
    // O arquivo é ASCII; ler perdoando byte inválido evita transformar um
    // caractere estranho em "modo desconhecido".
    let value = parse(&String::from_utf8_lossy(&bytes));
    *cache = Cache {
        mtime: Some(mtime),
        value,
    };
    value
}

/// Invalida o cache. O arquivo some quando o jogo é desinstalado, e a próxima
/// leitura não pode responder com o modo de antes.
fn forget() {
    let mut cache = CACHE.lock().unwrap_or_else(|err| err.into_inner());
    cache.mtime = None;
    cache.value = false;
}

/// `^\s*fullscreen\s*=\s*true` sem `^\s*borderless_fullscreen\s*=\s*true` — as
/// duas expressões da v1, sem crate de regex.
///
/// "sem borda" liga as duas chaves no arquivo do jogo, e é a chave da borda que
/// desempata: só a exclusiva tem `fullscreen = true` sozinha.
pub fn parse(text: &str) -> bool {
    is_true(text, "fullscreen") && !is_true(text, "borderless_fullscreen")
}

/// Alguma linha diz `chave = true`?
///
/// A comparação é por início de linha, como o `^` com a flag `m` do JavaScript:
/// é o que faz `borderless_fullscreen` não casar com a chave `fullscreen`.
fn is_true(text: &str, key: &str) -> bool {
    text.lines().any(|line| {
        let Some(rest) = line.trim_start().strip_prefix(key) else {
            return false;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            return false;
        };
        rest.trim_start().starts_with("true")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
    }

    #[test]
    fn exclusive_fullscreen_is_the_only_mode_that_warns() {
        assert!(parse(&fixture("user_settings-fullscreen.config")));
        assert!(!parse(&fixture("user_settings-borderless.config")));
        assert!(!parse(&fixture("user_settings-windowed.config")));
    }

    #[test]
    fn the_borderless_key_never_answers_for_the_fullscreen_one() {
        // Só a chave da borda ligada: o jogo não está em tela cheia exclusiva.
        assert!(!parse("borderless_fullscreen = true\n"));
        // E o prefixo não pode ser confundido no meio de outra chave.
        assert!(!parse("window_fullscreen = true\n"));
    }

    #[test]
    fn spacing_and_missing_keys_are_tolerated() {
        assert!(parse("fullscreen=true"));
        assert!(parse("   fullscreen   =   true   "));
        assert!(!parse("fullscreen = false"));
        assert!(!parse("fullscreen ="));
        assert!(!parse(""));
    }

    #[test]
    fn a_missing_game_config_never_warns() {
        // A máquina de teste não tem o jogo instalado; a leitura precisa
        // responder "não" em vez de estourar.
        assert!(!is_exclusive_fullscreen());
    }
}
