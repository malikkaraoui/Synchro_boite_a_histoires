//! Logique de synchronisation V2 : scan dossier audio, dédup SHA-256,
//! gestion des sidecars `.la-forge-a-histoires.json`, suppression des histoires orphelines.

use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::storybox_device::{InventoryStatus, StoryBoxInventoryResult};

/// Formats listés : ceux que l'import sait envoyer. MP3 seul tant que le WAV n'est pas converti
/// (prévu en M0005) — lister un format refusé à l'import ferait échouer la synchro.
const AUDIO_EXTENSIONS: &[&str] = &["mp3"];

/// Informations d'espace disque d'un volume monté.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

/// Retourne les infos d'espace disque via la syscall POSIX `statvfs` (sandbox-friendly).
#[cfg(unix)]
pub fn get_storage_info(mount: &str) -> Result<StorageInfo, String> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(mount).map_err(|_| "Chemin invalide".to_string())?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return Err(format!("statvfs échoué (errno {})", std::io::Error::last_os_error()));
    }
    let stat = unsafe { stat.assume_init() };
    let frsize = stat.f_frsize as u64;
    let total = stat.f_blocks as u64 * frsize;
    let free  = stat.f_bavail as u64 * frsize;
    let used  = total.saturating_sub(free);
    Ok(StorageInfo {
        total_bytes: total,
        used_bytes:  used,
        free_bytes:  free,
    })
}

#[cfg(not(unix))]
pub fn get_storage_info(_mount: &str) -> Result<StorageInfo, String> {
    Err("Espace disque non disponible sur cette plateforme".to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFile {
    /// Chemin absolu du fichier audio.
    pub path: String,
    /// Nom de fichier (avec extension).
    pub filename: String,
    /// story_id dérivé du nom de fichier sans extension.
    pub story_id: String,
    /// Hash SHA-256 du contenu audio ("sha256:<hex>").
    pub hash_sha256: String,
    /// Taille du fichier en octets.
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PushReason {
    /// Pas encore présent sur le device.
    New,
    /// Présent mais hash différent (fichier source modifié).
    Outdated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushTask {
    pub audio_file: AudioFile,
    pub reason: PushReason,
}

/// Plan de sync calculé avant toute action sur le device.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlan {
    pub device_mount: Option<String>,
    /// Fichiers à transférer (nouveaux + obsolètes).
    pub to_push: Vec<PushTask>,
    /// Nombre de fichiers déjà à jour sur le device.
    pub already_up_to_date: usize,
    /// short_uuids des histoires à supprimer (source effacée, sidecar présent).
    pub to_remove: Vec<String>,
    pub total_audio_files: usize,
    pub device_total_stories: usize,
}

// ── Scan dossier ─────────────────────────────────────────────────────────────

pub fn scan_audio_folder(folder_path: &str) -> Result<Vec<AudioFile>, String> {
    let dir = Path::new(folder_path);
    if !dir.is_dir() {
        return Err(format!("Dossier introuvable : {folder_path}"));
    }

    let mut files: Vec<AudioFile> = fs::read_dir(dir)
        .map_err(|e| format!("Lecture dossier échouée : {e}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let p = entry.path();
            if !p.is_file() || crate::storybox_device::is_macos_metadata(&entry.file_name().to_string_lossy()) {
                return None;
            }
            let ext = p.extension()?.to_str()?.to_lowercase();
            if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }
            let filename = p.file_name()?.to_string_lossy().into_owned();
            let story_id = p.file_stem()?.to_string_lossy().into_owned();
            let hash_sha256 = compute_file_hash(&p).ok()?;
            let size_bytes = p.metadata().map(|m| m.len()).unwrap_or(0);
            Some(AudioFile {
                path: p.to_string_lossy().into_owned(),
                filename,
                story_id,
                hash_sha256,
                size_bytes,
            })
        })
        .collect();

    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(files)
}

// ── Hash SHA-256 ──────────────────────────────────────────────────────────────

pub fn compute_file_hash(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Impossible d'ouvrir {:?}: {e}", path))?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("Erreur lecture : {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

// ── Plan de sync ──────────────────────────────────────────────────────────────

pub fn determine_needed_pushes(
    audio_files: &[AudioFile],
    inventory: &StoryBoxInventoryResult,
) -> SyncPlan {
    let device_mount = inventory.mount.clone();
    let device_total_stories = inventory.total_stories;
    let mut to_push = Vec::new();
    let mut already_up_to_date = 0usize;

    for af in audio_files {
        if inventory.status != InventoryStatus::Ok {
            // Device absent ou illisible → tout est à pusher
            to_push.push(PushTask {
                audio_file: af.clone(),
                reason: PushReason::New,
            });
            continue;
        }

        let device_entry = inventory
            .stories
            .iter()
            .find(|s| s.sidecar.as_ref().is_some_and(|sc| sc.story_id == af.story_id));

        match device_entry {
            None => to_push.push(PushTask {
                audio_file: af.clone(),
                reason: PushReason::New,
            }),
            Some(entry) => {
                let device_hash = entry.sidecar.as_ref().map(|sc| &sc.hash);
                if device_hash.is_some_and(|h| h == &af.hash_sha256) {
                    already_up_to_date += 1;
                } else {
                    to_push.push(PushTask {
                        audio_file: af.clone(),
                        reason: PushReason::Outdated,
                    });
                }
            }
        }
    }

    // Orphelins : stories avec sidecar dont la source audio a été supprimée
    let source_ids: Vec<&str> = audio_files.iter().map(|af| af.story_id.as_str()).collect();

    let to_remove: Vec<String> = if inventory.status == InventoryStatus::Ok {
        inventory
            .stories
            .iter()
            .filter(|s| {
                s.sidecar
                    .as_ref()
                    .is_some_and(|sc| !source_ids.contains(&sc.story_id.as_str()))
            })
            .map(|s| s.short_uuid.clone())
            .collect()
    } else {
        vec![]
    };

    SyncPlan {
        device_mount,
        to_push,
        already_up_to_date,
        to_remove,
        total_audio_files: audio_files.len(),
        device_total_stories,
    }
}

// ── Sidecar ───────────────────────────────────────────────────────────────────

/// Écrit `.la-forge-a-histoires.json` dans le dossier story après un import réussi.
pub fn write_sidecar(
    mount: &str,
    short_uuid: &str,
    story_id: &str,
    hash: &str,
) -> Result<(), String> {
    write_sidecar_in(&Path::new(mount).join(".content").join(short_uuid), story_id, hash)
}

/// Écrit le sidecar dans un dossier story donné (y compris le dossier de transit d'un import).
pub fn write_sidecar_in(story_dir: &Path, story_id: &str, hash: &str) -> Result<(), String> {
    if !story_dir.is_dir() {
        return Err(format!("Dossier story introuvable : {story_dir:?}"));
    }

    let payload = serde_json::json!({
        "story_id": story_id,
        "hash": hash,
        "pushed_at": Utc::now().to_rfc3339(),
        "source": "la-forge-a-histoires"
    });

    fs::write(
        story_dir.join(".la-forge-a-histoires.json"),
        serde_json::to_string_pretty(&payload).unwrap(),
    )
    .map_err(|e| format!("Écriture sidecar échouée : {e}"))
}

// ── Suppression orphelins ─────────────────────────────────────────────────────

/// Supprime un dossier story orphelin depuis `.content/<short_uuid>/`.
pub fn remove_orphan_story(mount: &str, short_uuid: &str) -> Result<(), String> {
    let story_dir = Path::new(mount).join(".content").join(short_uuid);
    if story_dir.is_dir() {
        fs::remove_dir_all(&story_dir)
            .map_err(|e| format!("Suppression {story_dir:?} échouée : {e}"))?;
    }
    Ok(())
}

// ── Recherche short_uuid après import ────────────────────────────────────────

#[allow(dead_code)]
/// Trouve le short_uuid d'un story_id dans `.content/` après que Python l'a importé.
/// Cherche le dossier créé le plus récemment qui n'a pas encore de sidecar.
pub fn find_newly_pushed_uuid(mount: &str, story_id: &str) -> Option<PathBuf> {
    let content_dir = Path::new(mount).join(".content");
    if !content_dir.is_dir() {
        return None;
    }

    // Chercher d'abord un sidecar correspondant (si Python a déjà écrit le sidecar)
    if let Ok(entries) = fs::read_dir(&content_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let sidecar_path = path.join(".la-forge-a-histoires.json");
            if let Ok(text) = fs::read_to_string(&sidecar_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if val.get("story_id").and_then(|v| v.as_str()) == Some(story_id) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn scan_returns_error_for_missing_dir() {
        let result = scan_audio_folder("/chemin/qui/nexiste/pas");
        assert!(result.is_err());
    }

    #[test]
    fn scan_filters_non_audio_files() {
        let tmp = TempDir::new("storybox-sync-scan");
        fs::write(tmp.path.join("story.mp3"), b"fake mp3").unwrap();
        fs::write(tmp.path.join("image.jpg"), b"fake jpg").unwrap();
        fs::write(tmp.path.join("README.txt"), b"readme").unwrap();

        let files = scan_audio_folder(&tmp.path.to_string_lossy()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "story.mp3");
        assert_eq!(files[0].story_id, "story");
        assert!(files[0].hash_sha256.starts_with("sha256:"));
    }

    #[test]
    fn scan_lists_only_importable_mp3() {
        let tmp = TempDir::new("storybox-sync-ext");
        for (i, ext) in ["mp3", "MP3", "m4a", "wav", "ogg", "flac"].iter().enumerate() {
            fs::write(tmp.path.join(format!("file{i}.{ext}")), b"data").unwrap();
        }
        fs::write(tmp.path.join("file.pdf"), b"data").unwrap();

        let files = scan_audio_folder(&tmp.path.to_string_lossy()).unwrap();
        let mut names: Vec<_> = files.iter().map(|f| f.filename.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["file0.mp3", "file1.MP3"]);
    }

    /// Sur une clé FAT/exFAT, macOS pose `._<nom>` à côté de chaque fichier : jamais importé.
    #[test]
    fn scan_ignores_appledouble_and_ds_store() {
        let tmp = TempDir::new("storybox-sync-appledouble");
        fs::write(tmp.path.join("story.mp3"), b"fake mp3").unwrap();
        fs::write(tmp.path.join("._story.mp3"), b"\x00\x05\x16\x07").unwrap();
        fs::write(tmp.path.join(".DS_Store"), b"x").unwrap();

        let files = scan_audio_folder(&tmp.path.to_string_lossy()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.filename.as_str()).collect();
        assert_eq!(names, vec!["story.mp3"]);
    }

    #[test]
    fn compute_hash_is_deterministic() {
        let tmp = TempDir::new("storybox-sync-hash");
        let path = tmp.path.join("test.mp3");
        fs::write(&path, b"hello world").unwrap();

        let h1 = compute_file_hash(&path).unwrap();
        let h2 = compute_file_hash(&path).unwrap();
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn write_sidecar_creates_json_file() {
        let tmp = TempDir::new("storybox-sidecar-write");
        let content = tmp.path.join(".content").join("ABCD1234");
        fs::create_dir_all(&content).unwrap();
        let mount = tmp.path.to_string_lossy().into_owned();

        write_sidecar(&mount, "ABCD1234", "mon-histoire", "sha256:abc123").unwrap();

        let sidecar_path = content.join(".la-forge-a-histoires.json");
        assert!(sidecar_path.exists());
        let text = fs::read_to_string(&sidecar_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(val["story_id"], "mon-histoire");
        assert_eq!(val["hash"], "sha256:abc123");
        assert_eq!(val["source"], "la-forge-a-histoires");
    }

    #[test]
    fn remove_orphan_story_deletes_dir() {
        let tmp = TempDir::new("storybox-remove-orphan");
        let content = tmp.path.join(".content").join("DEADBEEF");
        fs::create_dir_all(&content).unwrap();
        let mount = tmp.path.to_string_lossy().into_owned();

        remove_orphan_story(&mount, "DEADBEEF").unwrap();
        assert!(!content.exists());
    }
}
