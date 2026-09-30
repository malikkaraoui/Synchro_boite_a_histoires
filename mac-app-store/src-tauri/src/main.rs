#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_settings;
mod sandbox_access;
mod storybox_crypto;
mod storybox_device;
mod storybox_import;
mod storybox_sync;
mod storybox_v3;
mod story_pack;
mod studio_story;

use sandbox_access::SandboxAccess;
use serde::Serialize;
use std::path::Path;
use storybox_device::{
    DeviceState, StoryBoxDeviceInfo, StoryBoxDeviceProbe, StoryBoxInventoryResult, StoryCompareResult,
};
use storybox_sync::{AudioFile, StorageInfo, SyncPlan};
use tauri::{Emitter, Manager, State};

// ── Commandes device ──────────────────────────────────────────────────────────

/// Détection. Une boîte déjà validée est resondée seule ; sinon on cherche dans `/Volumes`
/// et, si la boîte y est illisible (sandbox), on tente de rouvrir l'accès par son bookmark.
#[tauri::command]
fn probe_storybox_device(app: tauri::AppHandle, access: State<'_, SandboxAccess>) -> StoryBoxDeviceProbe {
    // Même montage ET même numéro de série ; sinon l'accès est refermé (`stop…` au drop).
    if let Some(probe) = access.reprobe_validated() {
        return probe;
    }

    let probe = storybox_device::probe_storybox_device();
    match (probe.state, probe.mount.clone()) {
        (DeviceState::Connected, Some(mount)) => {
            access.set_device(mount, probe.device_id.clone(), None);
            probe
        }
        (DeviceState::AccessRequired, Some(mount)) => {
            restore_device_access(&app, &access, &mount).unwrap_or(probe)
        }
        _ => probe,
    }
}

/// Rouvre l'accès à `mount` avec le bookmark de la même boîte, s'il y en a un.
fn restore_device_access(
    app: &tauri::AppHandle,
    access: &SandboxAccess,
    mount: &str,
) -> Option<StoryBoxDeviceProbe> {
    let mut settings = app_settings::load(app);
    for (device_id, bookmark) in settings.device_bookmarks() {
        // Une boîte débranchée ne se résout pas (aucun montage automatique).
        let Ok(resolved) = sandbox_access::resolve(&bookmark) else { continue };
        if resolved.path != Path::new(mount) {
            continue;
        }
        let probe = storybox_device::probe_mount(Path::new(mount));
        if !probe.connected || probe.device_id.as_deref() != Some(device_id.as_str()) {
            continue;
        }
        if resolved.stale {
            match sandbox_access::create_bookmark(Path::new(mount)) {
                Ok(fresh) => {
                    settings.set_device_bookmark(&device_id, &fresh);
                    if let Err(e) = app_settings::save(app, &settings) {
                        eprintln!("[sandbox] bookmark boîte régénéré mais non enregistré : {e}");
                    }
                }
                Err(e) => eprintln!("[sandbox] bookmark boîte périmé non régénéré : {e}"),
            }
        }
        access.set_device(mount.to_string(), probe.device_id.clone(), Some(resolved.access));
        return Some(probe);
    }
    None
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceAccessGrant {
    probe: StoryBoxDeviceProbe,
    /// Faux si l'accès ne vaut que pour cette session (bookmark non enregistré).
    remembered: bool,
    warning: Option<String>,
}

/// Appelée après le NSOpenPanel : valide la boîte choisie et mémorise son bookmark.
#[tauri::command]
fn grant_device_access(
    app: tauri::AppHandle,
    access: State<'_, SandboxAccess>,
    path: String,
) -> Result<DeviceAccessGrant, String> {
    let mount = Path::new(&path);
    let probe = storybox_device::probe_mount(mount);
    match probe.state {
        DeviceState::Connected => {}
        DeviceState::AccessRequired => {
            return Err("macOS refuse encore la lecture de la boîte. Réessayez en choisissant la boîte elle-même.".to_string())
        }
        DeviceState::NotConnected => {
            return Err("Ce dossier n'est pas une boîte à histoires. Choisissez la boîte elle-même, dans la colonne de gauche de la fenêtre.".to_string())
        }
    }
    let warning = remember_device(&app, &probe, mount).err();
    if let Some(w) = &warning {
        eprintln!("[sandbox] {w}");
    }
    access.set_device(path, probe.device_id.clone(), None);
    Ok(DeviceAccessGrant { remembered: warning.is_none(), warning, probe })
}

fn remember_device(app: &tauri::AppHandle, probe: &StoryBoxDeviceProbe, mount: &Path) -> Result<(), String> {
    let device_id = probe
        .device_id
        .as_deref()
        .ok_or_else(|| "Identifiant de la boîte illisible : accès non mémorisé".to_string())?;
    let bookmark = sandbox_access::create_bookmark(mount)?;
    let mut settings = app_settings::load(app);
    settings.set_device_bookmark(device_id, &bookmark);
    app_settings::save(app, &settings).map_err(|e| format!("Réglages non enregistrés : {e}"))
}

#[tauri::command]
fn get_storybox_inventory(access: State<'_, SandboxAccess>) -> StoryBoxInventoryResult {
    storybox_device::get_storybox_inventory(access.validated_mount().as_deref())
}

#[tauri::command]
fn check_story_on_device(
    access: State<'_, SandboxAccess>,
    story_id: String,
    local_hash: Option<String>,
) -> Option<StoryCompareResult> {
    storybox_device::check_story_on_device(access.validated_mount().as_deref(), story_id, local_hash)
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderAccessGrant {
    remembered: bool,
    warning: Option<String>,
}

/// Appelée après le NSOpenPanel du dossier audio : mémorise le dossier avec son bookmark.
#[tauri::command]
fn grant_audio_folder_access(
    app: tauri::AppHandle,
    access: State<'_, SandboxAccess>,
    folder: String,
) -> Result<FolderAccessGrant, String> {
    std::fs::read_dir(&folder).map_err(|e| format!("Lecture dossier échouée : {e}"))?;
    let bookmark = sandbox_access::create_bookmark(Path::new(&folder));
    let mut settings = app_settings::load(&app);
    settings.set_audio_folder(&folder, bookmark.as_deref().ok());
    let mut warning = bookmark.err();
    if let Err(e) = app_settings::save(&app, &settings) {
        warning = Some(format!("Réglages non enregistrés : {e}"));
    }
    if let Some(w) = &warning {
        eprintln!("[sandbox] {w}");
    }
    // L'accès au nouveau dossier vient du NSOpenPanel ; l'ancien accès est refermé.
    access.set_audio(None);
    Ok(FolderAccessGrant { remembered: warning.is_none(), warning })
}

/// Au lancement : rouvre le dossier audio mémorisé, uniquement par son bookmark.
/// `Ok(None)` : aucun dossier mémorisé avec bookmark (`lastAudioFolder` seul n'est pas relu).
#[tauri::command]
fn restore_audio_folder(
    app: tauri::AppHandle,
    access: State<'_, SandboxAccess>,
) -> Result<Option<String>, String> {
    let mut settings = app_settings::load(&app);
    let Some(bookmark) = settings.audio_folder_bookmark() else { return Ok(None) };
    let resolved = sandbox_access::resolve(&bookmark)?;
    let folder = resolved.path.to_string_lossy().into_owned();
    if resolved.stale {
        match sandbox_access::create_bookmark(&resolved.path) {
            Ok(fresh) => {
                settings.set_audio_folder(&folder, Some(&fresh));
                if let Err(e) = app_settings::save(&app, &settings) {
                    eprintln!("[sandbox] bookmark dossier régénéré mais non enregistré : {e}");
                }
            }
            Err(e) => eprintln!("[sandbox] bookmark dossier périmé non régénéré : {e}"),
        }
    }
    access.set_audio(Some(resolved.access));
    Ok(Some(folder))
}

// ── Commandes sync ────────────────────────────────────────────────────────────

#[tauri::command]
fn scan_and_plan(access: State<'_, SandboxAccess>, folder_path: String) -> Result<SyncPlan, String> {
    let audio_files = storybox_sync::scan_audio_folder(&folder_path)?;
    let inventory = storybox_device::get_storybox_inventory(access.validated_mount().as_deref());
    Ok(storybox_sync::determine_needed_pushes(&audio_files, &inventory))
}

#[tauri::command]
fn write_sidecar_after_push(
    access: State<'_, SandboxAccess>,
    mount: String,
    device_id: String,
    short_uuid: String,
    story_id: String,
    hash: String,
) -> Result<(), String> {
    access.require_mount(&mount, &device_id)?;
    storybox_sync::write_sidecar(&mount, &short_uuid, &story_id, &hash)
}

#[tauri::command]
fn remove_orphan_story(
    access: State<'_, SandboxAccess>,
    mount: String,
    device_id: String,
    short_uuid: String,
) -> Result<(), String> {
    access.require_mount(&mount, &device_id)?;
    storybox_sync::remove_orphan_story(&mount, &short_uuid)
}

#[tauri::command]
fn move_story_in_pack_index(
    access: State<'_, SandboxAccess>,
    mount: String,
    device_id: String,
    short_uuid: String,
    direction: i32,
) -> Result<(), String> {
    access.require_mount(&mount, &device_id)?;
    storybox_device::move_story_in_pack_index(&mount, &short_uuid, direction)
}

#[tauri::command]
fn reorder_story_in_pack_index(
    access: State<'_, SandboxAccess>,
    mount: String,
    device_id: String,
    short_uuid: String,
    new_index: usize,
) -> Result<(), String> {
    access.require_mount(&mount, &device_id)?;
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
    access: State<'_, SandboxAccess>,
    folder_path: String,
    device_mount: String,
    device_id: String,
    selected_files: Vec<String>,
) -> Result<String, String> {
    let _ = folder_path;
    access.require_mount(&device_mount, &device_id)?;
    let total = selected_files.len();
    emit_sync_line(&app, serde_json::json!({
        "type": "progress", "step": "scan",
        "message": format!("{total} fichier(s) à transférer.")
    }));

    let mut added = 0u32;
    let mut errors = 0u32;

    for (i, audio_path_str) in selected_files.iter().enumerate() {
        // Avant chaque fichier : toujours la même boîte physique (numéro de série relu).
        if let Err(e) = access.require_mount(&device_mount, &device_id) {
            emit_sync_line(&app, serde_json::json!({"type":"error","message":e}));
            errors += (total - i) as u32;
            break;
        }

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
/// Seuls les dossiers complets sont indexés ; les incomplets sont renvoyés, pas supprimés.
#[tauri::command]
async fn repair_pack_index(
    access: State<'_, SandboxAccess>,
    device_mount: String,
    device_id: String,
) -> Result<storybox_device::PackIndexRepair, String> {
    access.require_mount(&device_mount, &device_id)?;
    storybox_device::repair_pack_index_native(&device_mount)
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
        .manage(SandboxAccess::default())
        .invoke_handler(tauri::generate_handler![
            probe_storybox_device,
            grant_device_access,
            get_device_info,
            get_storybox_inventory,
            check_story_on_device,
            get_storage_info,
            list_audio_files,
            grant_audio_folder_access,
            restore_audio_folder,
            scan_and_plan,
            write_sidecar_after_push,
            remove_orphan_story,
            move_story_in_pack_index,
            reorder_story_in_pack_index,
            get_app_settings,
            save_device_name,
            get_cover_base64,
            get_distribution_channel,
            eject_device,
            start_sync,
            check_for_update,
            open_release_page,
            download_and_install_update,
            repair_pack_index,
        ])
        .build(tauri::generate_context!())
        .expect("Erreur au démarrage de Synchro Boîte à histoires")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Fermeture : `stopAccessingSecurityScopedResource` sur tous les accès ouverts.
                app.state::<SandboxAccess>().release_all();
            }
        });
}
