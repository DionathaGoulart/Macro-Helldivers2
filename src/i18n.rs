//! Traduções pt/en.
//!
//! Porte integral de `legacy/src/renderer/data/translations.js`, mais as frases
//! que a v1 deixou embutidas no JSX com ternário de idioma. Ficaram de fora as
//! chaves da intro de boot (a animação foi cortada: o app abre direto) e as do
//! modificador de corrida (removido junto com a feature).

use crate::data::EquipSlot;
use crate::meta_stats::Faction;
use crate::settings::{Language, Speed};

pub struct Tr {
    pub tabs: Tabs,
    pub macros: Macros,
    pub build: Build,
    pub settings: SettingsText,
    pub overlay: Overlay,
    pub update: Update,
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
}

pub struct Build {
    pub sub_meta: &'static str,
    pub sub_random: &'static str,
    pub sub_custom: &'static str,
    pub saved: &'static str,
    pub saved_empty: &'static str,
    pub save_placeholder: &'static str,
    pub save_build: &'static str,
    pub delete_build: &'static str,
    pub generate: &'static str,
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
    pub macro_speed_normal: &'static str,
    pub macro_speed_fast: &'static str,
    pub macro_speed_turbo: &'static str,
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
}

pub struct Overlay {
    pub fullscreen_warning: &'static str,
}

/// Modal de "atualização pronta" — também embutido no JSX da v1.
pub struct Update {
    pub title: &'static str,
    pub body: &'static str,
    pub restart_now: &'static str,
    pub later: &'static str,
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
            Speed::Normal => self.macro_speed_normal,
            Speed::Fast => self.macro_speed_fast,
            Speed::Turbo => self.macro_speed_turbo,
        }
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

/// Textos do idioma escolhido.
pub fn tr(language: Language) -> &'static Tr {
    match language {
        Language::Pt => &PT,
        Language::En => &EN,
    }
}

/// Nome do idioma na própria língua — não traduzido, como na v1.
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
    },
    build: Build {
        sub_meta: "Meta",
        sub_random: "Aleatória",
        sub_custom: "Personalizada",
        saved: "Builds Salvas",
        saved_empty: "Monte ou gere uma build e salve com um nome",
        save_placeholder: "Nome da build...",
        save_build: "Salvar",
        delete_build: "Excluir build",
        generate: "Gerar Build",
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
                      em Builds Salvas.",
        custom_import: "Usar slots atuais",
        custom_clear: "Limpar tudo",
        custom_equipment: "Equipamento (opcional)",
        equip_none: "— Nenhum —",
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
                           use um perfil de FPS menor",
        macro_speed_normal: "Padrão · 30 fps",
        macro_speed_fast: "Rápida · 60 fps",
        macro_speed_turbo: "Turbo · 60+ fps",
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
    },
    overlay: Overlay {
        fullscreen_warning: "O jogo está em \"Tela Cheia\" — nesse modo o Windows minimiza o \
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
    },
    build: Build {
        sub_meta: "Meta",
        sub_random: "Random",
        sub_custom: "Custom",
        saved: "Saved Builds",
        saved_empty: "Build or generate a loadout and save it with a name",
        save_placeholder: "Build name...",
        save_build: "Save",
        delete_build: "Delete build",
        generate: "Generate Build",
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
                      gear if you want. Then name it and save it under Saved Builds.",
        custom_import: "Use current slots",
        custom_clear: "Clear all",
        custom_equipment: "Equipment (optional)",
        equip_none: "— None —",
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
                           inputs, pick a lower-FPS profile",
        macro_speed_normal: "Normal · 30 fps",
        macro_speed_fast: "Fast · 60 fps",
        macro_speed_turbo: "Turbo · 60+ fps",
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
    },
    overlay: Overlay {
        fullscreen_warning: "The game is in \"Fullscreen\" mode — Windows minimizes it whenever \
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
            }
        }
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
            "O jogo está em \"Tela Cheia\" — nesse modo o Windows minimiza o jogo quando o overlay aparece. Mude o vídeo do jogo para \"Tela Cheia sem Borda\" (mesmo visual e desempenho)."
        );
        assert_eq!(
            EN.settings.macro_speed_desc,
            "The game reads the keyboard once per frame, so each profile holds the key for at least one frame at the listed FPS. If the game drops inputs, pick a lower-FPS profile"
        );
    }

    /// Nenhuma string pode ter ficado vazia no porte — vazio na UI passa batido.
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
                t.build.equip_none,
                t.build.meta_credit,
                t.settings.keybinding,
                t.settings.macro_speed_desc,
                t.settings.backup_desc,
                t.settings.overlay_shortcut,
                t.settings.persistent_hud_off,
                t.overlay.fullscreen_warning,
                t.update.title,
                t.update.body,
                t.update.restart_now,
                t.update.later,
            ];
            assert!(strings.iter().all(|s| !s.trim().is_empty()), "{language}");
        }
    }
}
