//! Persistance des réglages LuniiSync : dossier audio mémorisé, noms des boîtes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// device_id (UUID volume) → nom personnalisé
    pub devices: HashMap<String, DeviceInfo>,
    /// Dernier dossier audio sélectionné
    pub last_audio_folder: Option<String>,
    /// Thème UI : "auto" | "light" | "dark"
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String { "auto".to_string() }

fn settings_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("settings.json"))
}

pub fn load(app: &tauri::AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Some(p) => p,
        None => return AppSettings::default(),
    };
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &tauri::AppHandle, settings: &AppSettings) -> io::Result<()> {
    let path = settings_path(app).ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Impossible de trouver app data dir")
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, json)
}
