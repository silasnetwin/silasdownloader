//! Persistenz für Einstellungen und Verlauf.

use std::{fs, path::PathBuf};

use crate::model::{HistoryEntry, Settings};

fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("SilasDownloader")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

fn history_path() -> PathBuf {
    data_dir().join("history.json")
}

/// Ordner eines früheren Programms, aus dem einmalig übernommen wird.
fn legacy_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("VideoDownloader")
}

/// Übernimmt Einstellungen und Verlauf aus einer früheren Installation.
fn adopt_legacy_data() {
    let legacy = legacy_dir();
    if legacy == data_dir() || !legacy.is_dir() {
        return;
    }
    for name in ["settings.json", "history.json"] {
        let source = legacy.join(name);
        if !source.is_file() {
            continue;
        }
        let target = data_dir().join(name);
        if target.exists() {
            continue;
        }
        if fs::create_dir_all(data_dir()).is_err() {
            return;
        }
        let _ = fs::copy(&source, &target);
    }
}

/// Lädt die Einstellungen.
pub fn load_settings() -> Settings {
    adopt_legacy_data();
    fs::read(settings_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Speichert die Einstellungen.
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(settings_path(), bytes).map_err(|e| e.to_string())
}

/// Lädt den Verlauf.
pub fn load_history() -> Vec<HistoryEntry> {
    adopt_legacy_data();
    fs::read(history_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Speichert den Verlauf.
pub fn save_history(history: &[HistoryEntry]) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(history).map_err(|e| e.to_string())?;
    fs::write(history_path(), bytes).map_err(|e| e.to_string())
}
