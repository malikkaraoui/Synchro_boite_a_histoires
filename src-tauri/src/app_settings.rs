//! Persistance des réglages LuniiSync : dossier audio mémorisé, noms des boîtes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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

fn normalize_device_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn normalize(settings: &mut AppSettings) {
    settings.devices.retain(|_, info| {
        if let Some(normalized) = normalize_device_name(&info.name) {
            info.name = normalized;
            true
        } else {
            false
        }
    });

    if settings.theme.trim().is_empty() {
        settings.theme = default_theme();
    }
}

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
    let mut settings = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    normalize(&mut settings);
    settings
}

pub fn save(app: &tauri::AppHandle, settings: &AppSettings) -> io::Result<()> {
    let path = settings_path(app).ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Impossible de trouver app data dir")
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut normalized = settings.clone();
    normalize(&mut normalized);
    let json = serde_json::to_string_pretty(&normalized)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, json)
}

pub fn delete_device(app: &tauri::AppHandle, device_id: &str) -> io::Result<()> {
    let mut settings = load(app);
    settings.devices.remove(device_id);
    save(app, &settings)
}

pub fn purge_legacy_devices(app: &tauri::AppHandle) -> io::Result<usize> {
    let mut settings = load(app);
    let before = settings.devices.len();
    settings.devices.retain(|id, _| id.starts_with("serial-"));
    let removed = before.saturating_sub(settings.devices.len());
    if removed > 0 {
        save(app, &settings)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::{normalize, AppSettings, DeviceInfo};
    use std::collections::HashMap;

    #[test]
    fn normalize_removes_blank_device_names_and_trims_values() {
        let mut devices = HashMap::new();
        devices.insert(
            "serial-abc".to_string(),
            DeviceInfo {
                name: "  Mia  ".to_string(),
            },
        );
        devices.insert(
            "uuid-old".to_string(),
            DeviceInfo {
                name: "   ".to_string(),
            },
        );

        let mut settings = AppSettings {
            devices,
            last_audio_folder: None,
            theme: " ".to_string(),
        };

        normalize(&mut settings);

        assert_eq!(settings.devices.len(), 1);
        assert_eq!(settings.devices["serial-abc"].name, "Mia");
        assert_eq!(settings.theme, "auto");
    }
}
