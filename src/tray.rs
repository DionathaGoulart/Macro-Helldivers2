//! Ícone na bandeja do sistema, em Win32 cru.
//!
//! É o que mantém o app útil com a janela fechada: minimizar e fechar escondem
//! a janela, os hooks continuam vivos e o ícone é o caminho de volta. A v1
//! (`legacy/src/main/index.js` ~385-433) tinha a mesma regra, junto com a
//! proteção que copiamos aqui: **sem ícone, fechar encerra de verdade** — uma
//! janela escondida sem bandeja seria irrecuperável.
//!
//! O ícone vive na thread da janela principal, porque é ela que recebe o
//! callback (`uCallbackMessage`) e é dela o `HWND` que o menu precisa.

/// Texto do menu de contexto, no idioma atual.
#[derive(Debug, Clone, Copy)]
pub struct MenuLabels<'a> {
    pub open: &'a str,
    pub exit: &'a str,
}

/// O que o usuário escolheu no menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Trazer a janela de volta.
    Open,
    /// Encerrar o app de verdade.
    Exit,
}

#[cfg(windows)]
pub use platform::{show_menu, Tray};

#[cfg(windows)]
mod platform {
    use anyhow::{Context, Result};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, HBITMAP,
    };
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
        PostMessageW, SetForegroundWindow, TrackPopupMenu, HICON, ICONINFO, MF_SEPARATOR,
        MF_STRING, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
    };

    use super::{Command, MenuLabels};
    use crate::gfx::images::{self, Alpha};
    use crate::util;

    /// Arquivo do ícone. É a versão de 64px — decodificar o `icon.png` de 1024
    /// para desenhar 16px na barra de tarefas seria desperdício (mesma escolha
    /// da v1).
    const ICON_ASSET: &str = "icons/tray.png";

    /// Id do ícone dentro da janela. Um só, então qualquer valor serve — mas
    /// tem que ser o mesmo na adição e na remoção.
    const ICON_ID: u32 = 1;

    /// Comandos do menu. Zero é reservado: `TPM_RETURNCMD` devolve 0 quando o
    /// usuário fecha o menu sem escolher nada.
    const CMD_OPEN: usize = 1;
    const CMD_EXIT: usize = 2;

    /// Ícone vivo na bandeja. Sai de lá no `Drop`, aconteça o que acontecer com
    /// a janela.
    pub struct Tray {
        hwnd: HWND,
        icon: HICON,
    }

    impl Tray {
        /// Põe o ícone na bandeja. `message` é a mensagem `WM_APP` que a janela
        /// vai receber a cada evento de mouse sobre ele.
        ///
        /// `None` quando o ícone não pôde ser criado ou registrado — e aí a
        /// janela volta a encerrar o app ao ser fechada.
        pub fn new(hwnd: HWND, message: u32, tooltip: &str) -> Option<Tray> {
            let icon = match create_icon() {
                Ok(icon) => icon,
                Err(err) => {
                    log::error!("ícone da bandeja não pôde ser criado: {err:#}");
                    return None;
                }
            };

            let mut data = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: ICON_ID,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: message,
                hIcon: icon,
                ..Default::default()
            };
            // A dica é um buffer de tamanho fixo: o texto entra truncado, com
            // espaço garantido para o terminador.
            let limit = data.szTip.len() - 1;
            for (slot, unit) in data
                .szTip
                .iter_mut()
                .zip(tooltip.encode_utf16().take(limit).chain(std::iter::once(0)))
            {
                *slot = unit;
            }

            // SAFETY: a estrutura vive durante a chamada e declara o próprio
            // tamanho; o ícone é destruído no `Drop`.
            if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
                log::error!(
                    "Shell_NotifyIconW(NIM_ADD) recusado; a janela vai fechar em vez de recolher"
                );
                // SAFETY: ícone recém-criado, destruído uma vez só.
                let _ = unsafe { DestroyIcon(icon) };
                return None;
            }
            log::info!("ícone da bandeja registrado");
            Some(Tray { hwnd, icon })
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            let data = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: ICON_ID,
                ..Default::default()
            };
            // SAFETY: mesma janela e mesmo id da adição.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
                let _ = DestroyIcon(self.icon);
            }
        }
    }

    /// Abre o menu de contexto no cursor e devolve a escolha.
    ///
    /// Fica fora do [`Tray`] de propósito: `TrackPopupMenu` roda o **próprio**
    /// loop de mensagens, que reentra no `WndProc` da janela. Sem precisar do
    /// `&Tray`, quem chama não tem nenhum empréstimo do estado da janela vivo
    /// enquanto o menu está aberto.
    pub fn show_menu(hwnd: HWND, labels: MenuLabels<'_>) -> Option<Command> {
        let mut point = POINT::default();
        // SAFETY: ponto próprio, vivo durante a chamada.
        if unsafe { GetCursorPos(&mut point) }.is_err() {
            return None;
        }

        // SAFETY: menu criado aqui e destruído antes de sair.
        let menu = unsafe { CreatePopupMenu() }.ok()?;
        let (open, exit) = (wide(labels.open), wide(labels.exit));
        // SAFETY: as strings vivem até o fim da função, depois do `TrackPopupMenu`.
        let built = unsafe {
            AppendMenuW(menu, MF_STRING, CMD_OPEN, PCWSTR(open.as_ptr()))
                .and_then(|()| AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()))
                .and_then(|()| AppendMenuW(menu, MF_STRING, CMD_EXIT, PCWSTR(exit.as_ptr())))
        };
        if let Err(err) = built {
            log::warn!("menu da bandeja não pôde ser montado: {err}");
            // SAFETY: menu ainda não exibido.
            let _ = unsafe { DestroyMenu(menu) };
            return None;
        }

        // O menu de uma bandeja só desaparece ao clicar fora se a janela dona
        // estiver em primeiro plano — é a receita da própria documentação da
        // API, junto com o `WM_NULL` depois de fechar.
        // SAFETY: janela viva; trazer para a frente uma janela escondida não a
        // mostra, só a torna a ativa.
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }

        // SAFETY: `TPM_RETURNCMD` devolve o id escolhido em vez de postar
        // `WM_COMMAND`, e `TPM_NONOTIFY` cala as notificações do menu.
        let choice = unsafe {
            TrackPopupMenu(
                menu,
                TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
                point.x,
                point.y,
                None,
                hwnd,
                None,
            )
        };
        // SAFETY: menu já fechado; a janela recebe o empurrão que a API pede.
        unsafe {
            let _ = DestroyMenu(menu);
            let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        }

        match choice.0 as usize {
            CMD_OPEN => Some(Command::Open),
            CMD_EXIT => Some(Command::Exit),
            // Zero: o usuário clicou fora e desistiu.
            _ => None,
        }
    }

    /// Converte o PNG da bandeja num `HICON`.
    ///
    /// O ícone é um bitmap de 32 bits com alfa direto (o Win32 compõe sozinho,
    /// diferente do Direct2D) mais uma máscara monocromática zerada, que é o
    /// combinado que o `CreateIconIndirect` espera de um ícone com canal alfa.
    fn create_icon() -> Result<HICON> {
        let path = util::asset_path(ICON_ASSET);
        let decoded = images::decode_alpha(&path, Alpha::Straight)?;

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: decoded.width as i32,
                // Negativo: linhas de cima para baixo, na ordem em que o
                // decodificador entrega.
                biHeight: -(decoded.height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: a descrição e o ponteiro de saída vivem durante a chamada; o
        // bitmap é destruído no fim da função.
        let color = unsafe {
            CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut pixels, None, 0)
                .context("CreateDIBSection")?
        };
        let color = BitmapGuard(color);
        if pixels.is_null() {
            anyhow::bail!("CreateDIBSection não devolveu buffer");
        }
        // SAFETY: o buffer tem `width * height * 4` bytes, o mesmo que a
        // decodificação produziu.
        unsafe {
            std::ptr::copy_nonoverlapping(
                decoded.bgra.as_ptr(),
                pixels as *mut u8,
                decoded.bgra.len(),
            );
        }

        // SAFETY: máscara monocromática do mesmo tamanho, sem dados iniciais —
        // zerada, que é o que faz o alfa do bitmap colorido mandar sozinho.
        let mask = unsafe { CreateBitmap(decoded.width as i32, decoded.height as i32, 1, 1, None) };
        if mask.is_invalid() {
            anyhow::bail!("CreateBitmap da máscara falhou");
        }
        let mask = BitmapGuard(mask);

        let icon_info = ICONINFO {
            fIcon: true.into(),
            hbmMask: mask.0,
            hbmColor: color.0,
            ..Default::default()
        };
        // SAFETY: os dois bitmaps vivem durante a chamada; o `CreateIconIndirect`
        // copia o que precisa, e os guards os soltam depois.
        let icon = unsafe { CreateIconIndirect(&icon_info) }.context("CreateIconIndirect")?;
        Ok(icon)
    }

    /// Solta um bitmap GDI mesmo se a criação do ícone falhar no meio.
    struct BitmapGuard(HBITMAP);

    impl Drop for BitmapGuard {
        fn drop(&mut self) {
            // SAFETY: bitmap criado aqui, destruído uma vez só.
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }
}
