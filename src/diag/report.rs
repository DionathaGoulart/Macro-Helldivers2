//! Relatório do modo debug: um arquivo só, para o testador mandar e quem
//! investiga abrir.
//!
//! Junta o retrato atual do PC, o resumo da sessão, a saúde do hook, os eventos
//! gravados no `debug.jsonl` (e no `.old`, se ele já girou) e o fim do
//! `app.log`. Funciona com o modo desligado também: aí o relatório sai só com o
//! retrato e o que tiver sobrado de sessões anteriores.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use super::{HookHealth, Session, Stats, LOG_FILE, OLD_LOG_FILE};
use crate::data::GameData;
use crate::settings::Language;
use crate::shared::Shared;
use crate::util;

/// Marca do formato, para uma ferramenta de análise reconhecer o arquivo.
pub const FORMAT: &str = "macro-helldivers2-debug";
const VERSION: u32 = 1;

/// Teto de eventos no relatório: os mais recentes ficam.
const MAX_EVENTS: usize = 5_000;
/// Teto do `app.log` no relatório: o fim, que é onde está a sessão atual.
const MAX_APP_LOG: usize = 256 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    format: &'static str,
    version: u32,
    generated_at: String,
    debug_enabled: bool,
    session: Session,
    stats: Stats,
    hook: HookHealth,
    events: Vec<serde_json::Value>,
    app_log: String,
}

/// Nome sugerido no diálogo de salvar, com a data de hoje.
pub fn file_name(language: Language) -> String {
    let date = &util::iso8601_now()[..10];
    match language {
        Language::Pt => format!("relatorio-debug-{date}.json"),
        Language::En => format!("debug-report-{date}.json"),
    }
}

/// Monta e grava o relatório em `path`.
pub fn write(path: &Path, shared: &Shared, data: &GameData) -> Result<()> {
    let report = Report {
        format: FORMAT,
        version: VERSION,
        generated_at: util::iso8601_now(),
        debug_enabled: shared.diag.enabled(),
        session: super::session(shared, data),
        stats: shared.diag.stats(),
        hook: shared.diag.hook_health(),
        events: events(&[util::config_path(OLD_LOG_FILE), util::config_path(LOG_FILE)]),
        app_log: redact_user(&tail(&util::config_path("app.log"), MAX_APP_LOG)),
    };
    let json = serde_json::to_vec_pretty(&report).context("relatório não serializou")?;
    util::write_atomic(path, &json)
}

/// Os eventos dos arquivos, na ordem, sem as linhas que não são JSON (uma
/// linha pela metade, se a thread `diag` estava escrevendo agora).
fn events(paths: &[std::path::PathBuf]) -> Vec<serde_json::Value> {
    let mut events: Vec<serde_json::Value> = paths
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .flat_map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect::<Vec<_>>()
        })
        .collect();
    let excess = events.len().saturating_sub(MAX_EVENTS);
    events.drain(..excess);
    events
}

/// Tira o nome do usuário do Windows do texto: o `app.log` registra caminhos
/// da pasta de configuração, e o relatório sai do PC.
fn redact_user(text: &str) -> String {
    redact(
        text,
        std::env::var("USERPROFILE").ok().as_deref(),
        std::env::var("USERNAME").ok().as_deref(),
    )
}

fn redact(text: &str, profile: Option<&str>, user: Option<&str>) -> String {
    let mut text = text.to_string();
    if let Some(profile) = profile.filter(|profile| !profile.is_empty()) {
        text = text.replace(profile, "%USERPROFILE%");
    }
    // Um nome curto demais casaria com pedaço de palavra comum.
    if let Some(user) = user.filter(|user| user.len() >= 3) {
        text = text.replace(user, "%USERNAME%");
    }
    text
}

/// Os últimos `max` bytes de um arquivo de texto, cortados num início de linha.
fn tail(path: &Path, max: usize) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    let start = bytes.len().saturating_sub(max);
    let text = String::from_utf8_lossy(&bytes[start..]);
    match (start > 0, text.find('\n')) {
        (true, Some(newline)) => text[newline + 1..].to_string(),
        _ => text.into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mh2-report-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn events_come_in_order_and_skip_broken_lines() {
        let dir = temp_dir("events");
        let (old, current) = (dir.join("old.jsonl"), dir.join("current.jsonl"));
        fs::write(&old, "{\"n\":1}\n{\"n\":2}\n").unwrap();
        fs::write(&current, "{\"n\":3}\n{\"n\":4").unwrap();

        let events = events(&[old, current, dir.join("ausente.jsonl")]);
        let numbers: Vec<_> = events.iter().map(|event| event["n"].as_i64()).collect();
        assert_eq!(
            numbers,
            [Some(1), Some(2), Some(3)],
            "a linha pela metade sai"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_tail_starts_at_a_line_boundary() {
        let dir = temp_dir("tail");
        let path = dir.join("app.log");
        fs::write(&path, "primeira linha\nsegunda\nterceira\n").unwrap();

        assert_eq!(tail(&path, 1_000), "primeira linha\nsegunda\nterceira\n");
        // Corte no meio de "segunda": a linha incompleta sai inteira.
        assert_eq!(tail(&path, 12), "terceira\n");
        assert_eq!(tail(&dir.join("nada.log"), 100), "");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_windows_user_does_not_leave_the_pc() {
        let log = "config em C:\\Users\\Fulano\\AppData\\Roaming\\Macro Helldivers 2\nFulano ok";
        let clean = redact(log, Some("C:\\Users\\Fulano"), Some("Fulano"));
        assert!(!clean.contains("Fulano"), "{clean}");
        assert!(clean.starts_with("config em %USERPROFILE%\\AppData"));
        // Nome de duas letras não apaga pedaços de palavra.
        assert_eq!(redact("Jogo ok", None, Some("ok")), "Jogo ok");
    }

    #[test]
    fn the_file_name_carries_the_date() {
        let name = file_name(Language::Pt);
        assert!(name.starts_with("relatorio-debug-20"), "{name}");
        assert!(name.ends_with(".json"));
        assert!(file_name(Language::En).starts_with("debug-report-"));
    }
}
