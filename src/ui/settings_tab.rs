//! Aba de configurações: atalhos dos slots, modificadores do jogo, idioma, os
//! três estratagemas de apoio fixo e o backup.
//!
//! Como a aba de macros, a tela é uma função do estado: entram os settings, saem
//! nós e áreas clicáveis. Um clique (ou uma tecla capturada) devolve uma
//! [`Action`] para a janela executar: gravar, refazer a tabela de atalhos,
//! avisar o overlay, abrir um diálogo de arquivo. A transformação em si mora em
//! [`Change::apply`], que é lógica pura e roda nos testes do host.
//!
//! Comportamento portado de `legacy/src/renderer/App.jsx` (~558-898), menos o
//! "modificador de sprint" (removido: o hook dispara com qualquer modificador
//! seguro) e a seção do updater, que chega na Fase 10.

use crate::data::SUPPORT_STRATS;
use crate::diag::typing::TypingTest;
use crate::diag::{HookHealth, LastRun, Stats, WindowInfo};
use crate::engine;
use crate::i18n::{self, DebugText, Tr};
use crate::keys::{self, Vk};
use crate::settings::{Language, Settings, Speed, Theme, SLOT_COUNT, SUPPORT_COUNT};
use crate::ui::theme::font;
use crate::ui::theme::{self, motion};
use crate::ui::toolkit::Weight;
use crate::ui::toolkit::{
    columns, grid_cell, grid_height, id, id_at, Id, Measure, Rect, TextStyle, Ui,
};
use crate::ui::widgets::{self, styles, ButtonVariant, CardHeader};

/// `screen-pad` da coluna de conteúdo.
const PAGE_PADDING: f32 = 24.0;
const PAGE_TOP: f32 = 20.0;
/// Espaço reservado à direita para a barra de rolagem.
const SCROLL_GUTTER: f32 = 14.0;
/// Espaço entre as colunas e entre os painéis: cabe a sombra dura.
const CARD_GAP: f32 = 20.0;
const SECTION_GAP: f32 = 22.0;

/// Altura de um rótulo de campo.
const LABEL_H: f32 = 16.0;
const LABEL_GAP: f32 = 8.0;
/// Botões de escolha.
const CHOICE_H: f32 = 36.0;
const CHOICE_GAP: f32 = 8.0;
/// Os cinco perfis de velocidade em três colunas: "Baixo FPS" ainda cabe num
/// terço da coluna com a janela na largura mínima, e não num quarto.
const SPEED_COLUMNS: usize = 3;
/// Botões de atalho.
const KEY_BUTTON_H: f32 = 40.0;
/// Caixa aninhada de cada atalho.
const KEY_BOX_PADDING: f32 = 12.0;
const KEY_BOX_GAP: f32 = 12.0;
/// Linhas de toggle.
const TOGGLE_H: f32 = 58.0;
const TOGGLE_GAP: f32 = 10.0;
/// Espaço entre blocos dentro do painel de controles.
const ROW_GAP: f32 = 20.0;
/// Entre as três colunas de apoio, e entre o card e o seu botão.
const SUPPORT_GAP: f32 = 16.0;
/// Canto do toast do backup.
const TOAST_MARGIN: f32 = 20.0;
/// Linha "rótulo · valor" das estatísticas do modo debug.
const STAT_ROW_H: f32 = 20.0;
/// Fração da largura que o rótulo de uma estatística ocupa.
const STAT_LABEL_SHARE: f32 = 0.34;

/// Quanto tempo o aviso de backup fica na tela (2,5s, como na v1).
pub const BACKUP_STATUS_MS: u32 = 2_500;
/// Onde a próxima tecla capturada vai parar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    Slot(usize),
    Support(usize),
}

/// Resultado da última operação de backup ou de relatório.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupStatus {
    Exported,
    Imported,
    Failed,
    ReportExported,
    ReportFailed,
}

/// Uma preferência mudou. A janela aplica, grava e espalha os efeitos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Shortcut {
        index: usize,
        key: String,
    },
    SupportShortcut {
        index: usize,
        key: String,
    },
    Modifier(String),
    Speed(Speed),
    UseArrows(bool),
    EnableOverlay(bool),
    AlwaysShowSlots(bool),
    Language(Language),
    /// `None` volta a seguir o modo claro/escuro do Windows.
    Theme(Option<Theme>),
    // As três opções de sorteio moram na aba de Builds, mas são preferências
    // como as outras: gravam no mesmo arquivo e pelo mesmo caminho.
    BuildMatchSet(bool),
    BuildBalanced(bool),
    BuildMaxOneSentry(bool),
    DebugMode(bool),
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
            Change::Theme(theme) => settings.theme = *theme,
            Change::BuildMatchSet(on) => settings.build_match_set = *on,
            Change::BuildBalanced(on) => settings.build_balanced = *on,
            Change::BuildMaxOneSentry(on) => settings.build_max_one_sentry = *on,
            Change::DebugMode(on) => settings.debug_mode = *on,
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
    ExportBackup,
    ImportBackup,
    /// Relatório do modo debug, pelo mesmo diálogo adiado do backup.
    ExportReport,
    /// Pasta de configuração no Explorer, onde ficam `debug.jsonl` e `app.log`.
    OpenFolder,
    TypingTest,
}

/// O que a aba precisa saber do resto do app.
pub struct Ctx<'a> {
    pub settings: &'a Settings,
    /// Limite de FPS do próprio jogo, lido do `user_settings.config`.
    pub fps_cap: Option<u32>,
    /// Números do modo debug, montados pela janela só com ele ligado.
    pub debug: Option<&'a DebugView>,
    /// Teste de digitação em andamento, ou o último que terminou.
    pub typing: Option<&'a TypingTest>,
}

/// O que o card de diagnóstico mostra do modo debug.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DebugView {
    pub stats: Stats,
    pub hook: HookHealth,
    pub window: WindowInfo,
}

impl Ctx<'_> {
    fn tr(&self) -> &'static Tr {
        i18n::tr(self.settings.language)
    }

    /// Limite do jogo e o perfil que ele comporta, quando o escolhido é rápido
    /// demais para ele. Mais lento que o necessário nunca gera aviso.
    fn fps_warning(&self) -> Option<(u32, Speed)> {
        let fps = self.fps_cap?;
        let safe = engine::fastest_safe_speed(fps);
        (self.settings.macro_speed > safe).then_some((fps, safe))
    }

    fn typing_running(&self) -> bool {
        self.typing.is_some_and(|test| !test.done())
    }
}

/// Estado da aba entre passagens de construção.
#[derive(Debug, Default)]
pub struct SettingsTab {
    capturing: Option<Capture>,
    backup_status: Option<BackupStatus>,
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

/// As três escolhas de tema: seguir o sistema, `rose` (escuro) e `crimson`
/// (claro).
const THEMES: [Option<Theme>; 3] = [None, Some(Theme::Rose), Some(Theme::Crimson)];

fn theme_id(index: usize) -> Id {
    id_at("settings.theme", index)
}

fn export_id() -> Id {
    id("settings.export")
}

fn import_id() -> Id {
    id("settings.import")
}

/// Chave da animação que apaga o aviso de backup. A janela dispara o pulso ao
/// terminar a operação; a aba só lê o valor.
pub fn backup_flash_id() -> Id {
    id("settings.backup.status")
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

fn debug_id() -> Id {
    id("settings.debug")
}

fn typing_id() -> Id {
    id("settings.debug.typing")
}

fn report_id() -> Id {
    id("settings.debug.report")
}

fn folder_id() -> Id {
    id("settings.debug.folder")
}

// --- Estilos ---

fn label_style() -> TextStyle {
    styles::label().middle()
}

fn hint_style() -> TextStyle {
    styles::hint()
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

    /// Desiste da captura em curso; a janela chama ao sair da aba.
    pub fn cancel_capture(&mut self) {
        self.capturing = None;
    }

    /// Resultado do último backup. A janela também dispara o pulso de
    /// [`backup_flash_id`], que é quem apaga o aviso depois de 2,5s.
    pub fn set_backup_status(&mut self, status: BackupStatus) {
        self.backup_status = Some(status);
    }

    // --- Construção ---

    pub fn build(&mut self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        let view = Rect::new(
            area.x + PAGE_PADDING,
            area.y + PAGE_TOP,
            area.w - PAGE_PADDING * 2.0,
            area.h - PAGE_TOP,
        );
        let width = view.w - SCROLL_GUTTER;
        let half = (width - CARD_GAP) / 2.0;

        let offset = ui.scroll_begin(scroll_id(), view);
        let mut y = view.y - offset;

        // Atalhos, idioma e tema empilhados à esquerda; os controles, que são
        // o painel mais alto, à direita.
        let controls = controls_height(measure, half, ctx);
        let row = left_column_height().max(controls);
        let top = columns(Rect::new(view.x, y, width, row), 2, CARD_GAP);
        let mut left = top[0];
        self.shortcuts_card(ui, measure, left.cut_top(shortcuts_height()), ctx);
        left.skip_top(SECTION_GAP);
        self.language_card(ui, left.cut_top(language_height()), ctx);
        left.skip_top(SECTION_GAP);
        self.theme_card(ui, left.cut_top(language_height()), ctx);
        self.controls_card(ui, measure, top[1].with_h(controls), ctx);
        y += row + SECTION_GAP;

        let height = support_height(width);
        self.support_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
        y += height + SECTION_GAP;

        let height = backup_height(measure, width, ctx);
        self.backup_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
        y += height + SECTION_GAP;

        let height = diagnostics_height(measure, width, ctx);
        self.diagnostics_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
        // A sombra do último painel e um respiro antes do rodapé.
        y += height + theme::SHADOW + PAGE_TOP;

        ui.scroll_end(scroll_id(), view, y - (view.y - offset));

        // O toast sai por cima da página, no canto de baixo, fora da rolagem.
        self.backup_toast(ui, area, ctx);
    }

    /// Card "Atalhos de Combate": os quatro atalhos de slot, dois por linha.
    fn shortcuts_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader::new(ctx.tr().settings.keybinding, "cfg")),
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
                measure,
                box_rect,
                &format!("{} {}", ctx.tr().settings.shortcut_label, index + 1),
                shortcut_id(index),
                ctx.settings.shortcut(index),
                Capture::Slot(index),
                ctx,
            );
        }
    }

    /// Caixa aninhada "rótulo + botão de atalho".
    #[allow(clippy::too_many_arguments)]
    fn key_box(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
        rect: Rect,
        label: &str,
        id: Id,
        bound: Option<&str>,
        capture: Capture,
        ctx: &Ctx,
    ) {
        widgets::inset_box(ui, rect, 0.0);

        let mut content = rect.inset(KEY_BOX_PADDING);
        let label_row = content.cut_top(LABEL_H);
        ui.text(
            label_row,
            label.to_uppercase(),
            label_style(),
            theme::palette().muted,
        );
        content.skip_top(LABEL_GAP);
        self.key_button(
            ui,
            measure,
            id,
            content.with_h(KEY_BUTTON_H),
            bound,
            capture,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn key_button(
        &self,
        ui: &mut Ui,
        measure: &mut dyn Measure,
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
        widgets::key_button(ui, measure, id, rect, label, capturing);
    }

    /// Card "Config. de Controles": modificador, velocidade e os três toggles.
    fn controls_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let palette = theme::palette();
        let mut content = widgets::card(
            ui,
            rect,
            Some(CardHeader::new(tr.settings.controller, "cfg")),
        );

        // Tecla de estratagema do jogo: quatro botões em duas linhas.
        ui.text(
            content.cut_top(LABEL_H),
            tr.settings.ingame_key.to_uppercase(),
            label_style(),
            palette.muted,
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

        // Velocidade: rótulo, explicação, os cinco perfis em duas linhas e o
        // aviso quando o limite de FPS do jogo não comporta o escolhido.
        ui.text(
            content.cut_top(LABEL_H),
            tr.settings.macro_speed.to_uppercase(),
            label_style(),
            palette.muted,
        );
        content.skip_top(LABEL_GAP);
        let desc_h = measure
            .text_size(tr.settings.macro_speed_desc, hint_style(), content.w)
            .1;
        ui.text(
            content.cut_top(desc_h),
            tr.settings.macro_speed_desc,
            hint_style(),
            palette.muted,
        );
        content.skip_top(LABEL_GAP);
        let grid = content.cut_top(speed_grid_height());
        for (index, speed) in Speed::ALL.iter().enumerate() {
            let cell = grid_cell(
                grid,
                SPEED_COLUMNS,
                widgets::CHOICE_SPLIT_HEIGHT,
                CHOICE_GAP,
                index,
            );
            widgets::choice_button(
                ui,
                speed_id(index),
                cell,
                tr.settings.speed(*speed),
                ctx.settings.macro_speed == *speed,
            );
        }
        if let Some((fps, suggested)) = ctx.fps_warning() {
            let (title, body) = tr.settings.fps_cap_hint(fps, suggested);
            content.skip_top(LABEL_GAP);
            let height = widgets::alert_height(measure, &body, content.w);
            widgets::alert(ui, content.cut_top(height), palette.warning, &title, &body);
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
            Some(CardHeader::new(ctx.tr().settings.language, "cfg")),
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

    /// Card "Tema": seguir o sistema, escuro (`rose`) ou claro (`crimson`).
    /// O toggle da topbar e o `Shift+T` fazem o mesmo, sem a opção do sistema.
    fn theme_card(&self, ui: &mut Ui, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let content = widgets::card(ui, rect, Some(CardHeader::new(tr.settings.theme, "cfg")));
        let row = content.with_h(CHOICE_H);
        for (index, choice) in THEMES.into_iter().enumerate() {
            let label = match choice {
                None => tr.settings.theme_system,
                Some(Theme::Rose) => tr.settings.theme_dark,
                Some(Theme::Crimson) => tr.settings.theme_light,
            };
            let cell = grid_cell(row, THEMES.len(), CHOICE_H, CHOICE_GAP, index);
            widgets::choice_button(
                ui,
                theme_id(index),
                cell,
                label,
                ctx.settings.theme == choice,
            );
        }
    }

    /// Card "Estratagemas de Apoio Fixo": três tiles, cada um com o seu botão
    /// de atalho.
    fn support_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let content = widgets::card(
            ui,
            rect,
            Some(CardHeader::new(ctx.tr().settings.support, "cfg")),
        );
        let cols = columns(content, SUPPORT_COUNT, SUPPORT_GAP);

        for (index, support) in SUPPORT_STRATS.iter().enumerate() {
            // Tile e botão com a mesma largura, centralizados na coluna.
            let column = cols[index];
            let width = support_tile(column.w);
            let mut col = Rect::new(
                column.x + (column.w - width) / 2.0,
                column.y,
                width,
                column.h,
            );
            let card = col.cut_top(support_tile_height(column.w));
            widgets::support_card(ui, index, card, support);
            col.skip_top(SUPPORT_GAP);
            self.key_button(
                ui,
                measure,
                support_shortcut_id(index),
                col.with_h(KEY_BUTTON_H),
                ctx.settings.support_shortcut(index),
                Capture::Support(index),
                ctx,
            );
        }
    }

    /// Card "Backup": a explicação e os dois botões. O resultado sai num toast.
    fn backup_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.settings.backup, "dat")));

        let desc_h = measure
            .text_size(tr.settings.backup_desc, hint_style(), content.w)
            .1;
        ui.text(
            content.cut_top(desc_h),
            tr.settings.backup_desc,
            hint_style(),
            theme::palette().muted,
        );
        content.skip_top(LABEL_GAP + 4.0);

        let buttons = columns(content.cut_top(CHOICE_H), 2, CHOICE_GAP + 4.0);
        widgets::button(
            ui,
            export_id(),
            buttons[0],
            tr.settings.backup_export,
            ButtonVariant::Secondary,
        );
        widgets::button(
            ui,
            import_id(),
            buttons[1],
            tr.settings.backup_import,
            ButtonVariant::Secondary,
        );
    }

    /// Card "Diagnóstico": o modo debug, os três botões, o resultado do teste de
    /// digitação e, com o modo ligado, os números da sessão.
    fn diagnostics_card(&self, ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
        let tr = ctx.tr();
        let text = &tr.debug;
        let palette = theme::palette();
        let mut content = widgets::card(ui, rect, Some(CardHeader::new(text.title, "log")));

        let desc_h = measure.text_size(text.desc, hint_style(), content.w).1;
        ui.text(
            content.cut_top(desc_h),
            text.desc,
            hint_style(),
            palette.muted,
        );
        content.skip_top(LABEL_GAP + 4.0);

        let mode_desc = match (ctx.settings.debug_mode, ctx.settings.enable_overlay) {
            (false, _) => text.mode_off,
            (true, true) => text.mode_on,
            (true, false) => text.mode_on_no_overlay,
        };
        widgets::toggle_row(
            ui,
            debug_id(),
            content.cut_top(TOGGLE_H),
            text.mode,
            mode_desc,
            ctx.settings.debug_mode,
        );
        content.skip_top(LABEL_GAP + 4.0);

        let buttons = columns(content.cut_top(CHOICE_H), 3, CHOICE_GAP + 4.0);
        let typing = if ctx.typing_running() {
            ButtonVariant::Disabled
        } else {
            ButtonVariant::Secondary
        };
        widgets::button(ui, typing_id(), buttons[0], text.typing, typing);
        widgets::button(
            ui,
            report_id(),
            buttons[1],
            text.export,
            ButtonVariant::Secondary,
        );
        widgets::button(
            ui,
            folder_id(),
            buttons[2],
            text.folder,
            ButtonVariant::Secondary,
        );
        content.skip_top(LABEL_GAP + 4.0);

        for (line, color) in typing_lines(ctx) {
            let height = measure.text_size(&line, hint_style(), content.w).1;
            ui.text(content.cut_top(height), line, hint_style(), color);
        }

        let Some(view) = ctx.debug else {
            return;
        };
        content.skip_top(LABEL_GAP + 4.0);
        let label_w = content.w * STAT_LABEL_SHARE;
        for (label, value) in stat_rows(view, text) {
            let mut row = content.cut_top(STAT_ROW_H);
            ui.text(
                row.cut_left(label_w),
                label.to_uppercase(),
                styles::micro().middle(),
                palette.muted,
            );
            ui.text(row, value, stat_value_style(), palette.content);
        }
    }

    /// Toast do último backup (§6.8), no canto de baixo da aba, apagando com o
    /// pulso disparado pela janela: fica inteiro e some no fade de saída.
    fn backup_toast(&self, ui: &mut Ui, area: Rect, ctx: &Ctx) {
        let Some(status) = self.backup_status else {
            return;
        };
        let left = ui.anim(backup_flash_id(), BACKUP_STATUS_MS);
        if left <= 0.0 {
            return;
        }
        // O pulso anda de 1 a 0 nos 2,5s do aviso; só o fim dele é a saída.
        let exit = motion::EXIT_MS as f32 / BACKUP_STATUS_MS as f32;
        let visible = (left / exit).min(1.0);

        let tr = ctx.tr();
        let palette = theme::palette();
        let (title, text, tone) = match status {
            BackupStatus::Exported => (
                tr.settings.toast_done,
                tr.settings.backup_exported,
                palette.success,
            ),
            BackupStatus::Imported => (
                tr.settings.toast_done,
                tr.settings.backup_imported,
                palette.success,
            ),
            BackupStatus::Failed => (
                tr.settings.toast_error,
                tr.settings.backup_error,
                palette.error,
            ),
            BackupStatus::ReportExported => {
                (tr.settings.toast_done, tr.debug.exported, palette.success)
            }
            BackupStatus::ReportFailed => (
                tr.settings.toast_error,
                tr.debug.export_error,
                palette.error,
            ),
        };
        let rect = Rect::new(
            area.right() - TOAST_MARGIN - theme::SHADOW - widgets::TOAST_WIDTH,
            area.bottom() - TOAST_MARGIN - theme::SHADOW - widgets::TOAST_HEIGHT,
            widgets::TOAST_WIDTH,
            widgets::TOAST_HEIGHT,
        );
        widgets::toast(ui, rect, tone, title, text, visible);
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
        for (index, choice) in THEMES.into_iter().enumerate() {
            if clicked == theme_id(index) {
                return Some(Action::Setting(Change::Theme(choice)));
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
        if clicked == export_id() {
            return Some(Action::ExportBackup);
        }
        if clicked == import_id() {
            return Some(Action::ImportBackup);
        }
        if clicked == debug_id() {
            return Some(Action::Setting(Change::DebugMode(!settings.debug_mode)));
        }
        if clicked == typing_id() {
            // Desabilitado no meio de um teste: o clique não reinicia nada.
            return Some(if ctx.typing_running() {
                Action::Redraw
            } else {
                Action::TypingTest
            });
        }
        if clicked == report_id() {
            return Some(Action::ExportReport);
        }
        if clicked == folder_id() {
            return Some(Action::OpenFolder);
        }
        None
    }

    /// Tecla recebida pela janela enquanto a aba espera um atalho. `None` quando
    /// não há captura em curso; aí a tecla segue seu caminho normal.
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
    let mut speed = LABEL_H + LABEL_GAP + desc_h + LABEL_GAP + speed_grid_height();
    if let Some((fps, suggested)) = ctx.fps_warning() {
        let (_, body) = ctx.tr().settings.fps_cap_hint(fps, suggested);
        speed += LABEL_GAP + widgets::alert_height(measure, &body, inner);
    }
    let toggles = TOGGLE_H * 3.0 + TOGGLE_GAP * 2.0;
    widgets::card_chrome(true) + modifiers + ROW_GAP + speed + ROW_GAP + toggles
}

fn speed_grid_height() -> f32 {
    grid_height(
        Speed::ALL.len(),
        SPEED_COLUMNS,
        widgets::CHOICE_SPLIT_HEIGHT,
        CHOICE_GAP,
    )
}

fn language_height() -> f32 {
    widgets::card_chrome(true) + CHOICE_H
}

/// Coluna da esquerda da primeira faixa: atalhos, idioma e tema.
fn left_column_height() -> f32 {
    shortcuts_height() + (SECTION_GAP + language_height()) * 2.0
}

fn backup_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let desc_h = measure
        .text_size(ctx.tr().settings.backup_desc, hint_style(), inner)
        .1;
    widgets::card_chrome(true) + desc_h + LABEL_GAP + 4.0 + CHOICE_H
}

fn stat_value_style() -> TextStyle {
    TextStyle::new(font::SIZE_LABEL, Weight::Regular).middle()
}

/// As linhas "rótulo · valor" das estatísticas do modo debug.
fn stat_rows(view: &DebugView, text: &DebugText) -> Vec<(&'static str, String)> {
    let stats = &view.stats;
    vec![
        (text.stat_calls, text.calls(stats)),
        (text.stat_hold, text.spread(&stats.hold)),
        (text.stat_gap, text.spread(&stats.gap)),
        (text.stat_rejected, stats.rejected_keys.to_string()),
        (text.stat_held, stats.with_held_keys.to_string()),
        (text.stat_ignored, stats.ignored.to_string()),
        (text.stat_hook, text.hook(&view.hook)),
        (text.stat_window, window_line(&view.window)),
        (text.stat_last, last_line(stats.last.as_ref(), text)),
    ]
}

fn window_line(window: &WindowInfo) -> String {
    match (window.class, window.exe.as_deref()) {
        ("", _) => "\u{2014}".to_string(),
        (class, Some(exe)) => format!("{class} \u{00B7} {exe}"),
        (class, None) => class.to_string(),
    }
}

fn last_line(last: Option<&LastRun>, text: &DebugText) -> String {
    let Some(last) = last else {
        return "\u{2014}".to_string();
    };
    let mut parts = vec![
        last.stratagem.clone().unwrap_or_else(|| "?".to_string()),
        text.outcome(last.outcome).to_string(),
    ];
    if let Some(hold) = last.min_hold_ms {
        parts.push(text.hud_min_hold(hold));
    }
    parts.join(" \u{00B7} ")
}

/// O que o card diz do teste de digitação: a explicação antes do primeiro, o
/// andamento durante, o veredito depois.
fn typing_lines(ctx: &Ctx) -> Vec<(String, crate::ui::theme::Color)> {
    let text = &ctx.tr().debug;
    let palette = theme::palette();
    let Some(test) = ctx.typing else {
        return vec![(text.typing_idle.to_string(), palette.muted)];
    };
    if !test.done() {
        let (done, total) = test.progress();
        return vec![(text.typing_running(done, total), palette.accent_text)];
    }
    let (passed, lines) = text.typing_result(&test.summary());
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let color = match (index, passed) {
                (0, true) => palette.success.text,
                (0, false) => palette.error.text,
                _ => palette.muted,
            };
            (line, color)
        })
        .collect()
}

fn diagnostics_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let text = &ctx.tr().debug;
    let desc_h = measure.text_size(text.desc, hint_style(), inner).1;
    let typing_h: f32 = typing_lines(ctx)
        .iter()
        .map(|(line, _)| measure.text_size(line, hint_style(), inner).1)
        .sum();
    let stats_h = ctx.debug.map_or(0.0, |view| {
        LABEL_GAP + 4.0 + STAT_ROW_H * stat_rows(view, text).len() as f32
    });
    widgets::card_chrome(true)
        + desc_h
        + (LABEL_GAP + 4.0) * 3.0
        + TOGGLE_H
        + CHOICE_H
        + typing_h
        + stats_h
}

/// Largura de cada tile de apoio: a coluna inteira até um teto; acima dele o
/// ícone viraria um pôster e empurraria o resto da aba para baixo.
fn support_tile(column: f32) -> f32 {
    column.min(SUPPORT_TILE_MAX)
}

fn support_tile_height(column: f32) -> f32 {
    widgets::tile_height(support_tile(column))
}

const SUPPORT_TILE_MAX: f32 = 140.0;

fn support_height(width: f32) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let column = (inner - SUPPORT_GAP * (SUPPORT_COUNT - 1) as f32) / SUPPORT_COUNT as f32;
    widgets::card_chrome(true) + support_tile_height(column) + SUPPORT_GAP + KEY_BUTTON_H
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
        Ctx {
            settings,
            fps_cap: None,
            debug: None,
            typing: None,
        }
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
        expected.extend((0..THEMES.len()).map(theme_id));
        expected.extend([
            arrows_id(),
            overlay_id(),
            hud_id(),
            export_id(),
            import_id(),
            debug_id(),
            typing_id(),
            report_id(),
            folder_id(),
        ]);

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
            tab.on_click(speed_id(0), &ctx(&settings)),
            Some(Action::Setting(Change::Speed(Speed::Potato)))
        );
        assert_eq!(
            tab.on_click(speed_id(1), &ctx(&settings)),
            Some(Action::Setting(Change::Speed(Speed::Low)))
        );
        assert_eq!(
            tab.on_click(speed_id(4), &ctx(&settings)),
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
            Change::BuildMatchSet(false),
            Change::BuildBalanced(true),
            Change::BuildMaxOneSentry(true),
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
        assert!(!settings.build_match_set);
        assert!(settings.build_balanced);
        assert!(settings.build_max_one_sentry);

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
    fn the_theme_card_offers_the_system_and_both_themes() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        assert_eq!(
            tab.on_click(theme_id(0), &ctx(&settings)),
            Some(Action::Setting(Change::Theme(None)))
        );
        assert_eq!(
            tab.on_click(theme_id(2), &ctx(&settings)),
            Some(Action::Setting(Change::Theme(Some(Theme::Crimson))))
        );

        let mut applied = settings.clone();
        Change::Theme(Some(Theme::Rose)).apply(&mut applied);
        assert_eq!(applied.theme, Some(Theme::Rose));
        Change::Theme(None).apply(&mut applied);
        assert_eq!(applied.theme, None);
    }

    #[test]
    fn the_backup_buttons_ask_for_the_file_dialogs() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        assert_eq!(
            tab.on_click(export_id(), &ctx(&settings)),
            Some(Action::ExportBackup)
        );
        assert_eq!(
            tab.on_click(import_id(), &ctx(&settings)),
            Some(Action::ImportBackup)
        );
    }

    #[test]
    fn the_backup_message_shows_up_and_fades_away() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();

        build_settled(&mut tab, &mut ui, &settings);
        let quiet = texts(&ui);
        assert!(!quiet.iter().any(|text| text.contains("EXPORTADO")));

        tab.set_backup_status(BackupStatus::Exported);
        ui.flash(backup_flash_id(), BACKUP_STATUS_MS);
        build_at(&mut tab, &mut ui, &settings, 2_000);
        assert!(texts(&ui)
            .iter()
            .any(|text| text.contains("BACKUP EXPORTADO")));

        // Passado o tempo do aviso, ele sai da tela sozinho.
        build_at(
            &mut tab,
            &mut ui,
            &settings,
            2_000 + BACKUP_STATUS_MS as u64 * 2,
        );
        assert_eq!(texts(&ui), quiet);
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
            .any(|text| text.contains("ATALHOS_DE_COMBATE")));

        let en = Settings {
            language: Language::En,
            ..Settings::default()
        };
        build(&mut tab, &mut ui, &en);
        let after = texts(&ui);
        assert!(after.iter().any(|text| text.contains("COMBAT_SHORTCUTS")));
        assert!(!after.iter().any(|text| text.contains("ATALHOS_DE_COMBATE")));
    }

    #[test]
    fn a_game_capped_below_the_profile_suggests_a_slower_one() {
        let mut tab = SettingsTab::new();
        let mut ui = Ui::new();
        let warned = |ui: &Ui| {
            texts(ui)
                .iter()
                .any(|text| text.contains("JOGO LIMITADO A 30 FPS"))
        };
        let build_capped = |tab: &mut SettingsTab, ui: &mut Ui, settings: &Settings, cap| {
            ui.begin(0);
            let ctx = Ctx {
                settings,
                fps_cap: cap,
                debug: None,
                typing: None,
            };
            tab.build(ui, &mut Fixed, AREA, &ctx);
            ui.end();
        };

        // Padrão a 30fps: o hold não cobre um quadro, e a dica aponta o Baixo FPS.
        let normal = Settings::default();
        build_capped(&mut tab, &mut ui, &normal, Some(30));
        assert!(warned(&ui));
        assert!(texts(&ui).iter().any(|text| text.contains("\"Baixo FPS\"")));

        // Com o perfil certo, sem limite ou com um limite folgado, nada.
        let low = Settings {
            macro_speed: Speed::Low,
            ..Settings::default()
        };
        build_capped(&mut tab, &mut ui, &low, Some(30));
        assert!(!warned(&ui));
        build_capped(&mut tab, &mut ui, &normal, None);
        assert!(!warned(&ui));
        build_capped(&mut tab, &mut ui, &normal, Some(60));
        assert!(!texts(&ui).iter().any(|text| text.contains("LIMITADO")));

        // Abaixo de 30, nem o Baixo FPS cobre: a dica aponta o Batata.
        build_capped(&mut tab, &mut ui, &low, Some(20));
        assert!(texts(&ui).iter().any(|text| text.contains("\"Batata\"")));
    }

    #[test]
    fn an_unknown_click_is_not_ours() {
        let settings = Settings::default();
        let mut tab = SettingsTab::new();
        assert_eq!(tab.on_click(id("outra.tela"), &ctx(&settings)), None);
    }
}
