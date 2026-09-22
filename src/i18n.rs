//! Traduções pt/en.
//!
//! Porte integral de `legacy/src/renderer/data/translations.js`, mais as frases
//! que a v1 deixou embutidas no JSX com ternário de idioma. Ficaram de fora as
//! chaves da intro de boot (a animação foi cortada: o app abre direto) e as do
//! modificador de corrida (removido junto com a feature).

use std::fmt::Display;

use crate::data::EquipSlot;
use crate::diag::typing::Summary;
use crate::diag::{HookHealth, RunOutcome, Spread, Stats};
use crate::meta_stats::Faction;
use crate::settings::{Language, Speed};

pub struct Tr {
    pub tabs: Tabs,
    pub macros: Macros,
    pub build: Build,
    pub settings: SettingsText,
    pub overlay: Overlay,
    pub update: Update,
    pub tray: Tray,
    pub debug: DebugText,
}

pub struct Tabs {
    pub macro_tab: &'static str,
    pub build: &'static str,
    pub settings: &'static str,
}

pub struct Macros {
    pub select_title: &'static str,
    pub others: &'static str,
    pub listening: &'static str,
    pub bind: &'static str,
    pub clear_slot: &'static str,
    pub search_placeholder: &'static str,
    pub search_no_results: &'static str,
    /// Slot sem estratagema, entre colchetes na tela (`[VAZIO]`).
    pub empty: &'static str,
    /// Kicker do estado vazio (`> NADA AQUI`).
    pub nothing_here: &'static str,
}

pub struct Build {
    pub sub_meta: &'static str,
    pub sub_random: &'static str,
    pub sub_custom: &'static str,
    pub sub_saved: &'static str,
    pub saved: &'static str,
    pub saved_empty: &'static str,
    pub saved_hint: &'static str,
    pub saved_no_gear: &'static str,
    pub save_placeholder: &'static str,
    pub save_build: &'static str,
    pub save_changes: &'static str,
    pub save_as_new: &'static str,
    pub replace_build: &'static str,
    pub discard: &'static str,
    pub apply_build: &'static str,
    pub edit_build: &'static str,
    pub delete_build: &'static str,
    pub confirm_delete: &'static str,
    pub in_slots: &'static str,
    /// Card da build exibida e as linhas de estado do salvar.
    pub current: &'static str,
    pub status_new: &'static str,
    pub status_saved: &'static str,
    pub status_editing: &'static str,
    pub status_unsaved: &'static str,
    pub status_replace: &'static str,
    pub status_taken: &'static str,
    pub status_empty: &'static str,
    /// Fim da frase do toast, depois do nome da build entre aspas.
    pub notice_saved: &'static str,
    pub notice_updated: &'static str,
    pub notice_applied: &'static str,
    pub notice_deleted: &'static str,
    /// Toast de aplicar uma build que ainda não tem nome.
    pub notice_applied_current: &'static str,
    pub generate: &'static str,
    pub reroll: &'static str,
    pub apply_stratagems: &'static str,
    pub hint: &'static str,
    pub stratagems: &'static str,
    pub stratagem: &'static str,
    pub equipment: &'static str,
    pub primary: &'static str,
    pub secondary: &'static str,
    pub grenade: &'static str,
    pub armor: &'static str,
    pub helmet: &'static str,
    pub cape: &'static str,
    pub booster: &'static str,
    pub lock: &'static str,
    pub unlock: &'static str,
    pub weight_light: &'static str,
    pub weight_medium: &'static str,
    pub weight_heavy: &'static str,
    pub meta: &'static str,
    pub meta_generate: &'static str,
    pub faction_terminid: &'static str,
    pub faction_automaton: &'static str,
    pub faction_illuminate: &'static str,
    pub meta_difficulty_all: &'static str,
    pub meta_top_strats: &'static str,
    pub meta_top_passives: &'static str,
    pub meta_loading: &'static str,
    pub meta_error: &'static str,
    pub meta_credit: &'static str,
    pub meta_games: &'static str,
    pub meta_new: &'static str,
    pub match_set: &'static str,
    pub match_set_on: &'static str,
    pub match_set_off: &'static str,
    pub balanced: &'static str,
    pub balanced_on: &'static str,
    pub balanced_off: &'static str,
    pub max_sentry: &'static str,
    pub max_sentry_on: &'static str,
    pub max_sentry_off: &'static str,
    pub set_badge: &'static str,
    pub custom_title: &'static str,
    pub custom_hint: &'static str,
    pub custom_import: &'static str,
    pub custom_clear: &'static str,
    pub custom_equipment: &'static str,
    pub equip_none: &'static str,
}

pub struct SettingsText {
    pub keybinding: &'static str,
    pub controller: &'static str,
    pub support: &'static str,
    pub shortcut_label: &'static str,
    pub support_shortcut_label: &'static str,
    pub ingame_key: &'static str,
    pub game_active: &'static str,
    pub game_inactive: &'static str,
    pub macro_speed: &'static str,
    pub macro_speed_desc: &'static str,
    pub macro_speed_potato: &'static str,
    pub macro_speed_low: &'static str,
    pub macro_speed_normal: &'static str,
    pub macro_speed_fast: &'static str,
    pub macro_speed_turbo: &'static str,
    /// Aviso de limite de FPS do jogo. `{fps}` e `{profile}` são trocados na
    /// hora (ver [`SettingsText::fps_cap_hint`]).
    pub fps_cap_title: &'static str,
    pub fps_cap_hint: &'static str,
    pub arrow_mode: &'static str,
    pub arrow_active: &'static str,
    pub wasd_active: &'static str,
    pub version: &'static str,
    pub updated: &'static str,
    pub language: &'static str,
    pub update_checking: &'static str,
    pub update_available: &'static str,
    pub update_download: &'static str,
    pub update_downloading: &'static str,
    pub update_ready: &'static str,
    pub update_up_to_date: &'static str,
    pub update_error: &'static str,
    pub update_retry: &'static str,
    pub tag_offensive: &'static str,
    pub tag_supply: &'static str,
    pub tag_defensive: &'static str,
    pub backup: &'static str,
    pub backup_desc: &'static str,
    pub backup_export: &'static str,
    pub backup_import: &'static str,
    pub backup_exported: &'static str,
    pub backup_imported: &'static str,
    pub backup_error: &'static str,
    // Estas quatro estavam no JSX com ternário de idioma, não no translations.js.
    pub overlay_shortcut: &'static str,
    pub overlay_enabled: &'static str,
    pub overlay_disabled: &'static str,
    pub persistent_hud: &'static str,
    pub persistent_hud_on: &'static str,
    pub persistent_hud_off: &'static str,
    pub theme: &'static str,
    pub theme_system: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    /// Títulos do toast do backup.
    pub toast_done: &'static str,
    pub toast_error: &'static str,
}

pub struct Overlay {
    pub warning_title: &'static str,
    pub fullscreen_warning: &'static str,
}

/// Modal de "atualização pronta", também embutido no JSX da v1.
pub struct Update {
    pub title: &'static str,
    pub body: &'static str,
    pub restart_now: &'static str,
    pub later: &'static str,
}

/// Card de diagnóstico, painel de teclas do overlay e teste de digitação. Os
/// textos com `{chave}` são preenchidos pelos métodos de [`DebugText`].
pub struct DebugText {
    pub title: &'static str,
    pub desc: &'static str,
    pub mode: &'static str,
    pub mode_on: &'static str,
    pub mode_on_no_overlay: &'static str,
    pub mode_off: &'static str,
    pub typing: &'static str,
    pub export: &'static str,
    pub folder: &'static str,
    pub exported: &'static str,
    pub export_error: &'static str,
    pub stat_calls: &'static str,
    pub stat_rejected: &'static str,
    pub stat_hold: &'static str,
    pub stat_gap: &'static str,
    pub stat_held: &'static str,
    pub stat_ignored: &'static str,
    pub stat_hook: &'static str,
    pub stat_window: &'static str,
    pub stat_last: &'static str,
    pub calls_value: &'static str,
    pub spread_value: &'static str,
    pub hook_value: &'static str,
    pub hook_idle: &'static str,
    pub completed: &'static str,
    pub aborted: &'static str,
    pub blocked: &'static str,
    pub unfocused: &'static str,
    pub typing_idle: &'static str,
    pub typing_running: &'static str,
    pub typing_passed: &'static str,
    pub typing_failed: &'static str,
    pub typing_interrupted: &'static str,
    pub typing_blocked: &'static str,
    pub typing_holds: &'static str,
    pub hud_waiting: &'static str,
    pub hud_min_hold: &'static str,
}

/// Menu do ícone da bandeja. A v1 escrevia os dois em português direto no
/// `Menu.buildFromTemplate`; aqui eles seguem o idioma escolhido, como o resto.
pub struct Tray {
    pub open: &'static str,
    pub exit: &'static str,
}

impl Build {
    /// Rótulo da categoria de equipamento.
    pub fn equip_label(&self, slot: EquipSlot) -> &'static str {
        match slot {
            EquipSlot::Primary => self.primary,
            EquipSlot::Secondary => self.secondary,
            EquipSlot::Grenade => self.grenade,
            EquipSlot::Armor => self.armor,
            EquipSlot::Helmet => self.helmet,
            EquipSlot::Cape => self.cape,
            EquipSlot::Booster => self.booster,
        }
    }

    /// Nome da facção na linha de escolha da sub-aba Meta.
    pub fn faction(&self, faction: Faction) -> &'static str {
        match faction {
            Faction::Terminid => self.faction_terminid,
            Faction::Automaton => self.faction_automaton,
            Faction::Illuminate => self.faction_illuminate,
        }
    }

    /// Peso da armadura, como o `peso` vem do `equipment.json`.
    pub fn weight(&self, peso: &str) -> &'static str {
        match peso {
            "Light" => self.weight_light,
            "Heavy" => self.weight_heavy,
            _ => self.weight_medium,
        }
    }
}

impl SettingsText {
    /// Rótulo do perfil de velocidade.
    pub fn speed(&self, speed: Speed) -> &'static str {
        match speed {
            Speed::Potato => self.macro_speed_potato,
            Speed::Low => self.macro_speed_low,
            Speed::Normal => self.macro_speed_normal,
            Speed::Fast => self.macro_speed_fast,
            Speed::Turbo => self.macro_speed_turbo,
        }
    }

    /// Só o nome do perfil, sem o FPS: "Baixo FPS".
    pub fn speed_name(&self, speed: Speed) -> &'static str {
        let label = self.speed(speed);
        label
            .split_once(" \u{00B7} ")
            .map_or(label, |(name, _)| name)
    }

    /// Título e texto do aviso de limite de FPS, já com os números.
    pub fn fps_cap_hint(&self, fps: u32, suggested: Speed) -> (String, String) {
        (
            self.fps_cap_title.replace("{fps}", &fps.to_string()),
            self.fps_cap_hint
                .replace("{profile}", &format!("\"{}\"", self.speed_name(suggested))),
        )
    }

    /// Rótulo da seção da grade de estratagemas, por tag.
    pub fn tag(&self, tag: &str, fallback: &'static str) -> &'static str {
        match tag {
            "Offensive" => self.tag_offensive,
            "Supply" => self.tag_supply,
            "Defensive" => self.tag_defensive,
            _ => fallback,
        }
    }
}

/// Troca cada `{chave}` de um texto pelo valor.
fn fill(template: &str, values: &[(&str, &dyn Display)]) -> String {
    values
        .iter()
        .fold(template.to_string(), |text, (key, value)| {
            text.replace(&format!("{{{key}}}"), &value.to_string())
        })
}

impl DebugText {
    pub fn outcome(&self, outcome: RunOutcome) -> &'static str {
        match outcome {
            RunOutcome::Completed => self.completed,
            RunOutcome::Aborted => self.aborted,
            RunOutcome::Blocked => self.blocked,
            RunOutcome::Unfocused => self.unfocused,
        }
    }

    pub fn calls(&self, stats: &Stats) -> String {
        fill(
            self.calls_value,
            &[
                ("runs", &stats.runs),
                ("completed", &stats.completed),
                ("aborted", &stats.aborted),
                ("blocked", &stats.blocked),
                ("unfocused", &stats.unfocused),
            ],
        )
    }

    /// Mínimo, média e máximo em milissegundos inteiros; "—" sem amostra.
    pub fn spread(&self, spread: &Spread) -> String {
        if spread.count == 0 {
            return "\u{2014}".to_string();
        }
        fill(
            self.spread_value,
            &[
                ("min", &format!("{:.0}", spread.min)),
                ("mean", &format!("{:.0}", spread.mean)),
                ("max", &format!("{:.0}", spread.max)),
            ],
        )
    }

    pub fn hook(&self, hook: &HookHealth) -> String {
        let Some(age) = hook.last_key_age_s else {
            return self.hook_idle.to_string();
        };
        fill(
            self.hook_value,
            &[
                ("keys", &hook.keys),
                ("age", &format!("{age:.0}")),
                ("us", &hook.max_callback_us),
            ],
        )
    }

    pub fn typing_running(&self, done: usize, total: usize) -> String {
        fill(self.typing_running, &[("done", &done), ("total", &total)])
    }

    /// Linhas do resultado do teste de digitação, da mais importante para a
    /// menos. `true` na primeira quando tudo chegou.
    pub fn typing_result(&self, summary: &Summary) -> (bool, Vec<String>) {
        let passed = summary.passed == summary.runs && summary.runs > 0;
        let mut lines = vec![if passed {
            fill(
                self.typing_passed,
                &[("passed", &summary.passed), ("runs", &summary.runs)],
            )
        } else {
            fill(
                self.typing_failed,
                &[
                    ("passed", &summary.passed),
                    ("runs", &summary.runs),
                    ("missing", &summary.missing_keys),
                    ("extra", &summary.extra_keys),
                ],
            )
        }];
        if summary.interrupted > 0 {
            lines.push(fill(
                self.typing_interrupted,
                &[("count", &summary.interrupted)],
            ));
        }
        if summary.blocked > 0 {
            lines.push(fill(self.typing_blocked, &[("count", &summary.blocked)]));
        }
        if summary.sent_hold.count > 0 && summary.received_hold.count > 0 {
            lines.push(fill(
                self.typing_holds,
                &[
                    ("sent", &format!("{:.0}", summary.sent_hold.mean)),
                    ("received", &format!("{:.0}", summary.received_hold.mean)),
                ],
            ));
        }
        (passed, lines)
    }

    pub fn hud_min_hold(&self, ms: f32) -> String {
        fill(self.hud_min_hold, &[("ms", &format!("{ms:.0}"))])
    }
}

/// Textos do idioma escolhido.
pub fn tr(language: Language) -> &'static Tr {
    match language {
        Language::Pt => &PT,
        Language::En => &EN,
    }
}

/// Nome do idioma na própria língua, não traduzido, como na v1.
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::Pt => "Português",
        Language::En => "English",
    }
}

pub static PT: Tr = Tr {
    tabs: Tabs {
        macro_tab: "Configurar Macros",
        build: "Builds",
        settings: "Configurações",
    },
    macros: Macros {
        select_title: "Selecionar Estratagemas para o Slot",
        others: "Outros",
        listening: "ESCUTANDO...",
        bind: "VINCULAR",
        clear_slot: "Remover do slot",
        search_placeholder: "Buscar estratagema...",
        search_no_results: "Nenhum estratagema encontrado para",
        empty: "Vazio",
        nothing_here: "Nada aqui",
    },
    build: Build {
        sub_meta: "Meta",
        sub_random: "Aleatória",
        sub_custom: "Personalizada",
        sub_saved: "Salvas",
        saved: "Builds Salvas",
        saved_empty: "Monte ou gere uma build e salve com um nome",
        saved_hint: "Aplicar põe os estratagemas nos slots de macro. Editar abre a build na aba \
                     Personalizada para trocar itens ou o nome.",
        saved_no_gear: "Sem equipamento",
        save_placeholder: "Nome da build...",
        save_build: "Salvar",
        save_changes: "Salvar alterações",
        save_as_new: "Salvar como nova",
        replace_build: "Substituir?",
        discard: "Descartar",
        apply_build: "Aplicar",
        edit_build: "Editar",
        delete_build: "Excluir",
        confirm_delete: "Confirmar?",
        in_slots: "Nos slots",
        current: "Build Atual",
        status_new: "Nova build · ainda não salva",
        status_saved: "Salva como",
        status_editing: "Editando",
        status_unsaved: "alterações não salvas",
        status_replace: "Já existe uma build com esse nome. Clique de novo para substituir",
        status_taken: "Outra build já usa esse nome",
        status_empty: "Adicione pelo menos um estratagema para salvar",
        notice_saved: "salva",
        notice_updated: "atualizada",
        notice_applied: "nos slots de macro",
        notice_deleted: "excluída",
        notice_applied_current: "Estratagemas aplicados nos slots",
        generate: "Gerar Build",
        reroll: "Rolar de novo",
        apply_stratagems: "Aplicar estratagemas nos slots",
        hint: "Clique em Gerar Build para sortear um loadout completo: 4 estratagemas, armas, \
               armadura, capacete, capa e booster. Use o cadeado para manter um item nos \
               próximos sorteios.",
        stratagems: "Estratagemas",
        stratagem: "Estratagema",
        equipment: "Equipamento",
        primary: "Primária",
        secondary: "Secundária",
        grenade: "Granada",
        armor: "Armadura",
        helmet: "Capacete",
        cape: "Capa",
        booster: "Booster",
        lock: "Travar item nos próximos sorteios",
        unlock: "Destravar item",
        weight_light: "Leve",
        weight_medium: "Média",
        weight_heavy: "Pesada",
        meta: "Meta por Facção",
        meta_generate: "Gerar Build Meta",
        faction_terminid: "Terminids",
        faction_automaton: "Automatons",
        faction_illuminate: "Illuminate",
        meta_difficulty_all: "Todas",
        meta_top_strats: "Top Estratagemas",
        meta_top_passives: "Top Passivas de Armadura",
        meta_loading: "Consultando estatísticas da comunidade...",
        meta_error: "Estatísticas indisponíveis no momento",
        meta_credit: "Dados da comunidade via helldive.live",
        meta_games: "partidas analisadas",
        meta_new: "NOVO",
        match_set: "Sets de Armadura",
        match_set_on: "Capacete do mesmo set + capa da warbond; set sem capa → capa aleatória",
        match_set_off: "Armadura, capacete e capa totalmente aleatórios",
        balanced: "Loadout Balanceado",
        balanced_on: "Garante 1 arma de apoio e 1 item de mochila, sem repetir (apoio com \
                      mochila conta pros dois)",
        balanced_off: "Estratagemas sem regra de arma de apoio/mochila",
        max_sentry: "Máx. 1 Torreta",
        max_sentry_on: "No máximo 1 sentinela por build",
        max_sentry_off: "Sem limite de sentinelas",
        set_badge: "SET",
        custom_title: "Montar Build",
        custom_hint: "Escolha o slot, clique nos estratagemas para equipar (clique de novo para \
                      remover) e selecione o equipamento se quiser. Depois dê um nome e salve \
                      no card Build Atual, logo abaixo.",
        custom_import: "Usar slots atuais",
        custom_clear: "Limpar tudo",
        custom_equipment: "Equipamento (opcional)",
        equip_none: "Nenhum",
    },
    settings: SettingsText {
        keybinding: "Atalhos de Combate",
        controller: "Config. de Controles",
        support: "Estratagemas de Apoio Fixo",
        shortcut_label: "Atalho",
        support_shortcut_label: "Atalho de Apoio",
        ingame_key: "Tecla de Estratégias In-Game",
        game_active: "Jogo Detectado",
        game_inactive: "Jogo Ausente",
        macro_speed: "Velocidade do Macro",
        macro_speed_desc: "O jogo lê o teclado uma vez por quadro, então cada perfil segura a \
                           tecla pelo tempo mínimo do FPS indicado. Se o jogo engolir inputs, \
                           use um perfil de FPS menor. Jogo travado em 30 fps, ou caindo \
                           abaixo disso em combate: Baixo FPS. Abaixo de 30 o tempo todo: \
                           Batata",
        macro_speed_potato: "Batata · 15 fps",
        macro_speed_low: "Baixo FPS · 30 fps",
        macro_speed_normal: "Padrão · 40 fps",
        macro_speed_fast: "Rápida · 60 fps",
        macro_speed_turbo: "Turbo · 60+ fps",
        fps_cap_title: "Jogo limitado a {fps} fps",
        fps_cap_hint: "Nesse FPS o perfil escolhido pode segurar uma tecla por menos de um \
                       quadro, e o jogo pode perdê-la. Use {profile}.",
        arrow_mode: "Modo Setas",
        arrow_active: "Setas ativas (Recomendado para movimento)",
        wasd_active: "WASD ativo (Pode travar o personagem)",
        version: "Versão",
        updated: "Atualizado",
        language: "Idioma",
        update_checking: "Verificando...",
        update_available: "Novo update!",
        update_download: "Baixar",
        update_downloading: "Baixando...",
        update_ready: "Instalar agora",
        update_up_to_date: "App atualizado",
        update_error: "Erro no update",
        update_retry: "Tentar de novo",
        tag_offensive: "Ofensivo",
        tag_supply: "Suprimento",
        tag_defensive: "Defensivo",
        backup: "Backup",
        backup_desc: "Exporta ou restaura tudo: loadouts, slots e configurações (atalhos, \
                      velocidade, idioma...)",
        backup_export: "Exportar",
        backup_import: "Importar",
        backup_exported: "Backup exportado!",
        backup_imported: "Backup restaurado!",
        backup_error: "Arquivo inválido",
        overlay_shortcut: "Atalho do Overlay",
        overlay_enabled: "Overlay Ativado (Ctrl + H)",
        overlay_disabled: "Overlay Desativado",
        persistent_hud: "HUD Persistente",
        persistent_hud_on: "Slots sempre visíveis no jogo",
        persistent_hud_off: "Esconder slots ao fechar",
        theme: "Tema",
        theme_system: "Sistema",
        theme_dark: "Escuro",
        theme_light: "Claro",
        toast_done: "Concluído",
        toast_error: "Erro",
    },
    overlay: Overlay {
        warning_title: "Aviso",
        fullscreen_warning: "O jogo está em \"Tela Cheia\": nesse modo o Windows minimiza o \
                             jogo quando o overlay aparece. Mude o vídeo do jogo para \"Tela \
                             Cheia sem Borda\" (mesmo visual e desempenho).",
    },
    update: Update {
        title: "Atualização Disponível",
        body: "Uma nova versão foi baixada e está pronta para ser instalada. Deseja reiniciar o \
               programa agora para aplicar as mudanças?",
        restart_now: "Reiniciar Agora",
        later: "Depois",
    },
    tray: Tray {
        open: "Abrir Macro Helldivers 2",
        exit: "Sair",
    },
    debug: DebugText {
        title: "Diagnóstico",
        desc: "Para investigar estratagema que falha. Ligado, o app grava o tempo real de cada \
               tecla, o que você segurava na hora do disparo e um retrato do PC. Só entram as \
               teclas que o próprio macro manda; o que você digita fora dele, não.",
        mode: "Modo Debug",
        mode_on: "Gravando cada disparo, com o painel de teclas no jogo",
        mode_on_no_overlay: "Gravando cada disparo. Ligue o overlay para ver as teclas no jogo",
        mode_off: "Desligado: nada é gravado",
        typing: "Testar digitação",
        export: "Exportar relatório",
        folder: "Abrir pasta",
        exported: "Relatório exportado!",
        export_error: "Relatório não foi salvo",
        stat_calls: "Disparos",
        stat_rejected: "Recusadas pelo Windows",
        stat_hold: "Tecla segurada",
        stat_gap: "Intervalo entre teclas",
        stat_held: "Com movimento segurado",
        stat_ignored: "Ignorados com o jogo na frente",
        stat_hook: "Hook de teclado",
        stat_window: "Janela da frente",
        stat_last: "Último disparo",
        calls_value: "{runs} · {completed} ok · {aborted} abortados · {blocked} bloqueados · \
                      {unfocused} sem foco",
        spread_value: "mín {min} · média {mean} · máx {max} ms",
        hook_value: "{keys} teclas · a última há {age}s · pior chamada {us} µs",
        hook_idle: "Nenhuma tecla desde que o modo ligou",
        completed: "completo",
        aborted: "abortado",
        blocked: "bloqueado",
        unfocused: "sem foco",
        typing_idle: "O teste digita 10 sequências nesta janela e confere cada tecla que chega. \
                      Não toque no teclado até ele terminar.",
        typing_running: "Testando {done}/{total}... não toque no teclado",
        typing_passed: "Teclado OK: {passed}/{runs} sequências chegaram completas e na ordem",
        typing_failed: "Falhou: {passed}/{runs} completas · {missing} teclas perdidas · {extra} \
                        a mais",
        typing_interrupted: "{count} rodada(s) interrompida(s): a janela perdeu o foco",
        typing_blocked: "{count} rodada(s) recusada(s): um atalho disparou durante o teste",
        typing_holds: "Tecla segurada, em média: {sent} ms mandados, {received} ms recebidos",
        hud_waiting: "Aguardando disparo",
        hud_min_hold: "menor {ms} ms",
    },
};

pub static EN: Tr = Tr {
    tabs: Tabs {
        macro_tab: "Configure Macros",
        build: "Builds",
        settings: "Settings",
    },
    macros: Macros {
        select_title: "Select Stratagems for Slot",
        others: "Others",
        listening: "LISTENING...",
        bind: "BIND",
        clear_slot: "Clear slot",
        search_placeholder: "Search stratagem...",
        search_no_results: "No stratagem found for",
        empty: "Empty",
        nothing_here: "Nothing here",
    },
    build: Build {
        sub_meta: "Meta",
        sub_random: "Random",
        sub_custom: "Custom",
        sub_saved: "Saved",
        saved: "Saved Builds",
        saved_empty: "Build or generate a loadout and save it with a name",
        saved_hint: "Apply puts the stratagems in your macro slots. Edit opens the build in the \
                     Custom tab to change items or the name.",
        saved_no_gear: "No gear",
        save_placeholder: "Build name...",
        save_build: "Save",
        save_changes: "Save changes",
        save_as_new: "Save as new",
        replace_build: "Replace?",
        discard: "Discard",
        apply_build: "Apply",
        edit_build: "Edit",
        delete_build: "Delete",
        confirm_delete: "Confirm?",
        in_slots: "In slots",
        current: "Current Build",
        status_new: "New build · not saved yet",
        status_saved: "Saved as",
        status_editing: "Editing",
        status_unsaved: "unsaved changes",
        status_replace: "A build with this name already exists. Click again to replace it",
        status_taken: "Another build already uses this name",
        status_empty: "Add at least one stratagem to save",
        notice_saved: "saved",
        notice_updated: "updated",
        notice_applied: "in your macro slots",
        notice_deleted: "deleted",
        notice_applied_current: "Stratagems applied to your slots",
        generate: "Generate Build",
        reroll: "Reroll",
        apply_stratagems: "Apply stratagems to slots",
        hint: "Click Generate Build to roll a full loadout: 4 stratagems, weapons, armor, \
               helmet, cape and booster. Use the lock to keep an item across rolls.",
        stratagems: "Stratagems",
        stratagem: "Stratagem",
        equipment: "Equipment",
        primary: "Primary",
        secondary: "Secondary",
        grenade: "Grenade",
        armor: "Armor",
        helmet: "Helmet",
        cape: "Cape",
        booster: "Booster",
        lock: "Lock item across rolls",
        unlock: "Unlock item",
        weight_light: "Light",
        weight_medium: "Medium",
        weight_heavy: "Heavy",
        meta: "Faction Meta",
        meta_generate: "Generate Meta Build",
        faction_terminid: "Terminids",
        faction_automaton: "Automatons",
        faction_illuminate: "Illuminate",
        meta_difficulty_all: "All",
        meta_top_strats: "Top Stratagems",
        meta_top_passives: "Top Armor Passives",
        meta_loading: "Fetching community statistics...",
        meta_error: "Statistics unavailable right now",
        meta_credit: "Community data via helldive.live",
        meta_games: "matches analyzed",
        meta_new: "NEW",
        match_set: "Armor Sets",
        match_set_on: "Matching set helmet + warbond cape; set without a cape → random cape",
        match_set_off: "Armor, helmet and cape fully random",
        balanced: "Balanced Loadout",
        balanced_on: "Guarantees 1 support weapon and 1 backpack item, no doubles (support with \
                      backpack counts as both)",
        balanced_off: "No support weapon/backpack rules",
        max_sentry: "Max 1 Sentry",
        max_sentry_on: "At most 1 sentry per build",
        max_sentry_off: "No sentry limit",
        set_badge: "SET",
        custom_title: "Build Loadout",
        custom_hint: "Pick a slot, click stratagems to equip (click again to remove) and choose \
                      gear if you want. Then name it and save it in the Current Build card \
                      below.",
        custom_import: "Use current slots",
        custom_clear: "Clear all",
        custom_equipment: "Equipment (optional)",
        equip_none: "None",
    },
    settings: SettingsText {
        keybinding: "Combat Shortcuts",
        controller: "Controller Settings",
        support: "Fixed Support Stratagems",
        shortcut_label: "Shortcut",
        support_shortcut_label: "Support Shortcut",
        ingame_key: "In-Game Stratagem Key",
        game_active: "Game Detected",
        game_inactive: "Game Not Found",
        macro_speed: "Macro Speed",
        macro_speed_desc: "The game reads the keyboard once per frame, so each profile holds \
                           the key for at least one frame at the listed FPS. If the game drops \
                           inputs, pick a lower-FPS profile. Game capped at 30 fps, or dipping \
                           below that in combat: Low FPS. Below 30 all the time: Potato",
        macro_speed_potato: "Potato · 15 fps",
        macro_speed_low: "Low FPS · 30 fps",
        macro_speed_normal: "Normal · 40 fps",
        macro_speed_fast: "Fast · 60 fps",
        macro_speed_turbo: "Turbo · 60+ fps",
        fps_cap_title: "Game capped at {fps} fps",
        fps_cap_hint: "At this frame rate the selected profile can hold a key for less than a \
                       frame, and the game may miss it. Use {profile}.",
        arrow_mode: "Arrow Mode",
        arrow_active: "Arrows active (Recommended for movement)",
        wasd_active: "WASD active (May lock character movement)",
        version: "Version",
        updated: "Updated",
        language: "Language",
        update_checking: "Checking...",
        update_available: "New update!",
        update_download: "Download",
        update_downloading: "Downloading...",
        update_ready: "Install now",
        update_up_to_date: "Up to date",
        update_error: "Update error",
        update_retry: "Try again",
        tag_offensive: "Offensive",
        tag_supply: "Supply",
        tag_defensive: "Defensive",
        backup: "Backup",
        backup_desc: "Export or restore everything: loadouts, slots and settings (shortcuts, \
                      speed, language...)",
        backup_export: "Export",
        backup_import: "Import",
        backup_exported: "Backup exported!",
        backup_imported: "Backup restored!",
        backup_error: "Invalid file",
        overlay_shortcut: "Overlay Shortcut",
        overlay_enabled: "Overlay Active (Ctrl + H)",
        overlay_disabled: "Overlay Disabled",
        persistent_hud: "Persistent HUD",
        persistent_hud_on: "Slots always visible in-game",
        persistent_hud_off: "Hide slots on close",
        theme: "Theme",
        theme_system: "System",
        theme_dark: "Dark",
        theme_light: "Light",
        toast_done: "Done",
        toast_error: "Error",
    },
    overlay: Overlay {
        warning_title: "Warning",
        fullscreen_warning: "The game is in \"Fullscreen\" mode: Windows minimizes it whenever \
                             the overlay appears. Switch the game video mode to \"Borderless \
                             Fullscreen\" (same look and performance).",
    },
    update: Update {
        title: "Update Available",
        body: "A new version has been downloaded and is ready to install. Would you like to \
               restart the program now to apply changes?",
        restart_now: "Restart Now",
        later: "Later",
    },
    tray: Tray {
        open: "Open Macro Helldivers 2",
        exit: "Exit",
    },
    debug: DebugText {
        title: "Diagnostics",
        desc: "For tracking down stratagems that fail. When on, the app records the real timing \
               of every key, what you were holding when the call fired and a snapshot of the PC. \
               Only the keys the macro itself sends are recorded; what you type outside it is not.",
        mode: "Debug Mode",
        mode_on: "Recording every call, with the key panel in game",
        mode_on_no_overlay: "Recording every call. Turn the overlay on to see the keys in game",
        mode_off: "Off: nothing is recorded",
        typing: "Typing test",
        export: "Export report",
        folder: "Open folder",
        exported: "Report exported!",
        export_error: "Report was not saved",
        stat_calls: "Calls",
        stat_rejected: "Rejected by Windows",
        stat_hold: "Key hold",
        stat_gap: "Gap between keys",
        stat_held: "While holding movement",
        stat_ignored: "Ignored with the game in front",
        stat_hook: "Keyboard hook",
        stat_window: "Front window",
        stat_last: "Last call",
        calls_value: "{runs} · {completed} ok · {aborted} aborted · {blocked} blocked · \
                      {unfocused} no focus",
        spread_value: "min {min} · avg {mean} · max {max} ms",
        hook_value: "{keys} keys · last one {age}s ago · slowest call {us} µs",
        hook_idle: "No keys since the mode was turned on",
        completed: "completed",
        aborted: "aborted",
        blocked: "blocked",
        unfocused: "no focus",
        typing_idle: "The test types 10 sequences into this window and checks every key that \
                      arrives. Don't touch the keyboard until it finishes.",
        typing_running: "Testing {done}/{total}... don't touch the keyboard",
        typing_passed: "Keyboard OK: {passed}/{runs} sequences arrived complete and in order",
        typing_failed: "Failed: {passed}/{runs} complete · {missing} keys lost · {extra} extra",
        typing_interrupted: "{count} run(s) interrupted: the window lost focus",
        typing_blocked: "{count} run(s) refused: a shortcut fired during the test",
        typing_holds: "Average key hold: {sent} ms sent, {received} ms received",
        hud_waiting: "Waiting for a call",
        hud_min_hold: "shortest {ms} ms",
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_resolve() {
        assert_eq!(tr(Language::Pt).tabs.settings, "Configurações");
        assert_eq!(tr(Language::En).tabs.settings, "Settings");
        assert_eq!(language_name(Language::Pt), "Português");
        assert_eq!(language_name(Language::En), "English");
    }

    #[test]
    fn speed_labels_cover_every_profile() {
        for language in Language::ALL {
            let text = &tr(language).settings;
            for speed in Speed::ALL {
                assert!(!text.speed(speed).is_empty(), "{language} {speed}");
                // O botão quebra o rótulo em nome e FPS; o aviso usa só o nome.
                assert_ne!(
                    text.speed_name(speed),
                    text.speed(speed),
                    "{language} {speed}"
                );
            }
        }
    }

    #[test]
    fn the_fps_cap_hint_carries_the_numbers() {
        let (title, body) = tr(Language::Pt).settings.fps_cap_hint(30, Speed::Low);
        assert_eq!(title, "Jogo limitado a 30 fps");
        assert!(body.ends_with("Use \"Baixo FPS\"."), "{body}");
        let (title, body) = tr(Language::En).settings.fps_cap_hint(45, Speed::Normal);
        assert_eq!(title, "Game capped at 45 fps");
        assert!(body.ends_with("Use \"Normal\"."), "{body}");
    }

    #[test]
    fn equipment_and_weight_labels_cover_every_value() {
        for language in Language::ALL {
            let text = &tr(language).build;
            for slot in EquipSlot::ALL {
                assert!(!text.equip_label(slot).is_empty(), "{language} {slot:?}");
            }
        }
        let pt = &tr(Language::Pt).build;
        assert_eq!(pt.weight("Light"), "Leve");
        assert_eq!(pt.weight("Heavy"), "Pesada");
        // Peso desconhecido cai em "Média", como o ternário da v1.
        assert_eq!(pt.weight("Medium"), "Média");
        assert_eq!(pt.weight("Exosuit"), "Média");
    }

    #[test]
    fn tag_labels_fall_back_to_others() {
        let text = &tr(Language::Pt).settings;
        let others = tr(Language::Pt).macros.others;
        assert_eq!(text.tag("Offensive", others), "Ofensivo");
        assert_eq!(text.tag("Supply", others), "Suprimento");
        assert_eq!(text.tag("Defensive", others), "Defensivo");
        assert_eq!(text.tag("Mecha", others), "Outros");
    }

    /// As frases longas são escritas com continuação de linha; um espaço perdido
    /// antes da barra invertida gruda duas palavras sem quebrar a compilação.
    #[test]
    fn wrapped_sentences_match_the_legacy_text() {
        assert_eq!(
            PT.build.hint,
            "Clique em Gerar Build para sortear um loadout completo: 4 estratagemas, armas, armadura, capacete, capa e booster. Use o cadeado para manter um item nos próximos sorteios."
        );
        assert_eq!(
            EN.build.balanced_on,
            "Guarantees 1 support weapon and 1 backpack item, no doubles (support with backpack counts as both)"
        );
        assert_eq!(
            PT.overlay.fullscreen_warning,
            "O jogo está em \"Tela Cheia\": nesse modo o Windows minimiza o jogo quando o overlay aparece. Mude o vídeo do jogo para \"Tela Cheia sem Borda\" (mesmo visual e desempenho)."
        );
        assert_eq!(
            EN.settings.macro_speed_desc,
            "The game reads the keyboard once per frame, so each profile holds the key for at least one frame at the listed FPS. If the game drops inputs, pick a lower-FPS profile. Game capped at 30 fps, or dipping below that in combat: Low FPS. Below 30 all the time: Potato"
        );
    }

    /// Nenhuma string pode ter ficado vazia no porte: vazio na UI passa batido.
    #[test]
    fn no_string_is_empty() {
        for language in Language::ALL {
            let t = tr(language);
            let strings = [
                t.tabs.macro_tab,
                t.tabs.build,
                t.tabs.settings,
                t.macros.select_title,
                t.macros.others,
                t.macros.listening,
                t.macros.bind,
                t.macros.clear_slot,
                t.macros.search_placeholder,
                t.macros.search_no_results,
                t.build.hint,
                t.build.custom_hint,
                t.build.saved_hint,
                t.build.sub_saved,
                t.build.reroll,
                t.build.current,
                t.build.status_new,
                t.build.status_replace,
                t.build.status_taken,
                t.build.status_empty,
                t.build.save_changes,
                t.build.save_as_new,
                t.build.confirm_delete,
                t.build.notice_applied_current,
                t.build.equip_none,
                t.build.meta_credit,
                t.settings.keybinding,
                t.settings.macro_speed_desc,
                t.settings.fps_cap_title,
                t.settings.fps_cap_hint,
                t.settings.backup_desc,
                t.settings.overlay_shortcut,
                t.settings.persistent_hud_off,
                t.settings.theme,
                t.settings.theme_system,
                t.settings.theme_dark,
                t.settings.theme_light,
                t.settings.toast_done,
                t.settings.toast_error,
                t.macros.empty,
                t.macros.nothing_here,
                t.overlay.warning_title,
                t.overlay.fullscreen_warning,
                t.update.title,
                t.update.body,
                t.update.restart_now,
                t.update.later,
                t.tray.open,
                t.tray.exit,
            ];
            assert!(strings.iter().all(|s| !s.trim().is_empty()), "{language}");

            let d = &t.debug;
            let debug = [
                d.title,
                d.desc,
                d.mode,
                d.mode_on,
                d.mode_on_no_overlay,
                d.mode_off,
                d.typing,
                d.export,
                d.folder,
                d.exported,
                d.export_error,
                d.stat_calls,
                d.stat_rejected,
                d.stat_hold,
                d.stat_gap,
                d.stat_held,
                d.stat_ignored,
                d.stat_hook,
                d.stat_window,
                d.stat_last,
                d.calls_value,
                d.spread_value,
                d.hook_value,
                d.hook_idle,
                d.completed,
                d.aborted,
                d.blocked,
                d.unfocused,
                d.typing_idle,
                d.typing_running,
                d.typing_passed,
                d.typing_failed,
                d.typing_interrupted,
                d.typing_blocked,
                d.typing_holds,
                d.hud_waiting,
                d.hud_min_hold,
            ];
            assert!(debug.iter().all(|s| !s.trim().is_empty()), "{language}");
        }
    }

    #[test]
    fn debug_templates_are_filled_without_leftover_keys() {
        use crate::diag::typing::Summary;
        use crate::diag::{HookHealth, Spread, Stats};

        for language in Language::ALL {
            let d = &tr(language).debug;
            let mut spread = Spread::default();
            spread.add(45.4);
            spread.add(54.6);
            let texts = [
                d.calls(&Stats {
                    runs: 7,
                    completed: 5,
                    aborted: 1,
                    blocked: 1,
                    ..Stats::default()
                }),
                d.spread(&spread),
                d.hook(&HookHealth {
                    keys: 120,
                    last_key_age_s: Some(2.4),
                    max_callback_us: 85,
                }),
                d.typing_running(3, 10),
                d.hud_min_hold(47.6),
            ];
            for text in texts.iter().chain(&d.typing_result(&Summary::default()).1) {
                assert!(!text.contains('{'), "{language}: {text}");
            }
            assert!(d.spread(&spread).contains("45"), "{language}");
            assert_eq!(d.spread(&Spread::default()), "\u{2014}");
            assert_eq!(d.hook(&HookHealth::default()), d.hook_idle);
        }

        let (passed, lines) = tr(Language::Pt).debug.typing_result(&Summary {
            runs: 10,
            passed: 8,
            missing_keys: 3,
            extra_keys: 0,
            interrupted: 1,
            ..Summary::default()
        });
        assert!(!passed);
        assert_eq!(
            lines[0],
            "Falhou: 8/10 completas · 3 teclas perdidas · 0 a mais"
        );
        assert_eq!(lines.len(), 2, "a linha da interrupção vem junto");
    }
}
