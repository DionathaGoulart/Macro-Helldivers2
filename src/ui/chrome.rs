//! Moldura da janela principal: topbar, banner de aviso, rodapé, o diálogo de
//! atualização e a textura de scanline, tudo o que está na tela e não é de
//! nenhuma aba.
//!
//! Vive fora de `window.rs` para ser lógica pura: a janela só junta o estado,
//! e a montagem roda (e é testada) no host, como as abas.

use crate::i18n::Tr;
use crate::shared::UpdateStatus;
use crate::ui::modal::{self, Modal};
use crate::ui::theme;
use crate::ui::toolkit::{id, Align, Id, Measure, Rect, Ui};
use crate::ui::widgets::{self, styles, Glyph, Tone, TAB_BAR_HEIGHT};

/// Rodapé com a versão, o estado do jogo e o updater.
pub const FOOTER_HEIGHT: f32 = 36.0;
/// Banner de "tela cheia exclusiva" abaixo da topbar.
pub const WARNING_HEIGHT: f32 = 76.0;
const PAGE_PADDING: f32 = 24.0;
/// Espaço entre os pedaços do rodapé.
const FOOTER_GAP: f32 = 10.0;
/// Quadrado de presença (`presence-dot`).
const PRESENCE: f32 = 10.0;

/// O que a moldura precisa saber do app.
pub struct Chrome<'a> {
    pub tr: &'static Tr,
    /// Aba da frente.
    pub tab: usize,
    pub game_focused: bool,
    /// Jogo em "Tela Cheia" exclusiva. Só aparece com o jogo em foco.
    pub fullscreen_warning: bool,
    pub update: &'a UpdateStatus,
    /// O usuário clicou "Depois": o diálogo não volta, o rodapé assume.
    pub update_deferred: bool,
    /// Textura de scanline (§4.4). Some com o movimento reduzido do sistema.
    pub scanlines: bool,
}

/// Botão do updater no rodapé: baixa quando há novidade, instala quando o
/// arquivo já está no disco, tenta de novo depois de um erro.
pub fn update_action_id() -> Id {
    id("footer.update")
}

/// Monta a tela inteira em `size` (DIP). A moldura vem por fora e `content`
/// desenha a aba da frente no retângulo que sobra.
pub fn build(
    ui: &mut Ui,
    measure: &mut dyn Measure,
    size: (f32, f32),
    chrome: &Chrome,
    content: impl FnOnce(&mut Ui, &mut dyn Measure, Rect),
) {
    let palette = theme::palette();
    let area = Rect::new(0.0, 0.0, size.0, size.1);
    let mut body = area;
    let header = body.cut_top(TAB_BAR_HEIGHT);
    let footer = body.cut_bottom(FOOTER_HEIGHT);

    // O fundo de página por baixo de tudo: as abas desenham por cima dele.
    ui.fill(body, palette.base_100);
    let tabs = [
        chrome.tr.tabs.macro_tab,
        chrome.tr.tabs.build,
        chrome.tr.tabs.settings,
    ];
    widgets::tab_bar(ui, measure, header, &tabs, chrome.tab);

    if chrome.fullscreen_warning && chrome.game_focused {
        let banner = body.cut_top(WARNING_HEIGHT);
        widgets::alert(
            ui,
            banner.inset_xy(PAGE_PADDING, 10.0),
            palette.warning,
            chrome.tr.overlay.warning_title,
            chrome.tr.overlay.fullscreen_warning,
        );
    }

    content(ui, measure, body);
    self::footer(ui, measure, footer, chrome);

    // Por último, sobre tudo: enquanto o diálogo está aberto, o véu é quem
    // responde a qualquer clique fora dele. Depois do "Depois" ele não volta;
    // o botão do rodapé assume.
    if let UpdateStatus::Ready { version } = chrome.update {
        if !chrome.update_deferred {
            let kicker = format!("v{version}");
            let dialog = Modal {
                file: "update.exe",
                kicker: &kicker,
                title: chrome.tr.update.title,
                body: chrome.tr.update.body,
                primary: chrome.tr.update.restart_now,
                secondary: chrome.tr.update.later,
            };
            modal::show(ui, measure, area, &dialog);
        }
    }

    if chrome.scanlines {
        ui.scanlines(area, theme::SCANLINE_STEP, palette.scanline);
    }
}

/// Rodapé: versão e presença do jogo à esquerda, updater à direita. Linha de
/// cima de 2px sobre o fundo de página, tudo em micro-texto.
fn footer(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, chrome: &Chrome) {
    let palette = theme::palette();
    ui.fill(rect, palette.base_100);
    ui.fill(rect.with_h(theme::BORDER), palette.base_300);

    let mut row = Rect::new(
        rect.x + PAGE_PADDING,
        rect.y + theme::BORDER,
        rect.w - PAGE_PADDING * 2.0,
        rect.h - theme::BORDER,
    );
    update_footer(ui, measure, &mut row, chrome);
    status_footer(ui, measure, &mut row, chrome);
}

/// Versão do app e o quadrado de presença do jogo.
fn status_footer(ui: &mut Ui, measure: &mut dyn Measure, row: &mut Rect, chrome: &Chrome) {
    let palette = theme::palette();
    let style = styles::micro().middle();

    let version = format!(
        "{} v{}",
        chrome.tr.settings.version,
        env!("CARGO_PKG_VERSION")
    )
    .to_uppercase();
    let width = measure.text_size(&version, style, f32::INFINITY).0;
    ui.text(row.cut_left(width), version, style, palette.muted);
    row.cut_left(FOOTER_GAP);
    ui.text(row.cut_left(8.0), "\u{00B7}", style, palette.muted);
    row.cut_left(FOOTER_GAP);

    // Presença: cheio de sucesso com o jogo na frente, vazio fora dele. O
    // rótulo fica em conteúdo: a página não recebe texto de status (§2.4).
    let square = row.cut_left(PRESENCE).middle_row(PRESENCE);
    let fill = if chrome.game_focused {
        palette.success.fill
    } else {
        palette.base_100
    };
    widgets::status_square(ui, square, fill);
    row.cut_left(8.0);

    let status = if chrome.game_focused {
        chrome.tr.settings.game_active
    } else {
        chrome.tr.settings.game_inactive
    }
    .to_uppercase();
    let width = measure.text_size(&status, style, f32::INFINITY).0;
    ui.text(
        row.cut_left(width),
        status,
        style,
        if chrome.game_focused {
            palette.content
        } else {
            palette.muted
        },
    );
}

/// Canto direito do rodapé: o andamento do updater e, quando há o que fazer,
/// o `icon-btn` que baixa, instala ou tenta de novo. Montado da direita para a
/// esquerda, então o botão vem antes do texto.
fn update_footer(ui: &mut Ui, measure: &mut dyn Measure, row: &mut Rect, chrome: &Chrome) {
    let palette = theme::palette();
    let text = &chrome.tr.settings;

    let action = match chrome.update {
        UpdateStatus::Ready { .. } => Some((text.update_ready, Tone::Accent)),
        // O download só começa no clique: nada de puxar o instalador no meio
        // de uma partida.
        UpdateStatus::Available { .. } => Some((text.update_download, Tone::Plain)),
        // Sem o botão, um erro de rede deixaria o updater morto até o boot.
        UpdateStatus::Error { .. } => Some((text.update_retry, Tone::Plain)),
        _ => None,
    };
    if let Some((label, tone)) = action {
        let width = widgets::icon_btn_width(measure, label);
        let button = row
            .cut_right(width)
            .middle_row(widgets::ICON_BTN_HEIGHT - 2.0);
        widgets::icon_btn(ui, update_action_id(), button, Glyph::Text(label), tone);
        row.cut_right(FOOTER_GAP);
    }
    // Pronto para instalar: o botão em accent já diz tudo.
    if let UpdateStatus::Ready { .. } = chrome.update {
        return;
    }

    let style = styles::micro().middle().align(Align::End);
    let label = update_label(chrome.update, chrome.tr);
    let busy = matches!(
        chrome.update,
        UpdateStatus::Checking | UpdateStatus::Downloading { .. }
    );
    // O caret do §4.5 no lugar de spinner: reservado mesmo apagado, o texto
    // não pula quando ele pisca.
    let width = measure
        .text_size(&format!("{label}_"), style, f32::INFINITY)
        .0;
    let slot = row.cut_right(width);
    if busy {
        widgets::caret_text(ui, measure, slot, &label, style, palette.muted);
    } else {
        ui.text(slot, label, style, palette.muted);
    }

    if let UpdateStatus::Error { .. } = chrome.update {
        row.cut_right(8.0);
        let square = row.cut_right(PRESENCE).middle_row(PRESENCE);
        widgets::status_square(ui, square, palette.error.fill);
    }
}

/// Texto do andamento do updater.
pub fn update_label(status: &UpdateStatus, tr: &Tr) -> String {
    let text = &tr.settings;
    match status {
        UpdateStatus::Checking => text.update_checking.to_uppercase(),
        UpdateStatus::Available { .. } => text.update_available.to_uppercase(),
        UpdateStatus::Downloading { percent } => format!(
            "{} {}%",
            text.update_downloading.trim_end_matches('.').to_uppercase(),
            percent.round()
        ),
        UpdateStatus::UpToDate => text.update_up_to_date.to_uppercase(),
        UpdateStatus::Error { .. } => text.update_error.to_uppercase(),
        // "Atualizado" é o texto de repouso, e também o que sobra depois de o
        // usuário adiar a instalação.
        UpdateStatus::Idle | UpdateStatus::Ready { .. } => text.updated.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n;
    use crate::settings::Language;
    use crate::ui::toolkit::{TextStyle, Visual};

    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, max: f32) -> (f32, f32) {
            let width = text.chars().count() as f32 * style.size * 0.6;
            if style.wrap && max.is_finite() && width > max {
                (max, style.size * 1.3 * (width / max).ceil())
            } else {
                (width, style.size)
            }
        }
    }

    const SIZE: (f32, f32) = (820.0, 640.0);

    fn chrome(update: &UpdateStatus) -> Chrome<'_> {
        Chrome {
            tr: i18n::tr(Language::Pt),
            tab: 0,
            game_focused: false,
            fullscreen_warning: false,
            update,
            update_deferred: false,
            scanlines: true,
        }
    }

    fn build_with(ui: &mut Ui, chrome: &Chrome, now: u64) -> Rect {
        let mut body = Rect::ZERO;
        ui.begin(now);
        build(ui, &mut Fixed, SIZE, chrome, |_, _, rect| body = rect);
        ui.end();
        body
    }

    fn texts(ui: &Ui) -> Vec<String> {
        ui.frame()
            .nodes
            .iter()
            .filter_map(|node| match &node.visual {
                Visual::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_tab_gets_what_the_topbar_and_the_footer_leave() {
        let idle = UpdateStatus::Idle;
        let mut ui = Ui::new();
        let body = build_with(&mut ui, &chrome(&idle), 0);
        assert_eq!(body.y, TAB_BAR_HEIGHT);
        assert_eq!(body.bottom(), SIZE.1 - FOOTER_HEIGHT);
        assert_eq!(body.w, SIZE.0);
    }

    #[test]
    fn the_fullscreen_banner_only_shows_with_the_game_in_front() {
        let idle = UpdateStatus::Idle;
        let mut ui = Ui::new();

        let warned = Chrome {
            fullscreen_warning: true,
            ..chrome(&idle)
        };
        let body = build_with(&mut ui, &warned, 0);
        assert_eq!(
            body.y, TAB_BAR_HEIGHT,
            "fora do jogo o aviso não ocupa espaço"
        );

        let in_game = Chrome {
            game_focused: true,
            ..warned
        };
        let body = build_with(&mut ui, &in_game, 0);
        assert_eq!(body.y, TAB_BAR_HEIGHT + WARNING_HEIGHT);
        assert!(texts(&ui).iter().any(|text| text == "! AVISO"));
    }

    #[test]
    fn the_footer_says_the_version_and_the_game_presence() {
        let idle = UpdateStatus::Idle;
        let mut ui = Ui::new();
        build_with(&mut ui, &chrome(&idle), 0);
        let texts = texts(&ui);
        // O rodapé vai em caixa alta, e a tag de um beta junto: `V2.1.0-BETA.1`.
        let version = format!("VERSÃO V{}", env!("CARGO_PKG_VERSION")).to_uppercase();
        assert!(texts.contains(&version), "{texts:?}");
        assert!(texts.iter().any(|text| text == "JOGO AUSENTE"));
        assert!(texts.iter().any(|text| text == "ATUALIZADO"));
        assert!(!ui.animating(), "rodapé em repouso não pede quadros");
    }

    #[test]
    fn a_running_check_blinks_the_caret_instead_of_spinning() {
        let checking = UpdateStatus::Checking;
        let mut ui = Ui::new();
        build_with(&mut ui, &chrome(&checking), 0);
        assert!(texts(&ui).iter().any(|text| text == "VERIFICANDO"));
        assert_eq!(
            ui.frame_delay(),
            Some(500),
            "o caret só acorda a janela quando troca de fase"
        );
    }

    #[test]
    fn the_update_button_appears_only_when_there_is_something_to_do() {
        let mut ui = Ui::new();
        for (status, button) in [
            (UpdateStatus::Idle, false),
            (UpdateStatus::Checking, false),
            (
                UpdateStatus::Available {
                    version: "2.1.0".into(),
                },
                true,
            ),
            (
                UpdateStatus::Error {
                    message: "rede".into(),
                },
                true,
            ),
        ] {
            build_with(&mut ui, &chrome(&status), 0);
            assert_eq!(ui.frame().has_hit(update_action_id()), button, "{status:?}");
        }
    }

    #[test]
    fn a_ready_update_opens_the_dialog_until_it_is_deferred() {
        let ready = UpdateStatus::Ready {
            version: "2.1.0".into(),
        };
        let mut ui = Ui::new();
        build_with(&mut ui, &chrome(&ready), 0);
        assert!(ui.frame().has_hit(modal::primary_id()));
        assert!(
            ui.frame().has_hit(update_action_id()),
            "e o rodapé também instala"
        );
        assert_eq!(ui.frame().hit_at(4.0, 4.0), Some(modal::scrim_id()));

        let deferred = Chrome {
            update_deferred: true,
            ..chrome(&ready)
        };
        build_with(&mut ui, &deferred, 0);
        assert!(!ui.frame().has_hit(modal::primary_id()));
        assert!(ui.frame().has_hit(update_action_id()));
    }

    #[test]
    fn the_scanlines_cover_everything_and_stay_out_of_the_way() {
        let idle = UpdateStatus::Idle;
        let mut ui = Ui::new();
        build_with(&mut ui, &chrome(&idle), 0);
        let last = ui.frame().nodes.last().unwrap();
        assert!(matches!(last.visual, Visual::Scanlines { .. }));
        assert_eq!(last.rect, Rect::new(0.0, 0.0, SIZE.0, SIZE.1));
        // A textura não tem área clicável: a aba de trás continua respondendo.
        let toggle = widgets::theme_toggle_id();
        assert!(ui.frame().has_hit(toggle));

        let off = Chrome {
            scanlines: false,
            ..chrome(&idle)
        };
        build_with(&mut ui, &off, 0);
        assert!(!ui
            .frame()
            .nodes
            .iter()
            .any(|node| matches!(node.visual, Visual::Scanlines { .. })));
    }
}
