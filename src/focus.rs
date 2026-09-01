//! Quem está em foco, e o que muda quando isso muda.
//!
//! A v1 descobria isso por polling (`legacy/src/main/index.js` ~203-267): um
//! timer de 400ms perguntando ao SO qual era a janela ativa, com um modo ocioso
//! de 2s para não torrar CPU. Aqui o sinal vem de graça pelo
//! `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, e este módulo fica com a parte
//! que não depende do Windows: classificar um título e decidir o que a troca
//! dispara. Assim a máquina de estados inteira é testável no host.
//!
//! A classificação é por título porque é o que a v1 usava e o que funciona sem
//! encostar no processo do jogo.

use crate::shared::OverlayState;

/// Título da janela principal. Casa com a regra "MACRO e HELLDIVERS".
pub const APP_WINDOW_TITLE: &str = "Macro Helldivers 2";

/// Título das duas janelas do overlay (strip e painel).
pub const OVERLAY_WINDOW_TITLE: &str = "HD2_OVERLAY";

/// Que janela está em primeiro plano, do ponto de vista do app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Game,
    /// Nossa janela principal.
    App,
    /// Strip ou painel do overlay.
    Overlay,
    /// Qualquer outra coisa: navegador, Discord, área de trabalho.
    Other,
}

impl Window {
    /// Com o jogo, o app ou o overlay em foco os atalhos valem; fora disso o
    /// teclado é de quem estiver na frente.
    pub fn arms_macros(self) -> bool {
        !matches!(self, Window::Other)
    }
}

/// Classifica pelo título da janela em foreground (R10).
pub fn classify(title: &str) -> Window {
    if contains_ignoring_case(title, "HD2_OVERLAY") {
        Window::Overlay
    } else if contains_ignoring_case(title, "MACRO") && contains_ignoring_case(title, "HELLDIVERS")
    {
        Window::App
    } else if contains_ignoring_case(title, "HELLDIVERS") {
        Window::Game
    } else {
        Window::Other
    }
}

/// `needle` precisa ser ASCII. Comparar sem construir uma cópia maiúscula do
/// título evita alocar a cada troca de janela do sistema.
fn contains_ignoring_case(haystack: &str, needle: &str) -> bool {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    if needle.is_empty() {
        return true;
    }
    haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}

/// Estado do app que muda o efeito de uma troca de foco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub enable_overlay: bool,
    pub always_show_slots: bool,
    /// Overlay com conteúdo na tela agora (`minimal` ou `panel`).
    pub overlay_visible: bool,
}

/// O que a troca de foco manda fazer. Vazio quando nada mudou.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Effects {
    /// Estado atual, mudando ou não.
    pub focused: bool,
    /// `true` só na transição.
    pub focus_changed: bool,
    pub overlay: Option<OverlayState>,
    /// Hora de reavaliar o modo de vídeo do jogo e avisar as janelas.
    pub check_fullscreen: bool,
    /// O check de atualização adiado (jogo em foco no boot) pode rodar agora.
    pub check_updates: bool,
    /// Reafirma o overlay no topo do z-order.
    pub reassert: bool,
}

/// Máquina de estados do foco. Vive na thread de hooks, que é a única a
/// observar trocas de janela.
#[derive(Debug, Default)]
pub struct Watcher {
    focused: bool,
    update_checked: bool,
}

impl Watcher {
    pub fn new() -> Watcher {
        Watcher::default()
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    /// Marca o check de atualização como feito, para ele não repetir na próxima
    /// perda de foco (a v1 chamava isso de `hasCheckedUpdates`).
    pub fn mark_update_checked(&mut self) {
        self.update_checked = true;
    }

    /// Registra a janela em foco e devolve o que ela provoca.
    pub fn observe(&mut self, window: Window, context: Context) -> Effects {
        let focused = window.arms_macros();
        let mut effects = Effects {
            focused,
            ..Effects::default()
        };

        if focused != self.focused {
            self.focused = focused;
            effects.focus_changed = true;

            if focused {
                // Os atalhos não são registrados nem removidos: quem decide é a
                // flag de foco lida pelo callback do hook. O que muda aqui é o
                // overlay e o aviso de tela cheia.
                if context.enable_overlay && context.always_show_slots {
                    effects.overlay = Some(OverlayState::Minimal);
                }
                effects.check_fullscreen = true;
            } else {
                if context.enable_overlay {
                    effects.overlay = Some(OverlayState::Hidden);
                }
                if !self.update_checked {
                    self.update_checked = true;
                    effects.check_updates = true;
                }
            }
        }

        // O jogo re-agarra o topo do z-order em alt-tab e em troca de modo de
        // vídeo; enquanto ele estiver na frente com overlay visível, reafirmamos.
        let visible = match effects.overlay {
            Some(state) => state != OverlayState::Hidden,
            None => context.overlay_visible,
        };
        effects.reassert = focused && visible;
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUD: Context = Context {
        enable_overlay: true,
        always_show_slots: true,
        overlay_visible: false,
    };

    const PLAIN: Context = Context {
        always_show_slots: false,
        ..HUD
    };

    #[test]
    fn titles_are_classified_like_the_v1_rules() {
        assert_eq!(classify("HELLDIVERS™ 2"), Window::Game);
        assert_eq!(classify(APP_WINDOW_TITLE), Window::App);
        assert_eq!(classify(OVERLAY_WINDOW_TITLE), Window::Overlay);
        assert_eq!(classify("Discord"), Window::Other);
        assert_eq!(classify(""), Window::Other);
    }

    #[test]
    fn classification_ignores_case_and_surrounding_text() {
        assert_eq!(classify("helldivers 2"), Window::Game);
        assert_eq!(classify("hd2_overlay panel"), Window::Overlay);
        // Um documento aberto no bloco de notas com o nome do jogo conta como
        // jogo — é assim que o teste manual do hook funciona.
        assert_eq!(
            classify("helldivers-notas.txt - Bloco de Notas"),
            Window::Game
        );
        // E a janela do app nunca é confundida com o jogo.
        assert_eq!(classify("MACRO HELLDIVERS 2"), Window::App);
    }

    #[test]
    fn only_foreign_windows_disarm_the_macros() {
        assert!(Window::Game.arms_macros());
        assert!(Window::App.arms_macros());
        assert!(Window::Overlay.arms_macros());
        assert!(!Window::Other.arms_macros());
    }

    #[test]
    fn gaining_focus_shows_the_persistent_hud_and_checks_the_video_mode() {
        let mut watcher = Watcher::new();
        let effects = watcher.observe(Window::Game, HUD);

        assert!(effects.focus_changed && effects.focused);
        assert_eq!(effects.overlay, Some(OverlayState::Minimal));
        assert!(effects.check_fullscreen);
        assert!(
            effects.reassert,
            "o strip que acabou de aparecer sobe junto"
        );
        assert!(!effects.check_updates);
        assert!(watcher.focused());
    }

    #[test]
    fn gaining_focus_without_the_persistent_hud_leaves_the_overlay_alone() {
        let mut watcher = Watcher::new();
        let effects = watcher.observe(Window::Game, PLAIN);

        assert_eq!(effects.overlay, None);
        assert!(!effects.reassert, "nada visível para reafirmar");
        assert!(effects.check_fullscreen);
    }

    #[test]
    fn the_overlay_stays_untouched_when_the_feature_is_off() {
        let context = Context {
            enable_overlay: false,
            ..HUD
        };
        let mut watcher = Watcher::new();
        assert_eq!(watcher.observe(Window::Game, context).overlay, None);
        assert_eq!(watcher.observe(Window::Other, context).overlay, None);
    }

    #[test]
    fn losing_focus_hides_the_overlay_and_releases_the_deferred_update_check() {
        let mut watcher = Watcher::new();
        watcher.observe(Window::Game, HUD);

        let effects = watcher.observe(Window::Other, HUD);
        assert!(effects.focus_changed && !effects.focused);
        assert_eq!(effects.overlay, Some(OverlayState::Hidden));
        assert!(effects.check_updates);
        assert!(!effects.reassert);

        // O check acontece uma vez só, por mais que o foco vá e volte.
        watcher.observe(Window::Game, HUD);
        assert!(!watcher.observe(Window::Other, HUD).check_updates);
    }

    #[test]
    fn an_already_checked_updater_never_asks_again() {
        let mut watcher = Watcher::new();
        watcher.mark_update_checked();
        watcher.observe(Window::Game, PLAIN);
        assert!(!watcher.observe(Window::Other, PLAIN).check_updates);
    }

    #[test]
    fn moving_between_our_own_windows_is_not_a_transition() {
        let mut watcher = Watcher::new();
        watcher.observe(Window::Game, PLAIN);

        for window in [Window::App, Window::Overlay, Window::Game] {
            let effects = watcher.observe(window, PLAIN);
            assert!(!effects.focus_changed, "{window:?}");
            assert!(effects.focused);
            assert_eq!(effects.overlay, None);
        }
    }

    #[test]
    fn the_periodic_tick_keeps_the_overlay_on_top_while_the_game_is_up() {
        let visible = Context {
            overlay_visible: true,
            ..PLAIN
        };
        let mut watcher = Watcher::new();
        watcher.observe(Window::Game, visible);

        // Sem transição, o tick só reafirma o z-order.
        let effects = watcher.observe(Window::Game, visible);
        assert!(!effects.focus_changed);
        assert!(effects.reassert);

        // E não reafirma nada com o jogo minimizado.
        assert!(!watcher.observe(Window::Other, visible).reassert);
    }
}
