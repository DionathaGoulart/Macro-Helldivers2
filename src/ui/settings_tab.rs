//! Aba de configurações: atalhos dos slots, modificadores do jogo, idioma e os
//! três estratagemas de apoio fixo.
//!
//! Como a aba de macros, a tela é uma função do estado: entram os settings, saem
//! nós e áreas clicáveis. Um clique (ou uma tecla capturada) devolve uma
//! [`Action`] para a janela executar — gravar, refazer a tabela de atalhos,
//! avisar o overlay. A transformação em si mora em [`Change::apply`], que é
//! lógica pura e roda nos testes do host.
//!
//! Comportamento portado de `legacy/src/renderer/App.jsx` (~558–898), menos o
//! "modificador de sprint" (removido: o hook dispara com qualquer modificador
//! seguro) e a seção do updater, que chega na Fase 10.

use crate::data::SUPPORT_STRATS;
use crate::i18n::{self, Tr};
use crate::keys::{self, Vk};
use crate::settings::{Language, Settings, Speed, SLOT_COUNT, SUPPORT_COUNT};
use crate::ui::theme::{self, font};
use crate::ui::toolkit::{columns, grid_cell, id, id_at, Id, Measure, Rect, TextStyle, Ui, Weight};
use crate::ui::widgets::{self, CardHeader};

/// `px-6` da coluna de conteúdo.
const PAGE_PADDING: f32 = 24.0;
/// Espaço reservado à direita para a barra de rolagem.
const SCROLL_GUTTER: f32 = 12.0;
/// `gap-6` entre as duas colunas da primeira linha.
const CARD_GAP: f32 = 12.0;
/// `space-y-8` entre os blocos.
const SECTION_GAP: f32 = 20.0;

/// Altura de um rótulo de campo (`text-[10px]` com folga).
const LABEL_H: f32 = 14.0;
const LABEL_GAP: f32 = 8.0;
/// `py-3` dos botões de escolha.
const CHOICE_H: f32 = 36.0;
const CHOICE_GAP: f32 = 8.0;
/// `py-3.5` dos botões de atalho.
const KEY_BUTTON_H: f32 = 40.0;
/// `p-5` da caixa de cada atalho.
const KEY_BOX_PADDING: f32 = 16.0;
/// `gap-4` entre as caixas de atalho.
const KEY_BOX_GAP: f32 = 12.0;
/// `p-4` das linhas de toggle.
const TOGGLE_H: f32 = 52.0;
const TOGGLE_GAP: f32 = 12.0;
/// Espaço entre blocos dentro do card de controles (`space-y-6`).
const ROW_GAP: f32 = 20.0;
/// `gap-5` entre as três colunas de apoio, e entre o card e o seu botão.
const SUPPORT_GAP: f32 = 16.0;
/// Onde a próxima tecla capturada vai parar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    Slot(usize),
    Support(usize),
}

/// Uma preferência mudou. A janela aplica, grava e espalha os efeitos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Shortcut { index: usize, key: String },
    SupportShortcut { index: usize, key: String },
    Modifier(String),
    Speed(Speed),
    UseArrows(bool),
    EnableOverlay(bool),
    AlwaysShowSlots(bool),
    Language(Language),
}

impl Change {
    /// Aplica a mudança num snapshot. Vive aqui, e não na janela, para o teste
    /// cobrir a mesma transformação que roda em produção.
    pub fn apply(&self, settings: &mut Settings) {
        match self {
            Change::Shortcut { index, key } => {
                if let Some(slot) = settings.shortcuts.get_mut(*index) {
                    *slot = Some(key.clone());
                }
            }
            Change::SupportShortcut { index, key } => {
                if let Some(slot) = settings.support_shortcuts.get_mut(*index) {
                    *slot = Some(key.clone());
                }
            }
            Change::Modifier(key) => settings.modifier_key = key.clone(),
            Change::Speed(speed) => settings.macro_speed = *speed,
            Change::UseArrows(on) => settings.use_arrows = *on,
            Change::EnableOverlay(on) => settings.enable_overlay = *on,
            Change::AlwaysShowSlots(on) => settings.always_show_slots = *on,
            Change::Language(language) => settings.language = *language,
        }
    }
}

/// O que a janela faz depois de um clique (ou de uma tecla) na aba.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Só reconstruir a tela.
    Redraw,
    /// Gravar o settings novo, refazer a tabela de atalhos e avisar o overlay.
    Setting(Change),
}

/// O que a aba precisa saber do resto do app.
pub struct Ctx<'a> {
    pub settings: &'a Settings,
}

impl Ctx<'_> {
    fn tr(&self) -> &'static Tr {
        i18n::tr(self.settings.language)
    }
}

/// Estado da aba entre passagens de construção.
#[derive(Debug, Default)]
pub struct SettingsTab {
    capturing: Option<Capture>,
}

// --- Ids ---

fn scroll_id() -> Id {
    id("settings.scroll")
}

fn shortcut_id(index: usize) -> Id {
    id_at("settings.shortcut", index)
}

fn support_shortcut_id(index: usize) -> Id {
    id_at("settings.support", index)
}

fn modifier_id(index: usize) -> Id {
    id_at("settings.modifier", index)
}

fn speed_id(index: usize) -> Id {
    id_at("settings.speed", index)
}

fn language_id(index: usize) -> Id {
    id_at("settings.language", index)
}

fn arrows_id() -> Id {
    id("settings.arrows")
}

fn overlay_id() -> Id {
    id("settings.overlay")
}

fn hud_id() -> Id {
    id("settings.hud")
}

// --- Estilos ---

fn label_style() -> TextStyle {
    TextStyle::new(font::SIZE_LABEL, Weight::Black).tracking(font::TRACKING_LABEL)
}

fn hint_style() -> TextStyle {
    TextStyle::new(font::SIZE_TINY, Weight::Regular).wrap()
}

impl SettingsTab {
    pub fn new() -> SettingsTab {
        SettingsTab::default()
    }

    /// Atalho sendo capturado agora, se algum. A janela usa isto para ligar o
    /// modo de gravação (`recording`), que desarma o hook de teclado.
    pub fn capturing(&self) -> Option<Capture> {
        self.capturing
    }

    /// Desiste da captura em curso — a janela chama ao sair da aba.
    pub fn cancel_capture(&mut self) {
        self.capturing = None;
    }

    // --- Construção ---

    pub fn build(&mut self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        let view = area.inset_xy(PAGE_PADDING, 16.0);
        let width = view.w - SCROLL_GUTTER;
        let half = (width - CARD_GAP) / 2.0;

        let offset = ui.scroll_begin(scroll_id(), view);
        let mut y = view.y - offset;

        // As duas primeiras colunas terminam na mesma linha, como o grid da v1.
        let row = shortcuts_height().max(controls_height(measure, half, ctx));
        let top = columns(Rect::new(view.x, y, width, row), 2, CARD_GAP);
        self.shortcuts_card(ui, top[0], ctx);
        self.controls_card(ui, measure, top[1], ctx);
        y += row + SECTION_GAP;

        let height = language_height();
        self.language_card(ui, Rect::new(view.x, y, half, height), ctx);
        y += height + SECTION_GAP;

        let height = support_height(width);
        self.support_card(ui, Rect::new(view.x, y, width, height), ctx);
        y += height;

        ui.scroll_end(scroll_id(), view, y - (view.y - offset));
    }

    /// Card "Atalhos de Combate": os quatro atalhos de slot, dois por linha.
    fn shortcuts_card(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: ctx.tr().settings.keybinding,
                accent: theme::YELLOW,
            }),
        );

        for index in 0..SLOT_COUNT {
            let row = index / 2;
            let col = index % 2;
            let width = (content.w - KEY_BOX_GAP) / 2.0;
            let box_rect = Rect::new(
                content.x + (width + KEY_BOX_GAP) * col as f32,
                content.y + (key_box_height() + KEY_BOX_GAP) * row as f32,
                width,
                key_box_height(),
            );
            self.key_box(
                ui,
                box_rect,
                &format!("{} {}", ctx.tr().settings.shortcut_label, index + 1),
                shortcut_id(index),
                ctx.settings.shortcut(index),
                Capture::Slot(index),
                ctx,
            );
        }
    }

    /// Caixa "rótulo + botão de atalho".
    #[allow(clippy::too_many_arguments)]
    fn key_box(
        &self,
        ui: &mut Ui,
        rect: Rect,
        label: &str,
        id: Id,
        bound: Option<&str>,
        capture: Capture,
        ctx: &Ctx,
    ) {
        ui.fill(rect, theme::RADIUS_CARD, theme::SURFACE);
        ui.stroke(
            rect,
            theme::RADIUS_CARD,
            theme::HAIRLINE_WIDTH,
            theme::BORDER,
        );

        let mut content = rect.inset(KEY_BOX_PADDING);
        let label_row = content.cut_top(LABEL_H);
        ui.text(
            label_row,
            label.to_uppercase(),
            label_style(),
            theme::TEXT_DIM,
        );
        content.skip_top(LABEL_GAP);
        self.key_button(ui, id, content.with_h(KEY_BUTTON_H), bound, capture, ctx);
    }

    fn key_button(
        &self,
        ui: &mut Ui,
        id: Id,
        rect: Rect,
        bound: Option<&str>,
        capture: Capture,
        ctx: &Ctx,
    ) {
        let capturing = self.capturing == Some(capture);
        let label = if capturing {
            ctx.tr().macros.listening
        } else {
            bound.unwrap_or(ctx.tr().macros.bind)
        };
        widgets::key_button(ui, id, rect, label, capturing);
    }

    /// Card "Config. de Controles": modificador, velocidade e os três toggles.
    fn controls_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let mut content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: tr.settings.controller,
                accent: theme::YELLOW,
            }),
        );

        // Tecla de estratagema do jogo: quatro botões em duas linhas.
        ui.text(
            content.cut_top(LABEL_H),
            tr.settings.ingame_key.to_uppercase(),
            label_style(),
            theme::TEXT_DIM,
        );
        content.skip_top(LABEL_GAP);
        let grid = content.cut_top(CHOICE_H * 2.0 + CHOICE_GAP);
        for (index, key) in keys::MODIFIER_KEYS.iter().enumerate() {
            let cell = grid_cell(grid, 2, CHOICE_H, CHOICE_GAP, index);
            widgets::choice_button(
                ui,
                modifier_id(index),
                cell,
                keys::modifier_label(key),
                ctx.settings.modifier_key == *key,
            );
        }
        content.skip_top(ROW_GAP);

        // Velocidade: rótulo, explicação e os três perfis lado a lado.
        ui.text(
            content.cut_top(LABEL_H),
            tr.settings.macro_speed.to_uppercase(),
            label_style(),
            theme::TEXT_DIM,
        );
        content.skip_top(LABEL_GAP);
        let desc_h = measure
            .text_size(tr.settings.macro_speed_desc, hint_style(), content.w)
            .1;
        ui.text(
            content.cut_top(desc_h),
            tr.settings.macro_speed_desc,
            hint_style(),
            theme::TEXT_DIM,
        );
        content.skip_top(LABEL_GAP);
        let row = content.cut_top(CHOICE_H);
        for (index, speed) in Speed::ALL.iter().enumerate() {
            let cell = grid_cell(row, 3, CHOICE_H, CHOICE_GAP, index);
            widgets::choice_button(
                ui,
                speed_id(index),
                cell,
                tr.settings.speed(*speed),
                ctx.settings.macro_speed == *speed,
            );
        }
        content.skip_top(ROW_GAP);

        let toggles = [
            (
                arrows_id(),
                tr.settings.arrow_mode,
                if ctx.settings.use_arrows {
                    tr.settings.arrow_active
                } else {
                    tr.settings.wasd_active
                },
                ctx.settings.use_arrows,
            ),
            (
                overlay_id(),
                tr.settings.overlay_shortcut,
                if ctx.settings.enable_overlay {
                    tr.settings.overlay_enabled
                } else {
                    tr.settings.overlay_disabled
                },
                ctx.settings.enable_overlay,
            ),
            (
                hud_id(),
                tr.settings.persistent_hud,
                if ctx.settings.always_show_slots {
                    tr.settings.persistent_hud_on
                } else {
                    tr.settings.persistent_hud_off
                },
                ctx.settings.always_show_slots,
            ),
        ];
        for (id, label, desc, on) in toggles {
            let row = content.cut_top(TOGGLE_H);
            widgets::toggle_row(ui, id, row, label, desc, on);
            content.skip_top(TOGGLE_GAP);
        }
    }

    fn language_card(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: ctx.tr().settings.language,
                accent: theme::YELLOW,
            }),
        );
        let row = content.with_h(CHOICE_H);
        for (index, language) in Language::ALL.iter().enumerate() {
            let cell = grid_cell(row, 2, CHOICE_H, CHOICE_GAP, index);
            widgets::choice_button(
                ui,
                language_id(index),
                cell,
                i18n::language_name(*language),
                ctx.settings.language == *language,
            );
        }
    }

    /// Card "Estratagemas de Apoio Fixo": três cards quadrados, cada um com o
    /// seu botão de atalho.
    fn support_card(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader {
                title: ctx.tr().settings.support,
                accent: theme::YELLOW,
            }),
        );
        let cols = columns(content, SUPPORT_COUNT, SUPPORT_GAP);

        for (index, support) in SUPPORT_STRATS.iter().enumerate() {
            let mut col = cols[index];
            // O card é quadrado (`aspect-square` da v1), então a coluna manda
            // na altura dele.
            let card = col.cut_top(col.w);
            widgets::support_card(ui, index, card, support);
            col.skip_top(SUPPORT_GAP);
            self.key_button(
                ui,
                support_shortcut_id(index),
                col.with_h(KEY_BUTTON_H),
                ctx.settings.support_shortcut(index),
                Capture::Support(index),
                ctx,
            );
        }
    }

    // --- Cliques e teclas ---

    /// Trata um clique da aba. `None` quando o id não é daqui.
    pub fn on_click(&mut self, clicked: Id, ctx: &Ctx) -> Option<Action> {
        for index in 0..SLOT_COUNT {
            if clicked == shortcut_id(index) {
                self.capturing = Some(Capture::Slot(index));
                return Some(Action::Redraw);
            }
        }
        for index in 0..SUPPORT_COUNT {
            if clicked == support_shortcut_id(index) {
                self.capturing = Some(Capture::Support(index));
                return Some(Action::Redraw);
            }
        }
        for (index, key) in keys::MODIFIER_KEYS.iter().enumerate() {
            if clicked == modifier_id(index) {
                return Some(Action::Setting(Change::Modifier(key.to_string())));
            }
        }
        for (index, speed) in Speed::ALL.iter().enumerate() {
            if clicked == speed_id(index) {
                return Some(Action::Setting(Change::Speed(*speed)));
            }
        }
        for (index, language) in Language::ALL.iter().enumerate() {
            if clicked == language_id(index) {
                return Some(Action::Setting(Change::Language(*language)));
            }
        }

        let settings = ctx.settings;
        if clicked == arrows_id() {
            return Some(Action::Setting(Change::UseArrows(!settings.use_arrows)));
        }
        if clicked == overlay_id() {
            return Some(Action::Setting(Change::EnableOverlay(
                !settings.enable_overlay,
            )));
        }
        if clicked == hud_id() {
            return Some(Action::Setting(Change::AlwaysShowSlots(
                !settings.always_show_slots,
            )));
        }
        None
    }

    /// Tecla recebida pela janela enquanto a aba espera um atalho. `None` quando
    /// não há captura em curso — aí a tecla segue seu caminho normal.
    ///
    /// Modificadores puros e teclas fora da tabela canônica são engolidos sem
    /// ligar nada: a v1 ignorava os primeiros, e uma tecla que `keys.rs` não
    /// sabe nomear viraria um atalho que o hook nunca reconheceria.
    pub fn on_key(&mut self, vk: Vk) -> Option<Action> {
        let capture = self.capturing?;
        if vk == keys::VK_ESCAPE {
            self.capturing = None;
            return Some(Action::Redraw);
        }
        if keys::is_modifier_vk(vk) {
            return Some(Action::Redraw);
        }
        let Some(name) = keys::name_from_vk(vk) else {
            return Some(Action::Redraw);
        };

        self.capturing = None;
        Some(Action::Setting(match capture {
            Capture::Slot(index) => Change::Shortcut {
                index,
                key: name.to_string(),
            },
            Capture::Support(index) => Change::SupportShortcut {
                index,
                key: name.to_string(),
            },
        }))
    }
}

// --- Alturas (a rolagem precisa delas antes de desenhar) ---

fn key_box_height() -> f32 {
    KEY_BOX_PADDING * 2.0 + LABEL_H + LABEL_GAP + KEY_BUTTON_H
}

fn shortcuts_height() -> f32 {
    widgets::card_chrome(true) + key_box_height() * 2.0 + KEY_BOX_GAP
}

fn controls_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let desc_h = measure
        .text_size(ctx.tr().settings.macro_speed_desc, hint_style(), inner)
        .1;
    let modifiers = LABEL_H + LABEL_GAP + CHOICE_H * 2.0 + CHOICE_GAP;
    let speed = LABEL_H + LABEL_GAP + desc_h + LABEL_GAP + CHOICE_H;
    let toggles = (TOGGLE_H + TOGGLE_GAP) * 3.0;
    widgets::card_chrome(true) + modifiers + ROW_GAP + speed + ROW_GAP + toggles
}

fn language_height() -> f32 {
    widgets::card_chrome(true) + CHOICE_H
}

fn support_height(width: f32) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let column = (inner - SUPPORT_GAP * (SUPPORT_COUNT - 1) as f32) / SUPPORT_COUNT as f32;
    widgets::card_chrome(true) + column + SUPPORT_GAP + KEY_BUTTON_H
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toolkit::{Input, Visual};

    /// Medidor de largura fixa: o layout só precisa de uma medida plausível.
    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, max: f32) -> (f32, f32) {
            let width = text.chars().count() as f32 * style.size * 0.6;
            if style.wrap && max.is_finite() && width > max {
                let lines = (width / max).ceil();
                (max, style.size * 1.3 * lines)
            } else {
                (width, style.size)
            }
        }
    }

    const AREA: Rect = Rect::new(0.0, 0.0, 820.0, 532.0);

    fn ctx(settings: &Settings) -> Ctx<'_> {
        Ctx { settings }
    }

    fn build_at(tab: &mut SettingsTab, ui: &mut Ui, settings: &Settings, now: u64) {
        ui.begin(now);
        tab.build(ui, &mut Fixed, AREA, &ctx(settings));
        ui.end();
    }

    fn build(tab: &mut SettingsTab, ui: &mut Ui, settings: &Settings) {
        build_at(tab, ui, settings, 0);
    }

    /// Duas passagens com folga entre elas: a primeira acende os switches que
    /// já nascem ligados, e a segunda os encontra parados na ponta.
    fn build_settled(tab: &mut SettingsTab, ui: &mut Ui, settings: &Settings) {
        build_at(tab, ui, settings, 0);
        build_at(tab, ui, settings, 2_000);
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

    fn vk(name: &str) -> Vk {
        keys::vk_from_name(name).expect(name)
    }

    #[test]
    fn every_control_of_the_tab_is_reachable_by_the_mouse() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &settings);

        let mut expected: Vec<Id> = Vec::new();
        expected.extend((0..SLOT_COUNT).map(shortcut_id));
        expected.extend((0..SUPPORT_COUNT).map(support_shortcut_id));
        expected.extend((0..keys::MODIFIER_KEYS.len()).map(modifier_id));
        expected.extend((0..Speed::ALL.len()).map(speed_id));
        expected.extend((0..Language::ALL.len()).map(language_id));
        expected.extend([arrows_id(), overlay_id(), hud_id()]);

        for id in expected {
            assert!(ui.frame().has_hit(id), "widget sem área clicável: {id}");
        }
    }

    #[test]
    fn clicking_a_shortcut_starts_listening_and_a_key_binds_it() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();

        assert_eq!(
            tab.on_click(shortcut_id(2), &ctx(&settings)),
            Some(Action::Redraw)
        );
        assert_eq!(tab.capturing(), Some(Capture::Slot(2)));

        assert_eq!(
            tab.on_key(vk("Numpad5")),
            Some(Action::Setting(Change::Shortcut {
                index: 2,
                key: "Numpad5".to_string()
            }))
        );
        assert_eq!(tab.capturing(), None, "a captura termina na primeira tecla");
    }

    #[test]
    fn escape_cancels_and_pure_modifiers_are_ignored() {
        let mut tab = SettingsTab::new();
        let settings = Settings::default();

        tab.on_click(shortcut_id(0), &ctx(&settings));
        for name in ["LeftControl", "LeftShift", "LeftAlt"] {
            assert_eq!(tab.on_key(vk(name)), Some(Action::Redraw));
            assert_eq!(
                tab.capturing(),
                Some(Capture::Slot(0)),
                "{name} não liga atalho nem encerra a captura"
            );
        }

        assert_eq!(tab.on_key(keys::VK_ESCAPE), Some(Action::Redraw));
        assert_eq!(tab.capturing(), None);
        // Fora da captura a tecla não é nossa.
        assert_eq!(tab.on_key(vk("F1")), None);
    }

    #[test]
    fn a_key_outside_the_canonical_table_never_becomes_a_shortcut() {
        let mut tab = SettingsTab::new();
        let settings = Settings::default();
        tab.on_click(shortcut_id(1), &ctx(&settings));

        // 0xC0 é a crase/til do teclado: existe no Windows, não em `keys.rs`.
        assert_eq!(tab.on_key(0xC0), Some(Action::Redraw));
        assert_eq!(tab.capturing(), Some(Capture::Slot(1)));
    }

    #[test]
    fn support_shortcuts_capture_into_their_own_list() {
        let mut tab = SettingsTab::new();
        let settings = Settings::default();

        tab.on_click(support_shortcut_id(1), &ctx(&settings));
        assert_eq!(tab.capturing(), Some(Capture::Support(1)));

        let action = tab.on_key(vk("F6"));
        assert_eq!(
            action,
            Some(Action::Setting(Change::SupportShortcut {
                index: 1,
                key: "F6".to_string()
            }))
        );

        let mut applied = settings.clone();
        let Some(Action::Setting(change)) = action else {
            panic!("mudança esperada");
        };
        change.apply(&mut applied);
        assert_eq!(applied.support_shortcut(1), Some("F6"));
        assert_eq!(applied.shortcuts, settings.shortcuts, "slots intocados");
    }

    #[test]
    fn an_unbound_support_offers_the_bind_label() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();
        build_settled(&mut tab, &mut ui, &settings);

        let bind = i18n::tr(settings.language).macros.bind;
        assert_eq!(
            texts(&ui).iter().filter(|text| *text == bind).count(),
            SUPPORT_COUNT,
            "os três apoios começam sem atalho"
        );
    }

    #[test]
    fn the_content_scrolls_because_it_is_taller_than_the_window() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &settings);

        // A roda só mexe num container cujo conteúdo transborda.
        assert!(
            ui.input(Input::Wheel {
                x: 400.0,
                y: 300.0,
                delta: -1.0,
            })
            .redraw,
            "a aba inteira não cabe em 532 DIP e precisa rolar"
        );
    }

    #[test]
    fn choice_buttons_report_the_value_they_carry() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();

        assert_eq!(
            tab.on_click(modifier_id(1), &ctx(&settings)),
            Some(Action::Setting(Change::Modifier("LeftAlt".to_string())))
        );
        assert_eq!(
            tab.on_click(speed_id(2), &ctx(&settings)),
            Some(Action::Setting(Change::Speed(Speed::Turbo)))
        );
        assert_eq!(
            tab.on_click(language_id(1), &ctx(&settings)),
            Some(Action::Setting(Change::Language(Language::En)))
        );
    }

    #[test]
    fn toggles_send_the_opposite_of_the_current_value() {
        let settings = Settings {
            use_arrows: false,
            enable_overlay: true,
            always_show_slots: false,
            ..Settings::default()
        };
        let mut tab = SettingsTab::new();

        assert_eq!(
            tab.on_click(arrows_id(), &ctx(&settings)),
            Some(Action::Setting(Change::UseArrows(true)))
        );
        assert_eq!(
            tab.on_click(overlay_id(), &ctx(&settings)),
            Some(Action::Setting(Change::EnableOverlay(false)))
        );
        assert_eq!(
            tab.on_click(hud_id(), &ctx(&settings)),
            Some(Action::Setting(Change::AlwaysShowSlots(true)))
        );
    }

    #[test]
    fn every_change_lands_on_the_field_it_names() {
        let mut settings = Settings::default();
        let changes = [
            Change::Shortcut {
                index: 0,
                key: "F9".into(),
            },
            Change::SupportShortcut {
                index: 2,
                key: "Home".into(),
            },
            Change::Modifier("Minus".into()),
            Change::Speed(Speed::Fast),
            Change::UseArrows(true),
            Change::EnableOverlay(false),
            Change::AlwaysShowSlots(true),
            Change::Language(Language::En),
        ];
        for change in &changes {
            change.apply(&mut settings);
        }

        assert_eq!(settings.shortcut(0), Some("F9"));
        assert_eq!(settings.support_shortcut(2), Some("Home"));
        assert_eq!(settings.modifier_key, "Minus");
        assert_eq!(settings.macro_speed, Speed::Fast);
        assert!(settings.use_arrows);
        assert!(!settings.enable_overlay);
        assert!(settings.always_show_slots);
        assert_eq!(settings.language, Language::En);

        // Índice fora da faixa não pode entrar em pânico nem inventar slot.
        Change::Shortcut {
            index: 9,
            key: "F1".into(),
        }
        .apply(&mut settings);
        assert_eq!(settings.shortcuts.len(), SLOT_COUNT);
    }

    #[test]
    fn the_listening_button_pulses_and_the_bound_key_does_not() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();

        build_settled(&mut tab, &mut ui, &settings);
        assert!(!ui.animating(), "sem captura, a aba fica parada");
        assert!(texts(&ui).iter().any(|text| text == "F1"));

        tab.on_click(shortcut_id(0), &ctx(&settings));
        build_settled(&mut tab, &mut ui, &settings);
        assert!(ui.animating(), "o pulso mantém o timer de animação vivo");
        assert!(texts(&ui)
            .iter()
            .any(|text| text.contains("ESCUTANDO") || text.contains("OUVINDO")));
    }

    #[test]
    fn the_whole_tab_follows_the_language() {
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();

        let pt = Settings::default();
        build(&mut tab, &mut ui, &pt);
        let before = texts(&ui);
        assert!(before
            .iter()
            .any(|text| text.contains("ATALHOS DE COMBATE")));

        let en = Settings {
            language: Language::En,
            ..Settings::default()
        };
        build(&mut tab, &mut ui, &en);
        let after = texts(&ui);
        assert!(after.iter().any(|text| text.contains("COMBAT SHORTCUTS")));
        assert!(!after.iter().any(|text| text.contains("ATALHOS DE COMBATE")));
    }

    #[test]
    fn an_unknown_click_is_not_ours() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        assert_eq!(tab.on_click(id("outra.tela"), &ctx(&settings)), None);
    }
}
