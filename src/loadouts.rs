//! Persistência dos slots de macro, das builds salvas e do backup (R9).
//!
//! O arquivo de slots guarda ids, e não o estratagema inteiro como a v1 gravava
//! no `localStorage`: assim uma mudança de nome, tag ou codex no
//! `stratagems.json` não deixa o slot com dados velhos, e um estratagema que
//! saiu do jogo vira slot vazio na leitura em vez de quebrar o arquivo.
//!
//! O backup, ao contrário, é byte a byte o formato da v1 — é o único caminho de
//! migração dos slots e das builds antigas, que moravam no `localStorage` do
//! Electron e não têm como ser lidos daqui.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::data::{self, GameData, Stratagem};
use crate::settings::{Settings, SLOT_COUNT};
use crate::shared::Slots;
use crate::util;

/// Arquivo com os quatro slots, em `config_dir`.
pub const SLOTS_FILE: &str = "slots.json";
/// Arquivo com as builds salvas, em `config_dir`.
pub const LOADOUTS_FILE: &str = "loadouts.json";

/// Lê os slots do disco. Sem arquivo — ou com um arquivo ilegível — o app abre
/// com os quatro vazios, que é o mesmo estado de uma instalação nova.
pub fn load_slots(data: &GameData) -> Slots {
    let path = util::config_path(SLOTS_FILE);
    if !path.exists() {
        return Slots::default();
    }
    match load_from(&path, data) {
        Ok(slots) => slots,
        Err(err) => {
            log::warn!("slots.json ilegível ({err:#}); começando com os slots vazios");
            // Afastado, o arquivo não é sobrescrito pelo próximo save de slots
            // e continua recuperável à mão.
            util::quarantine(&path);
            Slots::default()
        }
    }
}

pub fn load_from(path: &Path, data: &GameData) -> Result<Slots> {
    let bytes = std::fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
    let ids: Vec<Option<u32>> = serde_json::from_slice(&bytes).context("slots.json inválido")?;
    Ok(sanitize(&ids, data))
}

/// Grava os slots. Falha de escrita só custa a posição dos slots na próxima
/// abertura, então ela é registrada e o app segue.
pub fn save_slots(slots: &Slots) {
    if let Err(err) = save_to(&util::config_path(SLOTS_FILE), slots) {
        log::warn!("slots não foram salvos: {err:#}");
    }
}

pub fn save_to(path: &Path, slots: &Slots) -> Result<()> {
    let json = serde_json::to_vec(slots).context("falha ao serializar os slots")?;
    util::write_atomic(path, &json)
}

/// Resolve os ids contra os dados atuais e limpa o que não pode coexistir.
///
/// Porte de `legacy/src/renderer/App.jsx` (~82–98): cada slot é conferido
/// contra os anteriores, e o segundo exo (ou veículo) de uma dupla cai fora —
/// um save antigo pode ter nascido antes de a regra de exclusividade existir.
pub fn sanitize(ids: &[Option<u32>], data: &GameData) -> Slots {
    let mut slots = Slots::default();
    let mut kept: Vec<Option<&Stratagem>> = Vec::with_capacity(SLOT_COUNT);

    for (index, slot) in slots.iter_mut().enumerate() {
        // Uma lista curta demais (ou longa) não invalida o arquivo: o que falta
        // vira vazio e o que sobra é ignorado.
        let strat = ids
            .get(index)
            .copied()
            .flatten()
            .and_then(|id| data.by_id(id))
            .filter(|strat| !data::has_exclusive_conflict(strat, &kept, index));
        *slot = strat.map(|strat| strat.id);
        kept.push(strat);
    }
    slots
}

/// Uma build salva, no mesmo formato que a v1 gravava (a aba de Builds a
/// consome na Fase 7; o backup já precisa dela agora).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub id: String,
    pub name: String,
    /// Ids dos quatro slots, com `null` onde a build não define nada.
    #[serde(default)]
    pub slot_ids: Vec<Option<u32>>,
    /// Equipamento por categoria (`primary`, `armor`, ...). Builds salvas antes
    /// de o equipamento existir simplesmente não têm o campo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equip: Option<HashMap<String, String>>,
}

/// Lê as builds salvas. Arquivo ausente ou ilegível vira lista vazia — nenhuma
/// build é melhor que o app não abrir.
pub fn load_loadouts() -> Vec<Loadout> {
    let path = util::config_path(LOADOUTS_FILE);
    if !path.exists() {
        return Vec::new();
    }
    match load_loadouts_from(&path) {
        Ok(loadouts) => loadouts,
        Err(err) => {
            log::warn!("loadouts.json ilegível ({err:#}); começando sem builds salvas");
            // Sem afastar o arquivo, o primeiro "salvar build" da sessão
            // publicaria a lista vazia por cima de todas as builds do usuário —
            // que podem estar intactas (leitura bloqueada por antivírus).
            util::quarantine(&path);
            Vec::new()
        }
    }
}

pub fn load_loadouts_from(path: &Path) -> Result<Vec<Loadout>> {
    let bytes = std::fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
    serde_json::from_slice(&bytes).context("loadouts.json inválido")
}

pub fn save_loadouts(loadouts: &[Loadout]) {
    if let Err(err) = save_loadouts_to(&util::config_path(LOADOUTS_FILE), loadouts) {
        log::warn!("builds salvas não foram gravadas: {err:#}");
    }
}

pub fn save_loadouts_to(path: &Path, loadouts: &[Loadout]) -> Result<()> {
    let json = serde_json::to_vec(loadouts).context("falha ao serializar as builds")?;
    util::write_atomic(path, &json)
}

// --- Backup ---

/// Marca do app no arquivo de backup. O import recusa qualquer outro valor,
/// como a v1 fazia.
pub const BACKUP_APP: &str = "macro-helldivers2";
/// Nome sugerido no diálogo de exportação — o mesmo da v1.
pub const BACKUP_FILE_NAME: &str = "macro-helldivers2-backup.json";

/// Arquivo de backup, **exatamente** no formato da v1 (R9). Os campos são
/// opcionais na leitura porque um arquivo antigo pode não ter todos, e a v1
/// também importava o que encontrasse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub exported_at: String,
    /// Guardado cru: o merge com os settings atuais precisa saber quais chaves
    /// o arquivo realmente traz, e um `Settings` já teria preenchido o resto
    /// com os padrões.
    #[serde(default)]
    pub settings: serde_json::Value,
    /// `None` quando o arquivo não traz a lista — e aí a importação não mexe
    /// no que já existe, como a v1 fazia com o seu `Array.isArray`.
    #[serde(default)]
    pub loadouts: Option<Vec<Loadout>>,
    #[serde(default)]
    pub slot_ids: Option<Vec<Option<u32>>>,
}

impl Backup {
    pub fn new(settings: &Settings, loadouts: &[Loadout], slots: Slots) -> Result<Backup> {
        Ok(Backup {
            app: BACKUP_APP.to_string(),
            exported_at: util::iso8601_now(),
            settings: serde_json::to_value(settings).context("falha ao serializar settings")?,
            loadouts: Some(loadouts.to_vec()),
            slot_ids: Some(slots.to_vec()),
        })
    }

    /// Grava indentado com dois espaços, como o `JSON.stringify(data, null, 2)`
    /// da v1 — o arquivo continua legível a olho nu.
    pub fn write(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).context("falha ao serializar o backup")?;
        util::write_atomic(path, &json)
    }

    pub fn read(path: &Path) -> Result<Backup> {
        let bytes =
            std::fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
        let backup: Backup = serde_json::from_slice(&bytes).context("backup inválido")?;
        anyhow::ensure!(
            backup.app == BACKUP_APP,
            "o arquivo não é um backup do Macro Helldivers 2"
        );
        Ok(backup)
    }

    /// Settings do arquivo por cima dos atuais — o `{ ...settings, ...d.settings }`
    /// da v1. O que o backup não traz continua valendo, e chaves que não existem
    /// mais (`sprintModifier`) são ignoradas na desserialização.
    pub fn merged_settings(&self, current: &Settings) -> Result<Settings> {
        let Some(incoming) = self.settings.as_object() else {
            return Ok(current.clone());
        };
        let mut merged =
            serde_json::to_value(current).context("falha ao serializar settings atuais")?;
        let base = merged
            .as_object_mut()
            .context("settings atuais não são um objeto")?;
        for (key, value) in incoming {
            base.insert(key.clone(), value.clone());
        }
        serde_json::from_value(merged).context("settings do backup inválidos")
    }

    /// Os quatro slots do arquivo, resolvidos contra os dados atuais e sem os
    /// conflitos de exclusividade que um save antigo possa carregar.
    pub fn slots(&self, data: &GameData) -> Option<Slots> {
        self.slot_ids.as_ref().map(|ids| sanitize(ids, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Language, Speed};

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
    }

    /// Backup no formato exato que a v1 exportava, incluindo o `sprintModifier`
    /// que a v2 não conhece mais.
    fn v1_fixture() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/backup-v1.json")
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "mh2-loadouts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique).join(name)
    }

    #[test]
    fn ids_are_resolved_against_the_current_data() {
        let data = data();
        let known = data.all()[3].id;
        let slots = sanitize(&[Some(known), Some(9_999), None, Some(known)], &data);

        assert_eq!(slots[0], Some(known));
        assert_eq!(slots[1], None, "id que sumiu do jogo vira slot vazio");
        assert_eq!(slots[2], None);
        // Duplicata não é conflito de exclusividade: a v1 também a mantinha.
        assert_eq!(slots[3], Some(known));
    }

    #[test]
    fn a_short_or_long_list_is_normalized_to_four_slots() {
        let data = data();
        let id = data.all()[0].id;

        let short = sanitize(&[Some(id)], &data);
        assert_eq!(short, [Some(id), None, None, None]);

        let long = sanitize(&[None, None, None, None, Some(id)], &data);
        assert_eq!(long, [None; SLOT_COUNT], "o quinto slot é descartado");
    }

    #[test]
    fn an_inherited_exclusivity_conflict_is_dropped_on_load() {
        let data = data();
        let mechas: Vec<u32> = data
            .all()
            .iter()
            .filter(|strat| strat.has_tag("Mecha"))
            .map(|strat| strat.id)
            .collect();
        assert!(mechas.len() >= 2);

        let slots = sanitize(&[Some(mechas[0]), Some(mechas[1]), None, None], &data);
        assert_eq!(slots[0], Some(mechas[0]), "o primeiro fica");
        assert_eq!(slots[1], None, "o segundo exo cai");
    }

    #[test]
    fn slots_round_trip_through_disk() {
        let data = data();
        let path = temp_path(SLOTS_FILE);
        let id = data.all()[10].id;
        let slots: Slots = [Some(id), None, None, Some(data.all()[11].id)];

        save_to(&path, &slots).unwrap();
        assert_eq!(load_from(&path, &data).unwrap(), slots);
        // O formato é a lista de ids da R9, não o objeto inteiro da v1.
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.starts_with('['), "{raw}");
        assert!(!raw.contains("nome"), "{raw}");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_broken_file_is_an_error_instead_of_a_panic() {
        let data = data();
        let path = temp_path(SLOTS_FILE);
        util::write_atomic(&path, b"{ nao sou uma lista }").unwrap();

        assert!(load_from(&path, &data).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn saved_builds_round_trip_through_disk() {
        let path = temp_path(LOADOUTS_FILE);
        let loadouts = vec![
            Loadout {
                id: "1741977727311".into(),
                name: "Bug Sweep".into(),
                slot_ids: vec![Some(0), None, Some(3), None],
                equip: Some(HashMap::from([(
                    "primary".to_string(),
                    "ar-23".to_string(),
                )])),
            },
            // Build antiga, salva antes de o equipamento existir.
            Loadout {
                id: "1741977801044".into(),
                name: "Old Build".into(),
                slot_ids: vec![Some(1), Some(2), None, None],
                equip: None,
            },
        ];

        save_loadouts_to(&path, &loadouts).unwrap();
        assert_eq!(load_loadouts_from(&path).unwrap(), loadouts);

        // As chaves são as da v1: um backup exportado aqui continua importável lá.
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"slotIds\""), "{raw}");
        assert!(
            !raw.contains("\"equip\":null"),
            "build sem equipamento não ganha o campo: {raw}"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_real_v1_backup_is_read_whole() {
        let backup = Backup::read(&v1_fixture()).unwrap();

        assert_eq!(backup.app, BACKUP_APP);
        assert_eq!(backup.exported_at, "2025-03-14T18:42:07.311Z");
        let loadouts = backup.loadouts.as_ref().expect("builds salvas");
        assert_eq!(loadouts.len(), 2);
        assert_eq!(loadouts[0].name, "Bug Sweep");
        assert_eq!(loadouts[0].slot_ids, [Some(0), Some(3), Some(66), Some(12)]);
        assert_eq!(
            loadouts[0]
                .equip
                .as_ref()
                .and_then(|equip| equip.get("armor"))
                .map(String::as_str),
            Some("b-01-tactical")
        );
        assert_eq!(loadouts[1].equip, None, "build antiga não tem equipamento");
    }

    #[test]
    fn importing_v1_settings_keeps_what_the_file_does_not_carry() {
        let backup = Backup::read(&v1_fixture()).unwrap();
        // Estado atual diferente do arquivo em tudo o que ele traz.
        let current = Settings {
            language: Language::Pt,
            macro_speed: Speed::Normal,
            modifier_key: "LeftControl".into(),
            build_max_one_sentry: true,
            ..Settings::default()
        };

        let merged = backup.merged_settings(&current).unwrap();

        assert_eq!(merged.language, Language::En);
        assert_eq!(merged.macro_speed, Speed::Turbo);
        assert_eq!(merged.modifier_key, "LeftAlt");
        assert!(merged.use_arrows);
        assert!(merged.always_show_slots);
        assert_eq!(merged.shortcut(2), Some("Numpad1"));
        assert_eq!(merged.support_shortcut(0), Some("F5"));
        assert_eq!(merged.support_shortcut(1), None);
        assert!(merged.build_balanced);
        assert!(!merged.build_max_one_sentry);
        // E o `sprintModifier` da v1 não sobrevive à importação.
        let round = serde_json::to_string(&merged).unwrap();
        assert!(!round.contains("sprintModifier"), "{round}");
    }

    #[test]
    fn a_partial_settings_object_only_overwrites_what_it_names() {
        // Backup mais velho que os campos de build, ou editado à mão.
        let backup = Backup {
            app: BACKUP_APP.into(),
            exported_at: String::new(),
            settings: serde_json::json!({ "language": "en" }),
            loadouts: None,
            slot_ids: None,
        };
        let current = Settings {
            macro_speed: Speed::Turbo,
            build_max_one_sentry: true,
            ..Settings::default()
        };

        let merged = backup.merged_settings(&current).unwrap();
        assert_eq!(merged.language, Language::En);
        assert_eq!(merged.macro_speed, Speed::Turbo);
        assert!(merged.build_max_one_sentry);

        // Sem objeto de settings, nada muda.
        let empty = Backup {
            settings: serde_json::Value::Null,
            ..backup
        };
        assert_eq!(empty.merged_settings(&current).unwrap(), current);
    }

    #[test]
    fn imported_slots_are_resolved_and_cleaned() {
        let data = data();
        let backup = Backup::read(&v1_fixture()).unwrap();

        // O arquivo traz dois exos seguidos (66 e 67); o segundo cai.
        assert_eq!(
            backup.slot_ids.as_deref(),
            Some([Some(0), Some(66), Some(67), Some(3)].as_slice())
        );
        assert_eq!(
            backup.slots(&data),
            Some([Some(0), Some(66), None, Some(3)])
        );

        // Backup sem a lista não mexe nos slots de quem importa.
        let partial = Backup {
            slot_ids: None,
            ..backup
        };
        assert_eq!(partial.slots(&data), None);
    }

    #[test]
    fn a_file_from_another_app_is_refused() {
        let path = temp_path("outro.json");
        util::write_atomic(&path, br#"{"app":"outro-app","settings":{}}"#).unwrap();
        assert!(Backup::read(&path).is_err());

        util::write_atomic(&path, b"nem json").unwrap();
        assert!(Backup::read(&path).is_err());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_exported_backup_reads_back_identical() {
        let data = data();
        let path = temp_path(BACKUP_FILE_NAME);
        let settings = Settings {
            language: Language::En,
            macro_speed: Speed::Fast,
            ..Settings::default()
        };
        let loadouts = vec![Loadout {
            id: "1".into(),
            name: "Meta".into(),
            slot_ids: vec![Some(data.all()[0].id), None, None, None],
            equip: None,
        }];
        let slots: Slots = [Some(0), Some(66), None, Some(3)];

        Backup::new(&settings, &loadouts, slots)
            .unwrap()
            .write(&path)
            .unwrap();
        let read = Backup::read(&path).unwrap();

        assert_eq!(
            read.merged_settings(&Settings::default()).unwrap(),
            settings
        );
        assert_eq!(read.loadouts, Some(loadouts));
        assert_eq!(read.slots(&data), Some(slots));
        assert!(read.exported_at.ends_with('Z'));

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
