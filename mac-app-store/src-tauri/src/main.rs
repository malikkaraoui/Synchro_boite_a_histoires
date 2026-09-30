#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_settings;
mod storybox_crypto;
mod storybox_device;
mod storybox_import;
mod storybox_sync;
mod story_pack;
mod studio_story;

use storybox_device::{StoryBoxDeviceInfo, StoryBoxDeviceProbe, StoryBoxInventoryResult, StoryCompareResult};
use storybox_sync::{AudioFile, StorageInfo, SyncPlan};
use tauri::Emitter;

// ── Commandes device ──────────────────────────────────────────────────────────

#[tauri::command]
fn probe_storybox_device() -> StoryBoxDeviceProbe {
    storybox_device::probe_storybox_device()
}

#[tauri::command]
fn get_storybox_inventory() -> StoryBoxInventoryResult {
    storybox_device::get_storybox_inventory()
}

#[tauri::command]
fn check_story_on_device(
    story_id: String,
    local_hash: Option<String>,
) -> Option<StoryCompareResult> {
    storybox_device::check_story_on_device(story_id, local_hash)
}

#[tauri::command]
fn get_device_info(mount: String) -> StoryBoxDeviceInfo {
    storybox_device::read_device_info(&mount)
}

// ── Commandes espace disque + listing audio ───────────────────────────────────

#[tauri::command]
fn get_storage_info(mount: String) -> Result<StorageInfo, String> {
    storybox_sync::get_storage_info(&mount)
}

#[tauri::command]
fn list_audio_files(folder_path: String) -> Result<Vec<AudioFile>, String> {
    storybox_sync::scan_audio_folder(&folder_path)
}

// ── Commandes sync ────────────────────────────────────────────────────────────

#[tauri::command]
fn scan_and_plan(folder_path: String) -> Result<SyncPlan, String> {
    let audio_files = storybox_sync::scan_audio_folder(&folder_path)?;
    let inventory = storybox_device::get_storybox_inventory();
    Ok(storybox_sync::determine_needed_pushes(&audio_files, &inventory))
}

#[tauri::command]
fn write_sidecar_after_push(
    mount: String,
    short_uuid: String,
    story_id: String,
    hash: String,
) -> Result<(), String> {
    storybox_sync::write_sidecar(&mount, &short_uuid, &story_id, &hash)
}

#[tauri::command]
fn remove_orphan_story(mount: String, short_uuid: String) -> Result<(), String> {
    storybox_sync::remove_orphan_story(&mount, &short_uuid)
}

#[tauri::command]
fn move_story_in_pack_index(
    mount: String,
    short_uuid: String,
    direction: i32,
) -> Result<(), String> {
    storybox_device::move_story_in_pack_index(&mount, &short_uuid, direction)
}

#[tauri::command]
fn reorder_story_in_pack_index(
    mount: String,
    short_uuid: String,
    new_index: usize,
) -> Result<(), String> {
    storybox_device::reorder_story_in_pack_index(&mount, &short_uuid, new_index)
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
// Sandbox App Store : pas d'appel diskutil. L'utilisateur éjecte via le Finder.

#[tauri::command]
async fn eject_device(mount: String) -> Result<(), String> {
    let _ = mount;
    Err("Éjectez la boîte depuis le Finder (clic droit sur le volume → Éjecter).".to_string())
}

/// Vérifie qu'un chemin est bien une boîte boîte à histoires (fichier .md présent).
#[tauri::command]
fn validate_storybox_mount(path: String) -> bool {
    std::path::Path::new(&path).join(".md").exists()
}

// ── Pipeline d'import natif Rust ──────────────────────────────────────────────
//
// Pour chaque fichier audio sélectionné :
//   1. Génère un story pack ZIP (remplace SPG)
//   2. Injecte une couverture placeholder si absente
//   3. Patche story.json pour lecture directe
//   4. Importe vers la boîte à histoires via crypto XXTEA natif

#[tauri::command]
async fn start_sync(
    app: tauri::AppHandle,
    folder_path: String,
    device_mount: String,
    selected_files: Vec<String>,
) -> Result<String, String> {
    let _ = folder_path;
    let total = selected_files.len();
    emit_sync_line(&app, serde_json::json!({
        "type": "progress", "step": "scan",
        "message": format!("{total} fichier(s) à transférer.")
    }));

    let mut added = 0u32;
    let mut errors = 0u32;

    for (i, audio_path_str) in selected_files.iter().enumerate() {
        let audio_path = std::path::Path::new(audio_path_str);
        let display = audio_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let story_id = audio_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        emit_sync_line(&app, serde_json::json!({
            "type": "progress", "step": "import",
            "file": display, "current": i + 1, "total": total,
            "message": format!("[{}/{}] {}…", i + 1, total, display)
        }));

        let zip_path = match storybox_import::generate_simple_pack(audio_path) {
            Ok(p) => p,
            Err(e) => {
                emit_sync_line(&app, serde_json::json!({"type":"error","file":display,"message":e}));
                errors += 1;
                continue;
            }
        };
        let tmp_dir = zip_path.parent().map(|p| p.to_path_buf());

        let _ = story_pack::inject_placeholder_cover_if_missing(&zip_path, &story_id);
        let _ = story_pack::patch_direct_play_zip(&zip_path);

        let hash = storybox_sync::compute_file_hash(audio_path).unwrap_or_default();

        let app_ref = app.clone();
        let result = storybox_import::import_story(
            &device_mount,
            &zip_path,
            &story_id,
            &hash,
            &move |msg| {
                emit_sync_line(&app_ref, serde_json::json!({
                    "type": "progress", "step": "import", "message": msg
                }));
            },
        );

        if let Some(dir) = tmp_dir {
            let _ = std::fs::remove_dir_all(dir);
        }

        match result {
            Ok(imported) => {
                added += 1;
                emit_sync_line(&app, serde_json::json!({
                    "type": "progress", "step": "import",
                    "file": display,
                    "shortUuid": imported.short_uuid,
                    "message": format!("✓ {display} ({})", imported.short_uuid)
                }));
            }
            Err(e) => {
                errors += 1;
                emit_sync_line(&app, serde_json::json!({"type":"error","file":display,"message":e}));
            }
        }
    }

    emit_sync_line(&app, serde_json::json!({
        "type": "done",
        "added": added, "errors": errors,
        "message": format!("Terminé : {added} ajouté(s), {errors} erreur(s).")
    }));
    Ok("ok".to_string())
}

fn emit_sync_line(app: &tauri::AppHandle, payload: serde_json::Value) {
    let _ = app.emit("sync:line", payload.to_string());
}

/// Répare le fichier d'index (.pi) de la boîte à histoires en pur Rust.
#[tauri::command]
async fn repair_pack_index(device_mount: String) -> Result<String, String> {
    storybox_device::repair_pack_index_native(&device_mount)?;
    Ok("ok".to_string())
}

// ── Canal de distribution + mise à jour (App Store gère via le Store) ─────────

const BUILD_DISTRIBUTION_CHANNEL: &str = "mac-app-store";

#[tauri::command]
fn get_distribution_channel() -> String {
    BUILD_DISTRIBUTION_CHANNEL.to_string()
}

#[tauri::command]
async fn check_for_update() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
fn open_release_page() -> Result<(), String> {
    Err("Cette version se met à jour via le Mac App Store.".to_string())
}

#[tauri::command]
async fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app;
    Err("Cette version se met à jour via le Mac App Store.".to_string())
}

// ── Entrée principale ─────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            probe_storybox_device,
            get_device_info,
            get_storybox_inventory,
            check_story_on_device,
            get_storage_info,
            list_audio_files,
            scan_and_plan,
            write_sidecar_after_push,
            remove_orphan_story,
            move_story_in_pack_index,
            reorder_story_in_pack_index,
            get_app_settings,
            save_device_name,
            save_last_folder,
            get_cover_base64,
            get_distribution_channel,
            eject_device,
            validate_storybox_mount,
            start_sync,
            check_for_update,
            open_release_page,
            download_and_install_update,
            repair_pack_index,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au démarrage de Synchro Boîte à histoires");
}
