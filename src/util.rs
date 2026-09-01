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

/// Instante atual em ISO-8601 UTC com milissegundos.
///
/// É o formato que o `Date.prototype.toISOString()` da v1 gravava no
/// `exportedAt` do backup, e um arquivo exportado aqui precisa continuar
/// parecendo o de lá.
pub fn iso8601_now() -> String {
    iso8601(std::time::SystemTime::now())
}

fn iso8601(time: std::time::SystemTime) -> String {
    let since = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = since.as_secs() as i64;
    let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3_600,
        (rest % 3_600) / 60,
        rest % 60,
        since.subsec_millis(),
    )
}

/// Dias desde a época → data civil. Algoritmo de Howard Hinnant, o mesmo que
/// as bibliotecas de data usam; vale mais que arrastar uma dependência inteira
/// para formatar um campo.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Move a origem para 1º de março de 0000, onde o ano bissexto cai no fim.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Tipo de arquivo oferecido num diálogo (`{ name, extensions }` da v1).
#[derive(Debug, Clone, Copy)]
pub struct FileFilter<'a> {
    pub label: &'a str,
    /// Máscara no formato do shell, ex.: `*.json`.
    pub spec: &'a str,
    /// Extensão acrescentada quando o usuário não digita nenhuma.
    pub extension: &'a str,
}

/// Filtro dos arquivos de backup.
pub const JSON_FILTER: FileFilter<'static> = FileFilter {
    label: "JSON",
    spec: "*.json",
    extension: "json",
};

/// Diálogo "salvar como". `Ok(None)` quando o usuário cancela — que não é erro
/// e não vira aviso na tela.
#[cfg(windows)]
pub fn save_dialog(
    title: &str,
    default_name: &str,
    filter: FileFilter<'_>,
) -> Result<Option<PathBuf>> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::{FileSaveDialog, IFileDialog, IFileSaveDialog};

    let _com = ComScope::enter()?;
    // SAFETY: CLSID do diálogo do próprio shell, criado no processo.
    let dialog: IFileSaveDialog =
        unsafe { CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER) }
            .context("CoCreateInstance(FileSaveDialog)")?;
    let dialog: IFileDialog = dialog.cast().context("IFileDialog")?;

    let name = wide(default_name);
    // SAFETY: a string vive durante a chamada.
    unsafe { dialog.SetFileName(windows::core::PCWSTR(name.as_ptr())) }.context("SetFileName")?;
    show_dialog(&dialog, title, filter)
}

/// Diálogo "abrir". `Ok(None)` quando o usuário cancela.
#[cfg(windows)]
pub fn open_dialog(title: &str, filter: FileFilter<'_>) -> Result<Option<PathBuf>> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::{FileOpenDialog, IFileDialog, IFileOpenDialog};

    let _com = ComScope::enter()?;
    // SAFETY: CLSID do diálogo do próprio shell, criado no processo.
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
            .context("CoCreateInstance(FileOpenDialog)")?;
    let dialog: IFileDialog = dialog.cast().context("IFileDialog")?;
    show_dialog(&dialog, title, filter)
}

/// Configura, mostra e lê o caminho escolhido.
#[cfg(windows)]
fn show_dialog(
    dialog: &windows::Win32::UI::Shell::IFileDialog,
    title: &str,
    filter: FileFilter<'_>,
) -> Result<Option<PathBuf>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_CANCELLED;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::SIGDN_FILESYSPATH;

    let (title_w, label, spec, extension) = (
        wide(title),
        wide(filter.label),
        wide(filter.spec),
        wide(filter.extension),
    );
    let types = [COMDLG_FILTERSPEC {
        pszName: PCWSTR(label.as_ptr()),
        pszSpec: PCWSTR(spec.as_ptr()),
    }];

    // SAFETY: todas as strings vivem até o fim da função, depois do `Show`.
    unsafe {
        dialog
            .SetTitle(PCWSTR(title_w.as_ptr()))
            .context("SetTitle")?;
        dialog.SetFileTypes(&types).context("SetFileTypes")?;
        dialog
            .SetDefaultExtension(PCWSTR(extension.as_ptr()))
            .context("SetDefaultExtension")?;

        if let Err(err) = dialog.Show(None) {
            // Cancelar não é falha: a v1 também separava os dois casos para não
            // mostrar "arquivo inválido" a quem só desistiu.
            return if err.code() == ERROR_CANCELLED.to_hresult() {
                Ok(None)
            } else {
                Err(err).context("diálogo de arquivo")
            };
        }

        let item = dialog.GetResult().context("GetResult")?;
        let raw = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .context("GetDisplayName")?;
        let path = raw.to_string().context("caminho não é UTF-16 válido");
        CoTaskMemFree(Some(raw.0 as *const std::ffi::c_void));
        Ok(Some(PathBuf::from(path?)))
    }
}

/// COM inicializado enquanto o diálogo está de pé.
///
/// A thread da janela não inicializa COM no boot — só os diálogos precisam
/// dele —, então cada abertura entra e sai do apartamento.
#[cfg(windows)]
struct ComScope {
    /// Falso quando a thread já estava em outro apartamento: aí o
    /// `CoUninitialize` seria de um `CoInitializeEx` que não é nosso.
    owned: bool,
}

#[cfg(windows)]
impl ComScope {
    fn enter() -> Result<ComScope> {
        use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

        // SAFETY: chamada por thread; o par sai no `Drop`.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result == RPC_E_CHANGED_MODE {
            return Ok(ComScope { owned: false });
        }
        if result.is_err() {
            anyhow::bail!("CoInitializeEx falhou: {result:?}");
        }
        Ok(ComScope { owned: true })
    }
}

#[cfg(windows)]
impl Drop for ComScope {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: par do `CoInitializeEx` bem-sucedido desta thread.
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Fora do Windows não há diálogo de arquivo; o backup só existe no app real.
#[cfg(not(windows))]
pub fn save_dialog(
    _title: &str,
    _default_name: &str,
    _filter: FileFilter<'_>,
) -> Result<Option<PathBuf>> {
    anyhow::bail!("diálogos de arquivo só existem no Windows")
}

#[cfg(not(windows))]
pub fn open_dialog(_title: &str, _filter: FileFilter<'_>) -> Result<Option<PathBuf>> {
    anyhow::bail!("diálogos de arquivo só existem no Windows")
}

/// Nome do mutex global que marca "já existe um app rodando".
#[cfg(windows)]
const INSTANCE_MUTEX: windows::core::PCWSTR = windows::core::w!("Global\\MacroHelldivers2");

/// Marca desta instância no sistema. Enquanto o guard existir, uma segunda
/// execução enxerga o mutex e sabe que chegou depois.
///
/// A Fase 10 completa o comportamento: trazer a janela da primeira instância
/// para a frente e encerrar a segunda. Aqui o mutex já existe para que o
/// instalador e o updater possam detectar o app em execução.
#[cfg(windows)]
pub struct InstanceLock {
    handle: windows::Win32::Foundation::HANDLE,
    /// Já havia outra instância quando esta subiu.
    pub already_running: bool,
}

#[cfg(windows)]
impl InstanceLock {
    pub fn acquire() -> Option<InstanceLock> {
        use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
        use windows::Win32::System::Threading::CreateMutexW;

        // SAFETY: nome estático; o handle é fechado no `Drop`.
        let handle = unsafe { CreateMutexW(None, true, INSTANCE_MUTEX) };
        // O erro tem que ser lido logo depois da chamada: é ele, e não o
        // handle, que diz se o mutex já existia.
        let already_running =
            windows::core::Error::from_thread().code() == ERROR_ALREADY_EXISTS.to_hresult();
        match handle {
            Ok(handle) => Some(InstanceLock {
                handle,
                already_running,
            }),
            Err(err) => {
                log::warn!("CreateMutexW falhou ({err}); seguindo sem instância única");
                None
            }
        }
    }
}

#[cfg(windows)]
impl Drop for InstanceLock {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;

        // SAFETY: handle criado por `acquire`, fechado uma vez só.
        let _ = unsafe { CloseHandle(self.handle) };
    }
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
    fn timestamps_come_out_in_the_shape_javascript_wrote() {
        use std::time::{Duration, UNIX_EPOCH};

        assert_eq!(iso8601(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            iso8601(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123)),
            "2023-11-14T22:13:20.123Z"
        );
        // 29 de fevereiro de 2024: o ano bissexto tem que aparecer inteiro.
        assert_eq!(
            iso8601(UNIX_EPOCH + Duration::from_secs(1_709_208_000)),
            "2024-02-29T12:00:00.000Z"
        );

        let now = iso8601_now();
        assert_eq!(now.len(), 24, "{now}");
        assert!(now.ends_with('Z'), "{now}");
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
