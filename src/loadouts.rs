//! Persistência dos slots de macro (R9). As builds salvas entram aqui na Fase 7.
//!
//! O arquivo guarda ids, e não o estratagema inteiro como a v1 gravava no
//! `localStorage`: assim uma mudança de nome, tag ou codex no `stratagems.json`
//! não deixa o slot com dados velhos, e um estratagema que saiu do jogo vira
//! slot vazio na leitura em vez de quebrar o arquivo.

use std::path::Path;

use anyhow::{Context, Result};

use crate::data::{self, GameData, Stratagem};
use crate::settings::SLOT_COUNT;
use crate::shared::Slots;
use crate::util;

/// Arquivo com os quatro slots, em `config_dir`.
pub const SLOTS_FILE: &str = "slots.json";

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

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> GameData {
        GameData::load().expect("stratagems.json do repositório")
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
}
