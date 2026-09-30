//! Persistance des réglages Synchro Boîte à histoires : dossier audio mémorisé, noms des boîtes,
//! bookmarks security-scoped (accès sandbox).

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
    #[serde(default)]
    pub devices: HashMap<String, DeviceInfo>,
    /// Dernier dossier audio sélectionné (affichage). Sans `audio_folder_bookmark`, il n'est pas relu.
    #[serde(default)]
    pub last_audio_folder: Option<String>,
    /// Thème UI : "auto" | "light" | "dark"
    #[serde(default = "default_theme")]
    pub theme: String,
    /// device_id (`serial-<numéro de série>`) → bookmark security-scoped de la boîte, en hex.
    #[serde(default)]
    pub device_bookmarks: HashMap<String, String>,
    /// Bookmark security-scoped du dossier audio, en hex.
    #[serde(default)]
    pub audio_folder_bookmark: Option<String>,
}

fn default_theme() -> String { "auto".to_string() }

impl AppSettings {
    pub fn set_device_bookmark(&mut self, device_id: &str, bookmark: &[u8]) {
        self.device_bookmarks.insert(device_id.to_string(), hex::encode(bookmark));
    }

    /// Bookmarks des boîtes décodés ; une entrée corrompue est ignorée.
    pub fn device_bookmarks(&self) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<_> = self
            .device_bookmarks
            .iter()
            .filter_map(|(id, h)| hex::decode(h).ok().map(|b| (id.clone(), b)))
            .collect();
        out.sort();
        out
    }

    pub fn set_audio_folder(&mut self, folder: &str, bookmark: Option<&[u8]>) {
        self.last_audio_folder = Some(folder.to_string());
        self.audio_folder_bookmark = bookmark.map(hex::encode);
    }

    pub fn audio_folder_bookmark(&self) -> Option<Vec<u8>> {
        self.audio_folder_bookmark.as_deref().and_then(|h| hex::decode(h).ok())
    }
}

fn parse(json: &str) -> AppSettings {
    serde_json::from_str(json).unwrap_or_default()
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
    fs::read_to_string(&path).map(|s| parse(&s)).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::{parse, AppSettings};

    #[test]
    fn bookmarks_roundtrip_through_json() {
        let fake_device = [0x62u8, 0x6f, 0x6f, 0x6b, 0x00, 0xff, 0x10];
        let fake_audio = [1u8, 2, 3, 4, 250];
        let mut settings = AppSettings { theme: "dark".into(), ..Default::default() };
        settings.set_device_bookmark("serial-0123456789", &fake_device);
        settings.set_device_bookmark("serial-9999", &[7u8; 3]);
        settings.set_audio_folder("/Users/x/Music/Histoires", Some(&fake_audio));

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("\"deviceBookmarks\""));
        assert!(json.contains("\"audioFolderBookmark\": \"01020304fa\""));

        let back = parse(&json);
        assert_eq!(
            back.device_bookmarks(),
            vec![
                ("serial-0123456789".to_string(), fake_device.to_vec()),
                ("serial-9999".to_string(), vec![7u8; 3]),
            ]
        );
        assert_eq!(back.audio_folder_bookmark(), Some(fake_audio.to_vec()));
        assert_eq!(back.last_audio_folder.as_deref(), Some("/Users/x/Music/Histoires"));
        assert_eq!(back.theme, "dark");
    }

    #[test]
    fn settings_without_bookmarks_still_load() {
        // Réglages écrits par une version antérieure : aucun champ bookmark.
        let old = r#"{"devices":{"serial-1":{"name":"Chambre"}},"lastAudioFolder":"/Users/x/Music","theme":"light"}"#;
        let s = parse(old);
        assert_eq!(s.devices["serial-1"].name, "Chambre");
        assert_eq!(s.theme, "light");
        assert!(s.device_bookmarks().is_empty());
        // Le dossier reste affichable, mais sans bookmark il n'est pas relu.
        assert_eq!(s.last_audio_folder.as_deref(), Some("/Users/x/Music"));
        assert_eq!(s.audio_folder_bookmark(), None);
    }

    #[test]
    fn corrupted_bookmark_is_ignored() {
        let json = r#"{"deviceBookmarks":{"serial-1":"zz-pas-hex","serial-2":"0a0b"},"audioFolderBookmark":"xyz"}"#;
        let s = parse(json);
        assert_eq!(s.device_bookmarks(), vec![("serial-2".to_string(), vec![0x0a, 0x0b])]);
        assert_eq!(s.audio_folder_bookmark(), None);
        assert_eq!(s.theme, "auto");
    }

    #[test]
    fn clearing_audio_bookmark_on_folder_change() {
        let mut s = AppSettings::default();
        s.set_audio_folder("/a", Some(&[1, 2]));
        s.set_audio_folder("/b", None);
        assert_eq!(s.last_audio_folder.as_deref(), Some("/b"));
        assert_eq!(s.audio_folder_bookmark(), None);
    }
}
