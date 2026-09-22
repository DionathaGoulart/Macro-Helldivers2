//! Modo de vídeo do Helldivers 2, lido do `user_settings.config` do jogo.
//!
//! O HD2 em "Tela Cheia" (DXGI exclusivo) se auto-minimiza quando QUALQUER
//! janela desenha por cima dele (comportamento do jogo, sem relação com foco).
//! Overlay de janela só funciona em "Tela Cheia sem Borda", então o app detecta
//! o modo e avisa em vez de deixar o overlay quebrar a partida.
//!
//! Porte de `legacy/src/main/index.js` (~31-44), inclusive o cache: o arquivo é
//! consultado a cada troca de foco e de estado do overlay, e reler algumas
//! dezenas de KB nesse ritmo não se paga. O `mtime` decide se a leitura vale.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use serde::Serialize;

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

/// O que o app lê do vídeo do jogo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    /// "Tela Cheia" exclusiva, o modo em que o overlay derruba o jogo.
    pub exclusive_fullscreen: bool,
    /// Limite de FPS do próprio jogo. `None` sem limite (`max_fps = 0`) ou sem
    /// a chave no arquivo. Um limite imposto por fora (painel da NVIDIA,
    /// RivaTuner) não aparece aqui.
    pub max_fps: Option<u32>,
    pub vsync: Option<bool>,
    pub resolution: Option<(u32, u32)>,
}

/// Última leitura. `mtime` em `None` significa "nada válido em cache": arquivo
/// ausente (jogo nunca aberto) ou ilegível.
struct Cache {
    mtime: Option<SystemTime>,
    value: Option<Video>,
}

static CACHE: Mutex<Cache> = Mutex::new(Cache {
    mtime: None,
    value: None,
});

/// O jogo está configurado em tela cheia exclusiva?
///
/// Sem arquivo, sem permissão ou com conteúdo inesperado a resposta é `false`:
/// a v1 fazia o mesmo, e um aviso falso atrapalharia mais que a ausência dele.
pub fn is_exclusive_fullscreen() -> bool {
    video().is_some_and(|video| video.exclusive_fullscreen)
}

/// Limite de FPS configurado no jogo, se houver.
pub fn max_fps() -> Option<u32> {
    video().and_then(|video| video.max_fps)
}

/// Vídeo do jogo como está no disco agora. `None` sem arquivo ou sem permissão
/// de leitura.
pub fn video() -> Option<Video> {
    let path = config_path();
    let Ok(mtime) = std::fs::metadata(&path).and_then(|meta| meta.modified()) else {
        forget();
        return None;
    };

    let mut cache = CACHE.lock().unwrap_or_else(|err| err.into_inner());
    if cache.mtime == Some(mtime) {
        return cache.value;
    }

    let Ok(bytes) = std::fs::read(&path) else {
        *cache = Cache {
            mtime: None,
            value: None,
        };
        return None;
    };
    // O arquivo é ASCII; ler perdoando byte inválido evita transformar um
    // caractere estranho em "modo desconhecido".
    let value = parse_video(&String::from_utf8_lossy(&bytes));
    *cache = Cache {
        mtime: Some(mtime),
        value: Some(value),
    };
    Some(value)
}

/// Invalida o cache. O arquivo some quando o jogo é desinstalado, e a próxima
/// leitura não pode responder com o modo de antes.
fn forget() {
    let mut cache = CACHE.lock().unwrap_or_else(|err| err.into_inner());
    cache.mtime = None;
    cache.value = None;
}

/// `^\s*fullscreen\s*=\s*true` sem `^\s*borderless_fullscreen\s*=\s*true`: as
/// duas expressões da v1, sem crate de regex.
///
/// "sem borda" liga as duas chaves no arquivo do jogo, e é a chave da borda que
/// desempata: só a exclusiva tem `fullscreen = true` sozinha.
pub fn parse(text: &str) -> bool {
    is_true(text, "fullscreen") && !is_true(text, "borderless_fullscreen")
}

/// Todas as chaves de vídeo que o app usa, de uma leitura só.
pub fn parse_video(text: &str) -> Video {
    let number = |key: &str| {
        value(text, key).and_then(|raw| {
            let digits: String = raw.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u32>().ok()
        })
    };
    let flag = |key: &str| {
        value(text, key).and_then(|raw| {
            if raw.starts_with("true") {
                Some(true)
            } else if raw.starts_with("false") {
                Some(false)
            } else {
                None
            }
        })
    };
    Video {
        exclusive_fullscreen: parse(text),
        max_fps: number("max_fps").filter(|fps| *fps > 0),
        vsync: flag("vsync"),
        resolution: number("screen_width").zip(number("screen_height")),
    }
}

/// O valor da primeira linha `chave = valor`, já sem espaços à esquerda.
fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(key)?;
        Some(rest.trim_start().strip_prefix('=')?.trim_start())
    })
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
    fn the_video_block_is_read_in_one_pass() {
        let windowed = parse_video(&fixture("user_settings-windowed.config"));
        assert_eq!(
            windowed,
            Video {
                exclusive_fullscreen: false,
                max_fps: Some(60),
                vsync: Some(true),
                resolution: Some((1600, 900)),
            }
        );

        // `max_fps = 0` é o jogo sem limite.
        let borderless = parse_video(&fixture("user_settings-borderless.config"));
        assert_eq!(borderless.max_fps, None);
        assert_eq!(borderless.resolution, Some((2560, 1440)));
        assert!(parse_video(&fixture("user_settings-fullscreen.config")).exclusive_fullscreen);
    }

    #[test]
    fn video_keys_do_not_match_longer_keys_or_garbage() {
        let video = parse_video("max_fps_menu = 30\nvsync_mode = true\nmax_fps = abc\n");
        assert_eq!(video.max_fps, None);
        assert_eq!(video.vsync, None);
        assert_eq!(parse_video("\tmax_fps=30 // comentário").max_fps, Some(30));
        assert_eq!(parse_video(""), Video::default());
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
