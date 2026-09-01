//! Atualização pelo GitHub Releases.
//!
//! A v1 usava o `electron-updater` (`legacy/src/main/index.js` ~588-663), que
//! arrastava js-yaml, semver e lodash só para ler um JSON e baixar um arquivo.
//! Aqui o ciclo é o mesmo, escrito à mão: consulta o release mais recente,
//! compara versões, e — **só quando o usuário manda** — baixa o instalador e o
//! executa.
//!
//! Duas regras vêm inteiras da v1:
//!
//! - **Nunca com o jogo em foco.** Rede, disco e CPU no meio de uma missão não
//!   valem a pena. Se o app sobe com o jogo na frente, o check fica adiado e
//!   dispara na primeira perda de foco (o `hasCheckedUpdates` de lá é o
//!   [`State::auto_checked`] daqui).
//! - **Download manual.** Puxar o instalador sozinho é o que a v1 desligou com
//!   `autoDownload = false`.
//!
//! Todo o trabalho de rede roda em worker efêmero; o resultado volta para a
//! janela por [`UiEvent::UpdateStatus`].

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;

use crate::shared::{Shared, UiEvent, UpdateStatus};
use crate::util;

/// Repositório de onde saem os releases.
pub const REPO: &str = "DionathaGoulart/Macro-Helldivers2";

/// Versão deste executável, no formato do `Cargo.toml`.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A API do GitHub recusa pedido sem identificação.
const USER_AGENT: &str = concat!("macro-helldivers2/", env!("CARGO_PKG_VERSION"));

/// O check é um JSON pequeno: prazo curto, como o da consulta de estatísticas.
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// Pedaço lido por vez do instalador.
const CHUNK_BYTES: usize = 64 * 1024;

/// Passo mínimo entre dois avisos de progresso. Cada aviso acorda a janela para
/// uma repintura; um por cento é o que a barra da v1 mostrava.
const PROGRESS_STEP: f32 = 1.0;

/// Nome usado quando o asset vem com um nome que não serve de arquivo.
const FALLBACK_INSTALLER: &str = "macro-helldivers2-setup.exe";

/// Um release do GitHub, com só o que o app usa.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Release {
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// Um arquivo publicado no release.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Asset {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub browser_download_url: String,
    /// Tamanho anunciado; serve de denominador quando a resposta do download
    /// não traz `Content-Length`.
    #[serde(default)]
    pub size: u64,
}

impl Release {
    /// Versão da tag (`v2.0.1` → `2.0.1`).
    pub fn version(&self) -> Option<Version> {
        parse_version(&self.tag_name)
    }

    /// O instalador do release: o primeiro `.exe` publicado.
    pub fn installer(&self) -> Option<&Asset> {
        self.assets
            .iter()
            .find(|asset| has_exe_extension(&asset.name) && !asset.browser_download_url.is_empty())
    }
}

/// Endereço do release mais recente.
pub fn latest_url() -> String {
    format!("https://api.github.com/repos/{REPO}/releases/latest")
}

/// Versão a partir de uma tag, com ou sem o `v` da frente.
pub fn parse_version(text: &str) -> Option<Version> {
    let trimmed = text.trim();
    let bare = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);
    Version::parse(bare).ok()
}

/// A tag é mais nova que a versão em execução?
///
/// Versão que não parseia nunca conta como novidade: oferecer download por
/// causa de uma tag estranha seria pior que deixar passar.
pub fn is_newer(tag: &str, current: &str) -> bool {
    match (parse_version(tag), parse_version(current)) {
        (Some(remote), Some(local)) => remote > local,
        _ => false,
    }
}

/// Versão mostrada na tela: a tag sem o `v`.
pub fn display_version(tag: &str) -> String {
    let trimmed = tag.trim();
    trimmed
        .strip_prefix(['v', 'V'])
        .unwrap_or(trimmed)
        .to_string()
}

fn has_exe_extension(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}

/// Estado do ciclo de atualização, compartilhado entre a janela e os workers.
struct State {
    /// Release do último check com novidade — é dele que sai o download.
    latest: Mutex<Option<Release>>,
    /// Instalador já no disco, esperando o "Reiniciar Agora".
    installer: Mutex<Option<PathBuf>>,
    /// Já houve um check automático nesta sessão.
    auto_checked: AtomicBool,
    /// Um worker de rede por vez: dois downloads simultâneos escreveriam no
    /// mesmo arquivo.
    busy: AtomicBool,
}

static STATE: LazyLock<State> = LazyLock::new(|| State {
    latest: Mutex::new(None),
    installer: Mutex::new(None),
    auto_checked: AtomicBool::new(false),
    busy: AtomicBool::new(false),
});

/// Trava que sobrevive a um worker que entrou em panic: o dado é substituível,
/// e ficar sem updater até o próximo boot seria pior.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

/// Check automático da sessão: no máximo um, e nunca com o jogo em foco.
///
/// Chamado no boot (depois da primeira pintura) e na primeira perda de foco. Se
/// o jogo estava na frente, a chamada não gasta a única tentativa — é o que faz
/// o check adiado da v1 acontecer mais tarde.
pub fn auto_check(shared: &Arc<Shared>) {
    if shared.is_game_focused() {
        log::debug!("check de atualização adiado: o jogo está em foco");
        return;
    }
    if STATE.auto_checked.swap(true, Ordering::Relaxed) {
        return;
    }
    check(shared);
}

/// Consulta o release mais recente num worker.
pub fn check(shared: &Arc<Shared>) {
    spawn(shared, "updater-check", |shared| {
        shared.send_ui(UiEvent::UpdateStatus(UpdateStatus::Checking));
        let status = match fetch_latest() {
            Ok(release) => available_status(release),
            Err(err) => {
                log::warn!("check de atualização falhou: {err:#}");
                UpdateStatus::Error {
                    message: err.to_string(),
                }
            }
        };
        shared.send_ui(UiEvent::UpdateStatus(status));
    });
}

/// Guarda o release e diz o que a tela mostra a respeito dele.
fn available_status(release: Release) -> UpdateStatus {
    if !is_newer(&release.tag_name, CURRENT_VERSION) {
        log::info!(
            "app atualizado (release mais recente: {})",
            release.tag_name
        );
        return UpdateStatus::UpToDate;
    }
    if release.installer().is_none() {
        // Release publicado sem o `.exe` (build em andamento, por exemplo):
        // anunciar novidade que não dá para baixar só irritaria.
        log::warn!("release {} não tem instalador .exe", release.tag_name);
        return UpdateStatus::UpToDate;
    }

    let version = display_version(&release.tag_name);
    log::info!("atualização disponível: {version}");
    *lock(&STATE.latest) = Some(release);
    UpdateStatus::Available { version }
}

/// Baixa o instalador do release encontrado. Só sai daqui por clique do
/// usuário (a v1 desligou o download automático pelo mesmo motivo).
pub fn download(shared: &Arc<Shared>) {
    let Some(release) = lock(&STATE.latest).clone() else {
        log::warn!("download pedido sem release em mãos");
        return;
    };

    spawn(shared, "updater-download", move |shared| {
        shared.send_ui(UiEvent::UpdateStatus(UpdateStatus::Downloading {
            percent: 0.0,
        }));
        let progress = |percent| {
            shared.send_ui(UiEvent::UpdateStatus(UpdateStatus::Downloading { percent }));
        };
        let status = match download_installer(&release, progress) {
            Ok(path) => {
                log::info!("instalador baixado em {}", path.display());
                *lock(&STATE.installer) = Some(path);
                UpdateStatus::Ready {
                    version: display_version(&release.tag_name),
                }
            }
            Err(err) => {
                log::warn!("download da atualização falhou: {err:#}");
                UpdateStatus::Error {
                    message: err.to_string(),
                }
            }
        };
        shared.send_ui(UiEvent::UpdateStatus(status));
    });
}

/// Roda o instalador baixado.
///
/// Quem chama encerra o app logo depois: o instalador precisa substituir o
/// executável que está rodando. Como o app já sobe elevado (manifesto
/// `requireAdministrator`), o processo filho herda a elevação e o UAC não
/// aparece de novo.
pub fn install() -> Result<()> {
    let path = lock(&STATE.installer)
        .clone()
        .context("nenhum instalador baixado")?;
    std::process::Command::new(&path)
        .spawn()
        .with_context(|| format!("não foi possível executar {}", path.display()))?;
    Ok(())
}

/// Sobe um worker de rede, se já não houver um rodando.
fn spawn(shared: &Arc<Shared>, name: &str, job: impl FnOnce(Arc<Shared>) + Send + 'static) {
    if STATE.busy.swap(true, Ordering::SeqCst) {
        log::debug!("{name} ignorado: já há uma operação de atualização em curso");
        return;
    }

    let worker = Arc::clone(shared);
    let spawned = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            // A trava é solta aconteça o que acontecer com o worker.
            let _busy = BusyGuard;
            job(worker);
        });

    if let Err(err) = spawned {
        log::warn!("worker de atualização não subiu: {err}");
        STATE.busy.store(false, Ordering::SeqCst);
        shared.send_ui(UiEvent::UpdateStatus(UpdateStatus::Error {
            message: err.to_string(),
        }));
    }
}

struct BusyGuard;

impl Drop for BusyGuard {
    fn drop(&mut self) {
        STATE.busy.store(false, Ordering::SeqCst);
    }
}

/// Lê o release mais recente da API do GitHub.
pub fn fetch_latest() -> Result<Release> {
    let url = latest_url();
    let release = util::http_agent(Some(CHECK_TIMEOUT))
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .with_context(|| format!("GET {url}"))?
        .body_mut()
        .read_json::<Release>()
        .context("resposta inesperada da API do GitHub")?;
    Ok(release)
}

/// Baixa o instalador para a pasta temporária, avisando o progresso.
fn download_installer(release: &Release, mut progress: impl FnMut(f32)) -> Result<PathBuf> {
    let asset = release.installer().context("release sem instalador .exe")?;

    // Sem prazo global: o tamanho do arquivo é que manda no tempo.
    let mut response = util::http_agent(None)
        .get(&asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("GET {}", asset.browser_download_url))?;

    let total = content_length(&response).unwrap_or(asset.size);
    let dir = download_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("não foi possível criar {}", dir.display()))?;
    let path = dir.join(installer_file_name(&asset.name));

    let mut file = std::fs::File::create(&path)
        .with_context(|| format!("não foi possível criar {}", path.display()))?;
    let mut reader = response.body_mut().as_reader();
    let mut buffer = vec![0u8; CHUNK_BYTES];
    let mut written: u64 = 0;
    let mut reported = 0.0;

    loop {
        let read = reader
            .read(&mut buffer)
            .context("falha ao ler o download")?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .with_context(|| format!("falha ao escrever {}", path.display()))?;
        written += read as u64;

        let percent = percent_of(written, total);
        if percent - reported >= PROGRESS_STEP {
            reported = percent;
            progress(percent);
        }
    }
    file.flush().context("falha ao fechar o instalador")?;

    if written == 0 {
        anyhow::bail!("o download veio vazio");
    }
    Ok(path)
}

fn content_length(response: &ureq::http::Response<ureq::Body>) -> Option<u64> {
    response
        .headers()
        .get(ureq::http::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

/// Onde o instalador baixado espera pelo clique de instalar.
fn download_dir() -> PathBuf {
    std::env::temp_dir().join(util::APP_DIR_NAME)
}

/// Nome de arquivo seguro para o asset.
///
/// O nome vem do release, que é dado de fora: só o trecho depois da última
/// barra entra, e qualquer coisa que não sirva de nome de arquivo vira o
/// padrão. Sem isso um asset chamado `..\algo.exe` escreveria fora da pasta
/// temporária. As duas barras são tratadas na mão porque o alvo é o Windows,
/// onde as duas separam caminho — e porque o teste roda no host, onde a
/// contrabarra é um caractere comum.
fn installer_file_name(asset_name: &str) -> PathBuf {
    let name = asset_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    // Começar com ponto cobre o `..`; os dois-pontos cortam letra de unidade e
    // fluxo alternativo de dados (`arquivo.exe:oculto`).
    if name.starts_with('.') || name.contains(':') || !has_exe_extension(name) {
        return PathBuf::from(FALLBACK_INSTALLER);
    }
    PathBuf::from(name)
}

/// Progresso em 0..=100. Total desconhecido fica em zero — barra parada é
/// melhor que barra mentindo.
fn percent_of(done: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    ((done as f64 / total as f64) as f32 * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorte de uma resposta real de `releases/latest`, com os campos que o
    /// app lê e alguns dos que ele ignora.
    const RELEASE_JSON: &[u8] = br#"{
        "url": "https://api.github.com/repos/DionathaGoulart/Macro-Helldivers2/releases/1",
        "tag_name": "v2.1.0",
        "name": "2.1.0",
        "draft": false,
        "prerelease": false,
        "assets": [
            {
                "name": "latest.yml",
                "browser_download_url": "https://github.com/x/releases/download/v2.1.0/latest.yml",
                "size": 372
            },
            {
                "name": "Macro-Helldivers-2-Setup-2.1.0.exe",
                "browser_download_url": "https://github.com/x/releases/download/v2.1.0/Setup.exe",
                "size": 4194304
            }
        ]
    }"#;

    fn release() -> Release {
        serde_json::from_slice(RELEASE_JSON).expect("release do GitHub")
    }

    #[test]
    fn a_release_parses_with_the_fields_the_updater_uses() {
        let release = release();
        assert_eq!(release.tag_name, "v2.1.0");
        assert_eq!(release.version(), Some(Version::new(2, 1, 0)));

        let installer = release.installer().expect("asset .exe");
        assert_eq!(installer.name, "Macro-Helldivers-2-Setup-2.1.0.exe");
        assert_eq!(installer.size, 4_194_304);
    }

    #[test]
    fn a_release_without_an_installer_offers_nothing() {
        let release: Release =
            serde_json::from_slice(br#"{"tag_name":"v9.0.0","assets":[{"name":"notes.txt"}]}"#)
                .unwrap();
        assert!(release.installer().is_none());
        // E o app segue se dizendo atualizado, em vez de anunciar um download
        // que não existe.
        assert_eq!(available_status(release), UpdateStatus::UpToDate);
    }

    #[test]
    fn campos_ausentes_nao_derrubam_a_leitura() {
        let release: Release = serde_json::from_slice(b"{}").unwrap();
        assert!(release.tag_name.is_empty());
        assert!(release.version().is_none());
        assert!(release.installer().is_none());
    }

    #[test]
    fn tags_parse_with_and_without_the_v() {
        assert_eq!(parse_version("v2.0.0"), Some(Version::new(2, 0, 0)));
        assert_eq!(parse_version(" 2.0.0 "), Some(Version::new(2, 0, 0)));
        assert_eq!(parse_version("V2.0.0"), Some(Version::new(2, 0, 0)));
        assert!(parse_version("beta").is_none());
        assert!(parse_version("").is_none());

        assert_eq!(display_version("v2.1.0"), "2.1.0");
        assert_eq!(display_version("2.1.0"), "2.1.0");
    }

    #[test]
    fn only_a_higher_version_counts_as_an_update() {
        assert!(is_newer("v2.0.1", "2.0.0"));
        assert!(is_newer("v2.1.0", "2.0.9"));
        assert!(!is_newer("v2.0.0", "2.0.0"));
        assert!(!is_newer("v1.9.9", "2.0.0"));

        // Pré-lançamento vem antes do final, como no Cargo: quem roda o
        // "2.0.0-dev" do desenvolvimento recebe o 2.0.0.
        assert!(is_newer("v2.0.0", "2.0.0-dev"));
        assert!(!is_newer("v2.0.0-beta.1", "2.0.0"));
        assert!(is_newer("v2.0.0-beta.2", "2.0.0-beta.1"));

        // Tag que não parseia nunca vira oferta de download.
        assert!(!is_newer("latest", "2.0.0"));
        assert!(!is_newer("v2.0.1", "não-é-versão"));
    }

    /// A versão do `Cargo.toml` precisa continuar comparável — é o outro lado
    /// de toda comparação de update.
    #[test]
    fn the_running_version_is_a_valid_semver() {
        assert!(
            parse_version(CURRENT_VERSION).is_some(),
            "{CURRENT_VERSION}"
        );
    }

    #[test]
    fn the_api_url_points_at_the_projects_repository() {
        let url = latest_url();
        assert_eq!(
            url,
            "https://api.github.com/repos/DionathaGoulart/Macro-Helldivers2/releases/latest"
        );
    }

    #[test]
    fn the_asset_name_never_escapes_the_temp_folder() {
        assert_eq!(
            installer_file_name("Macro-Setup-2.1.0.exe"),
            PathBuf::from("Macro-Setup-2.1.0.exe")
        );
        // Só o trecho depois da última barra entra…
        assert_eq!(
            installer_file_name(r"..\..\evil.exe"),
            PathBuf::from("evil.exe")
        );
        assert_eq!(
            installer_file_name("../../evil.exe"),
            PathBuf::from("evil.exe")
        );
        assert_eq!(
            installer_file_name(r"C:\Windows\algo.exe"),
            PathBuf::from("algo.exe")
        );
        // …e o que não serve de nome de arquivo cai no padrão.
        for hostile in ["", "..", "setup.txt", "algo.exe:oculto", ".exe"] {
            assert_eq!(
                installer_file_name(hostile),
                PathBuf::from(FALLBACK_INSTALLER),
                "{hostile}"
            );
        }
    }

    #[test]
    fn progress_stays_inside_the_bar() {
        assert_eq!(percent_of(0, 100), 0.0);
        assert_eq!(percent_of(50, 200), 25.0);
        assert_eq!(percent_of(100, 100), 100.0);
        // Servidor que manda mais do que anunciou não estoura a barra, e total
        // desconhecido não inventa progresso.
        assert_eq!(percent_of(150, 100), 100.0);
        assert_eq!(percent_of(10, 0), 0.0);
    }

    /// Consulta de verdade — fora do `cargo test` padrão porque depende de rede
    /// e do repositório ter release publicado.
    #[test]
    #[ignore = "depende de rede"]
    fn a_live_check_reaches_github() {
        let release = fetch_latest().expect("releases/latest");
        assert!(!release.tag_name.is_empty());
    }
}
