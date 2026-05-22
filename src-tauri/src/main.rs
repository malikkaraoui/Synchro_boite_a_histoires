#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_settings;
mod lunii_device;
mod lunii_sync;

use lunii_device::{LuniiDeviceInfo, LuniiDeviceProbe, LuniiInventoryResult, StoryCompareResult};
use lunii_sync::{AudioFile, StorageInfo, SyncPlan};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

// ── Commandes device ──────────────────────────────────────────────────────────

#[tauri::command]
fn probe_lunii_device() -> LuniiDeviceProbe {
    lunii_device::probe_lunii_device()
}

#[tauri::command]
fn get_lunii_inventory() -> LuniiInventoryResult {
    lunii_device::get_lunii_inventory()
}

#[tauri::command]
fn check_story_on_device(
    story_id: String,
    local_hash: Option<String>,
) -> Option<StoryCompareResult> {
    lunii_device::check_story_on_device(story_id, local_hash)
}

#[tauri::command]
fn get_device_info(mount: String) -> LuniiDeviceInfo {
    lunii_device::read_device_info(&mount)
}

// ── Commandes espace disque + listing audio ───────────────────────────────────

#[tauri::command]
fn get_storage_info(mount: String) -> Result<StorageInfo, String> {
    lunii_sync::get_storage_info(&mount)
}

#[tauri::command]
fn list_audio_files(folder_path: String) -> Result<Vec<AudioFile>, String> {
    lunii_sync::scan_audio_folder(&folder_path)
}

// ── Commandes sync ────────────────────────────────────────────────────────────

#[tauri::command]
fn scan_and_plan(folder_path: String) -> Result<SyncPlan, String> {
    let audio_files = lunii_sync::scan_audio_folder(&folder_path)?;
    let inventory = lunii_device::get_lunii_inventory();
    Ok(lunii_sync::determine_needed_pushes(&audio_files, &inventory))
}

#[tauri::command]
fn write_sidecar_after_push(
    mount: String,
    short_uuid: String,
    story_id: String,
    hash: String,
) -> Result<(), String> {
    lunii_sync::write_sidecar(&mount, &short_uuid, &story_id, &hash)
}

#[tauri::command]
fn remove_orphan_story(mount: String, short_uuid: String) -> Result<(), String> {
    lunii_sync::remove_orphan_story(&mount, &short_uuid)
}

// ── Réglages persistants ──────────────────────────────────────────────────────

#[tauri::command]
fn get_app_settings(app: tauri::AppHandle) -> app_settings::AppSettings {
    app_settings::load(&app)
}

#[tauri::command]
fn save_device_name(
    app: tauri::AppHandle,
    device_id: String,
    name: String,
) -> Result<(), String> {
    let mut settings = app_settings::load(&app);
    settings.devices.insert(device_id, app_settings::DeviceInfo { name });
    app_settings::save(&app, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_last_folder(app: tauri::AppHandle, folder: String) -> Result<(), String> {
    let mut settings = app_settings::load(&app);
    settings.last_audio_folder = Some(folder);
    app_settings::save(&app, &settings).map_err(|e| e.to_string())
}

// ── Lecture image couverture en base64 ────────────────────────────────────────

#[tauri::command]
fn get_cover_base64(path: String) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(&path).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    let mime = if buf.len() >= 2 {
        if buf[0] == 0xFF && buf[1] == 0xD8 { "image/jpeg" }
        else if buf[0] == b'B' && buf[1] == b'M' { "image/bmp" }
        else { "image/png" }
    } else { "image/png" };
    Some(format!("data:{};base64,{}", mime, base64_encode(&buf)))
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            if chunk.len() > 1 { chunk[1] } else { 0 },
            if chunk.len() > 2 { chunk[2] } else { 0 },
        ];
        out.push(TABLE[( b[0] >> 2)                    as usize] as char);
        out.push(TABLE[((b[0] & 3) << 4 | b[1] >> 4)  as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((b[1] & 0xf) << 2 | b[2] >> 6) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(b[2] & 0x3f) as usize] as char } else { '=' });
    }
    out
}

// ── Éjection device ───────────────────────────────────────────────────────────

#[tauri::command]
async fn eject_device(mount: String) -> Result<(), String> {
    use tokio::process::Command;
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("diskutil")
            .arg("eject")
            .arg(&mount)
            .output()
            .await
            .map_err(|e| format!("diskutil échoué : {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("umount")
            .arg(&mount)
            .output()
            .await
            .map_err(|e| format!("umount échoué : {e}"))?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Éjection non supportée sur cette plateforme".to_string())
    }
}

// ── Lancement du bridge Python ────────────────────────────────────────────────

/// Lance lunii-bridge.py en subprocess, stream les lignes JSON vers le frontend
/// via l'événement Tauri `sync:line`, et retourne quand le process termine.
#[tauri::command]
async fn start_sync(
    app: tauri::AppHandle,
    folder_path: String,
    device_mount: String,
    selected_files: Vec<String>,
) -> Result<String, String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let bridge_path = locate_bridge(&app)?;

    let python = locate_python3();

    let mut cmd = Command::new(&python);
    cmd.arg(&bridge_path)
        .arg(&folder_path)
        .arg(&device_mount)
        .env("PATH", "/Library/Frameworks/Python.framework/Versions/3.13/bin:/Library/Frameworks/Python.framework/Versions/3.12/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Passer les fichiers sélectionnés comme arguments supplémentaires
    for file in &selected_files {
        cmd.arg(file);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Impossible de lancer lunii-bridge.py avec '{}' : {e}", python))?;

    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");

    let app_stdout = app.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app_stdout.emit("sync:line", &line);
        }
    });

    let app_stderr = app.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Émet les erreurs stderr comme des lignes d'erreur JSON
            let json = serde_json::json!({"type": "stderr", "message": line});
            let _ = app_stderr.emit("sync:line", json.to_string());
        }
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Attente process échouée : {e}"))?;

    let _ = tokio::join!(stdout_task, stderr_task);

    if status.success() {
        Ok("ok".to_string())
    } else {
        Err(format!(
            "lunii-bridge.py a échoué (code {})",
            status.code().unwrap_or(-1)
        ))
    }
}

/// Répare le fichier d'index (.pi) de la Lunii via --repair-index dans le bridge Python.
#[tauri::command]
async fn repair_pack_index(
    app: tauri::AppHandle,
    device_mount: String,
) -> Result<String, String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let bridge_path = locate_bridge(&app)?;
    let python = locate_python3();

    let mut child = Command::new(&python)
        .arg(&bridge_path)
        .arg("--repair-index")
        .arg(&device_mount)
        .env("PATH", "/Library/Frameworks/Python.framework/Versions/3.13/bin:/Library/Frameworks/Python.framework/Versions/3.12/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Impossible de lancer lunii-bridge.py : {e}"))?;

    let stdout = child.stdout.take().expect("stdout");
    let app2 = app.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app2.emit("sync:line", &line);
        }
    });

    let status = child.wait().await.map_err(|e| format!("Attente échouée : {e}"))?;
    let _ = stdout_task.await;

    if status.success() { Ok("ok".to_string()) }
    else { Err(format!("Réparation échouée (code {})", status.code().unwrap_or(-1))) }
}

/// Retourne le chemin du python3 qui a PySide6 installé (évite /usr/bin/python3 système 3.9).
fn locate_python3() -> String {
    let candidates = [
        "/Library/Frameworks/Python.framework/Versions/3.13/bin/python3",
        "/Library/Frameworks/Python.framework/Versions/3.12/bin/python3",
        "/Library/Frameworks/Python.framework/Versions/3.11/bin/python3",
        "/opt/homebrew/bin/python3",
        "/usr/local/bin/python3",
        "python3",
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "python3".to_string()
}

/// Localise `lunii-bridge.py` dans le bundle (Resources/) ou en dev (racine projet).
fn locate_bridge(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(res_dir) = app.path().resource_dir() {
        // Tauri bundle : Resources/lunii-bridge.py
        let c1 = res_dir.join("lunii-bridge.py");
        if c1.exists() { return Ok(c1); }
        // Tauri bundle avec chemin ../  → Resources/_up_/lunii-bridge.py
        let c2 = res_dir.join("_up_").join("lunii-bridge.py");
        if c2.exists() { return Ok(c2); }
    }

    // Mode dev : remonte à la racine du projet
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1) {
            let candidate = ancestor.join("lunii-bridge.py");
            if candidate.exists() { return Ok(candidate); }
        }
    }

    Err("lunii-bridge.py introuvable dans le bundle.".to_string())
}

// ── Vérification mise à jour ──────────────────────────────────────────────────

const GITHUB_RELEASE_URL: &str =
    "https://api.github.com/repos/malikkaraoui/Lunii_Synchro/releases/latest";
const GITHUB_RELEASES_PAGE: &str =
    "https://github.com/malikkaraoui/Lunii_Synchro/releases/latest";

#[tauri::command]
async fn check_for_update() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("LuniiSync/2.0")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .get(GITHUB_RELEASE_URL)
        .send()
        .await
        .map_err(|e| format!("Requête échouée : {e}"))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON invalide : {e}"))?;

    json.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
        .ok_or_else(|| "tag_name absent de la réponse".to_string())
}

#[tauri::command]
fn open_release_page() -> Result<(), String> {
    open::that(GITHUB_RELEASES_PAGE).map_err(|e| format!("Impossible d'ouvrir le navigateur : {e}"))
}

#[tauri::command]
async fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("LuniiSync/2.0")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Récupérer l'URL de l'asset .tar.gz depuis la release GitHub
    let release: serde_json::Value = client
        .get(GITHUB_RELEASE_URL)
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    let asset_url = release["assets"]
        .as_array()
        .and_then(|arr| arr.iter().find(|a| {
            a["name"].as_str().map(|n| n.ends_with(".tar.gz")).unwrap_or(false)
        }))
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or("Aucun asset .tar.gz trouvé dans la release")?
        .to_string();

    // 2. Télécharger l'archive
    let bytes = client.get(&asset_url)
        .send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;

    let tmp_dir = std::env::temp_dir();
    let archive_path = tmp_dir.join("luniisync_update.tar.gz");
    let extract_dir = tmp_dir.join("luniisync_update");

    std::fs::write(&archive_path, &bytes).map_err(|e| e.to_string())?;

    // 3. Extraire
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    let status = std::process::Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap(),
               "-C", extract_dir.to_str().unwrap()])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Extraction de l'archive échouée".to_string());
    }

    // 4. Trouver le chemin du bundle .app courant (exe → .app/Contents/MacOS/binary)
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let app_bundle = exe.ancestors().nth(3)
        .ok_or("Impossible de déterminer le chemin du bundle")?.to_path_buf();
    let app_parent = app_bundle.parent()
        .ok_or("Impossible de trouver le dossier parent")?.to_string_lossy().into_owned();
    let app_bundle_str = app_bundle.to_string_lossy().into_owned();

    // 5. Script shell : attend la fermeture, remplace, relance
    let script = format!(
        "#!/bin/bash\nsleep 2\nrm -rf '{app_bundle_str}'\n\
         extracted=$(find '{extract}' -maxdepth 1 -name '*.app' | head -1)\n\
         cp -R \"$extracted\" '{app_parent}/'\n\
         xattr -rd com.apple.quarantine '{app_bundle_str}' 2>/dev/null\n\
         open '{app_bundle_str}'\n\
         rm -rf '{archive}' '{extract}' \"$0\"\n",
        extract = extract_dir.to_str().unwrap(),
        archive = archive_path.to_str().unwrap(),
    );
    let script_path = tmp_dir.join("luniisync_install.sh");
    std::fs::write(&script_path, &script).map_err(|e| e.to_string())?;

    std::process::Command::new("bash")
        .arg(&script_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    // 6. Quitter l'app courante
    app.exit(0);
    Ok(())
}

// ── Entrée principale ─────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            probe_lunii_device,
            get_device_info,
            get_lunii_inventory,
            check_story_on_device,
            get_storage_info,
            list_audio_files,
            scan_and_plan,
            write_sidecar_after_push,
            remove_orphan_story,
            get_app_settings,
            save_device_name,
            save_last_folder,
            get_cover_base64,
            eject_device,
            start_sync,
            check_for_update,
            open_release_page,
            download_and_install_update,
            repair_pack_index,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au démarrage de LuniiSync");
}
