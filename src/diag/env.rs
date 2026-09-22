//! Retrato do PC para o registro do modo debug: Windows, monitor, teclado,
//! vídeo do jogo e os programas que costumam mexer em teclado, FPS ou overlay.
//!
//! Tudo aqui é leitura de estado do sistema, sem tocar no processo do jogo, e
//! cada pedaço é opcional: uma chamada recusada vira `None` no relatório em vez
//! de derrubar o retrato inteiro.

use serde::Serialize;

use crate::game_config::{self, Video};

/// Executáveis conhecidos por remapear teclado, injetar input, limitar FPS ou
/// desenhar overlay por cima do jogo. O retrato diz quais estão rodando; a
/// lista completa de processos nunca sai do PC.
pub const KNOWN_SOFTWARE: &[&str] = &[
    // Automação e remapeamento de teclado.
    "autohotkey.exe",
    "autohotkey32.exe",
    "autohotkey64.exe",
    "autohotkeyu32.exe",
    "autohotkeyu64.exe",
    "autohotkeyux.exe",
    "autoit3.exe",
    "autoit3_x64.exe",
    "powertoys.keyboardmanagerengine.exe",
    "xmousebuttoncontrol.exe",
    "rewasdservice.exe",
    "rewasdengine.exe",
    "rewasdtray.exe",
    "joytokey.exe",
    "ds4windows.exe",
    "x360ce.exe",
    "antimicrox.exe",
    // Software de periféricos, que tem macro e perfil por jogo.
    "razer synapse 3.exe",
    "razer synapse service.exe",
    "razercentralservice.exe",
    "lghub.exe",
    "lghub_agent.exe",
    "lcore.exe",
    "icue.exe",
    "steelseriesgg.exe",
    "steelseriesengine.exe",
    "wootility.exe",
    // Limite de FPS e overlay.
    "rtss.exe",
    "msiafterburner.exe",
    "nvidia overlay.exe",
    "discord.exe",
    "obs64.exe",
    "wemod.exe",
];

/// Executável do jogo.
pub const GAME_EXE: &str = "helldivers2.exe";

/// O retrato inteiro.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub app_version: &'static str,
    pub os: Option<Os>,
    pub cpu_threads: Option<usize>,
    pub ram_mb: Option<u64>,
    /// Layout de teclado da janela em foco, como o Windows o identifica
    /// (`04160416` é o ABNT2). Os atalhos comparam virtual-key, que depende
    /// dele; as teclas enviadas são scancode, que não depende.
    pub keyboard_layout: Option<String>,
    pub display: Option<Display>,
    /// O app roda elevado, como o instalador pede.
    pub elevated: Option<bool>,
    /// `LowLevelHooksTimeout` do registro: o prazo que o Windows dá ao hook
    /// de teclado antes de derrubá-lo em silêncio. `None` é o padrão do sistema.
    pub hooks_timeout_ms: Option<u32>,
    pub game: Game,
    /// Quais de [`KNOWN_SOFTWARE`] estão rodando.
    pub software: Vec<&'static str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Os {
    pub product: Option<String>,
    /// `23H2`, `24H2`...
    pub version: Option<String>,
    /// Build com revisão (`26100.4061`). A partir de 22000 é Windows 11, mesmo
    /// que o `product` diga 10 (o registro nunca foi atualizado).
    pub build: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// O `user_settings.config` existe: o jogo já foi aberto neste usuário.
    pub config_found: bool,
    pub video: Option<Video>,
    pub running: Option<bool>,
}

/// Monta o retrato agora. Pode levar alguns milissegundos (a lista de
/// processos): chamar fora do caminho quente, na thread `diag` ou na
/// exportação.
pub fn snapshot() -> Environment {
    let processes = platform::process_names();
    let running = |name: &str| {
        processes.as_ref().map(|list| {
            list.iter()
                .any(|process| process.eq_ignore_ascii_case(name))
        })
    };
    let software = KNOWN_SOFTWARE
        .iter()
        .copied()
        .filter(|name| running(name) == Some(true))
        .collect();

    let video = game_config::video();
    Environment {
        app_version: env!("CARGO_PKG_VERSION"),
        os: platform::os(),
        cpu_threads: std::thread::available_parallelism()
            .ok()
            .map(|threads| threads.get()),
        ram_mb: platform::ram_mb(),
        keyboard_layout: platform::keyboard_layout(),
        display: platform::display(),
        elevated: platform::elevated(),
        hooks_timeout_ms: platform::hooks_timeout_ms(),
        game: Game {
            config_found: video.is_some(),
            video,
            running: running(GAME_EXE),
        },
        software,
    }
}

/// Nome do executável dono de uma janela, sem caminho. É o que o registro guarda
/// das janelas que não são do jogo nem do app, no lugar do título.
#[cfg(windows)]
pub fn process_name(hwnd: windows::Win32::Foundation::HWND) -> Option<String> {
    platform::window_process(hwnd)
}

#[cfg(windows)]
mod platform {
    use windows::core::{w, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
    use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    use super::{Display, Os};

    /// Fecha o handle ao sair do escopo, aconteça o que acontecer no meio.
    struct Owned(HANDLE);

    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: handle aberto por nós e fechado uma vez só.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    const CURRENT_VERSION: PCWSTR = w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");

    pub fn os() -> Option<Os> {
        let product = reg_string(HKEY_LOCAL_MACHINE, CURRENT_VERSION, w!("ProductName"));
        let version = reg_string(HKEY_LOCAL_MACHINE, CURRENT_VERSION, w!("DisplayVersion"));
        let build =
            reg_string(HKEY_LOCAL_MACHINE, CURRENT_VERSION, w!("CurrentBuild")).map(|build| {
                match reg_dword(HKEY_LOCAL_MACHINE, CURRENT_VERSION, w!("UBR")) {
                    Some(revision) => format!("{build}.{revision}"),
                    None => build,
                }
            });
        (product.is_some() || build.is_some()).then_some(Os {
            product,
            version,
            build,
        })
    }

    pub fn hooks_timeout_ms() -> Option<u32> {
        let key = w!("Control Panel\\Desktop");
        let value = w!("LowLevelHooksTimeout");
        // A chave aparece como DWORD ou como texto, conforme quem a criou.
        reg_dword(HKEY_CURRENT_USER, key, value).or_else(|| {
            reg_string(HKEY_CURRENT_USER, key, value).and_then(|text| text.trim().parse().ok())
        })
    }

    pub fn ram_mb() -> Option<u64> {
        let mut status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        // SAFETY: estrutura nossa, com o tamanho declarado.
        unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
        Some(status.ullTotalPhys / (1024 * 1024))
    }

    pub fn keyboard_layout() -> Option<String> {
        // SAFETY: leituras de estado global; thread 0 é a nossa, se não houver
        // janela em foco.
        let layout = unsafe {
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            GetKeyboardLayout(thread)
        };
        (!layout.is_invalid()).then(|| format!("{:08X}", layout.0 as usize as u32))
    }

    pub fn display() -> Option<Display> {
        let mut mode = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        // SAFETY: `None` é o monitor primário; a estrutura declara o tamanho.
        let found =
            unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut mode) };
        found.as_bool().then_some(Display {
            width: mode.dmPelsWidth,
            height: mode.dmPelsHeight,
            refresh_hz: mode.dmDisplayFrequency,
        })
    }

    pub fn elevated() -> Option<bool> {
        let mut token = HANDLE::default();
        // SAFETY: pseudo-handle do próprio processo; o token é fechado pelo guard.
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.ok()?;
        let token = Owned(token);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        // SAFETY: buffer do tamanho declarado, vivo durante a chamada.
        unsafe {
            GetTokenInformation(
                token.0,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            )
        }
        .ok()?;
        Some(elevation.TokenIsElevated != 0)
    }

    /// Nomes de todos os processos, só para a comparação com a lista conhecida.
    pub fn process_names() -> Option<Vec<String>> {
        // SAFETY: snapshot do sistema inteiro, fechado pelo guard.
        let snapshot = Owned(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.ok()?);
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut names = Vec::new();
        // SAFETY: a entrada declara o próprio tamanho e vive durante o laço.
        let mut next = unsafe { Process32FirstW(snapshot.0, &mut entry) };
        while next.is_ok() {
            names.push(wide_to_string(&entry.szExeFile));
            // SAFETY: idem.
            next = unsafe { Process32NextW(snapshot.0, &mut entry) };
        }
        Some(names)
    }

    pub fn window_process(hwnd: HWND) -> Option<String> {
        let mut pid = 0u32;
        // SAFETY: leitura de estado de uma janela; `pid` é nosso.
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == 0 {
            return None;
        }
        // SAFETY: acesso mínimo de leitura; fechado pelo guard.
        let process =
            Owned(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);
        let mut buffer = [0u16; 260];
        let mut len = buffer.len() as u32;
        // SAFETY: o buffer e o tamanho dele vivem durante a chamada.
        unsafe {
            QueryFullProcessImageNameW(
                process.0,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut len,
            )
        }
        .ok()?;
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }

    fn reg_dword(root: HKEY, key: PCWSTR, value: PCWSTR) -> Option<u32> {
        let mut data = 0u32;
        let mut size = size_of::<u32>() as u32;
        // SAFETY: buffer de um DWORD com o tamanho declarado.
        let status = unsafe {
            RegGetValueW(
                root,
                key,
                value,
                RRF_RT_REG_DWORD,
                None,
                Some(&mut data as *mut u32 as *mut _),
                Some(&mut size),
            )
        };
        status.is_ok().then_some(data)
    }

    fn reg_string(root: HKEY, key: PCWSTR, value: PCWSTR) -> Option<String> {
        let mut buffer = [0u16; 256];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: buffer nosso com o tamanho em bytes declarado; o Windows
        // garante o terminador nulo com `RRF_RT_REG_SZ`.
        let status = unsafe {
            RegGetValueW(
                root,
                key,
                value,
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
        };
        status.is_ok().then(|| wide_to_string(&buffer))
    }

    /// Texto UTF-16 terminado em nulo, cortado no terminador.
    fn wide_to_string(wide: &[u16]) -> String {
        let len = wide
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(wide.len());
        String::from_utf16_lossy(&wide[..len])
    }
}

/// Fora do Windows não há o que retratar além do que a biblioteca padrão sabe.
#[cfg(not(windows))]
mod platform {
    use super::{Display, Os};

    pub fn os() -> Option<Os> {
        None
    }

    pub fn hooks_timeout_ms() -> Option<u32> {
        None
    }

    pub fn ram_mb() -> Option<u64> {
        None
    }

    pub fn keyboard_layout() -> Option<String> {
        None
    }

    pub fn display() -> Option<Display> {
        None
    }

    pub fn elevated() -> Option<bool> {
        None
    }

    pub fn process_names() -> Option<Vec<String>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_list_is_lowercase_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in KNOWN_SOFTWARE {
            assert_eq!(*name, name.to_lowercase(), "{name}");
            assert!(name.ends_with(".exe"), "{name}");
            assert!(seen.insert(*name), "repetido: {name}");
        }
    }

    #[test]
    fn a_snapshot_never_fails_even_without_the_game() {
        let env = snapshot();
        assert_eq!(env.app_version, env!("CARGO_PKG_VERSION"));
        assert!(env.cpu_threads.is_some_and(|threads| threads > 0));
        // A máquina de teste não tem o jogo: o retrato diz isso, sem estourar.
        assert!(!env.game.config_found);
        assert!(env.game.video.is_none());
    }
}
