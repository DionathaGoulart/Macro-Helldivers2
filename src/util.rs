//! Utilidades de base: caminhos do usuário, localização dos assets, logging e
//! escrita atômica de arquivos.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use directories::BaseDirs;

/// Pasta de dados do usuário. No Windows resolve para
/// `%APPDATA%\Macro Helldivers 2` — o caminho exato que o desinstalador preserva.
pub const APP_DIR_NAME: &str = "Macro Helldivers 2";

/// Subpasta que a v1 (Electron) criava dentro do `userData`.
const LEGACY_SUBDIR: &str = "Helldivers Macro";

/// Candidatos a `userData` da v1. O Electron usa `productName` quando existe
/// ("Macro Helldivers 2") e cai no `name` do package.json ("helldivers-macro")
/// quando não; tentamos os dois porque o instalador antigo circulou nas duas formas.
const LEGACY_APP_DIRS: [&str; 2] = ["Macro Helldivers 2", "helldivers-macro"];

fn base_config_dir() -> PathBuf {
    BaseDirs::new()
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Diretório onde ficam `settings.json`, `slots.json`, `loadouts.json` e caches.
pub fn config_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| base_config_dir().join(APP_DIR_NAME))
}

/// Caminho de um arquivo de configuração do app.
pub fn config_path(file: &str) -> PathBuf {
    config_dir().join(file)
}

/// Caminhos onde a v1 pode ter deixado o mesmo arquivo, em ordem de preferência.
pub fn legacy_config_paths(file: &str) -> Vec<PathBuf> {
    let base = base_config_dir();
    LEGACY_APP_DIRS
        .iter()
        .map(|dir| base.join(dir).join(LEGACY_SUBDIR).join(file))
        .collect()
}

/// Raiz dos assets. Em debug aponta pro repositório; em release, pra pasta
/// `assets/` ao lado do executável (é assim que o instalador distribui).
pub fn assets_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join("assets")))
                .unwrap_or_else(|| PathBuf::from("assets"))
        }
    })
}

/// Caminho de um asset, relativo à raiz de `assets/` (ex.: `data/stratagems.json`).
pub fn asset_path(rel: &str) -> PathBuf {
    assets_dir().join(rel)
}

/// Grava criando o diretório se preciso, via arquivo temporário + rename, para
/// que uma queda no meio da escrita nunca deixe um JSON truncado no lugar do bom.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("caminho sem diretório pai: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("não foi possível criar {}", parent.display()))?;

    let mut tmp_name = path
        .file_name()
        .with_context(|| format!("caminho sem nome de arquivo: {}", path.display()))?
        .to_os_string();
    tmp_name.push(".tmp");
    let tmp = parent.join(tmp_name);

    fs::write(&tmp, bytes).with_context(|| format!("falha ao escrever {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("falha ao publicar {}", path.display()))?;
    Ok(())
}

/// Liga o logger. Sem console em release, então isto serve principalmente para
/// rodar o app a partir de um terminal com `RUST_LOG=debug` durante o diagnóstico.
pub fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = env_logger::Builder::new().parse_filters(&filter).try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "mh2-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique).join(name)
    }

    #[test]
    fn write_atomic_creates_dirs_and_replaces_file() {
        let path = temp_path("settings.json");
        write_atomic(&path, b"{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");

        write_atomic(&path, b"{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");

        // O temporário não fica para trás.
        assert!(!path.with_file_name("settings.json.tmp").exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn assets_dir_resolves_to_repo_in_debug() {
        assert!(asset_path("data/stratagems.json").exists());
    }

    #[test]
    fn legacy_paths_cover_both_electron_names() {
        let paths = legacy_config_paths("settings.json");
        assert_eq!(paths.len(), 2);
        // `Path::ends_with` compara por componentes, então a barra normal serve nos dois SOs.
        assert!(paths
            .iter()
            .all(|p| p.ends_with("Helldivers Macro/settings.json")));
    }
}
