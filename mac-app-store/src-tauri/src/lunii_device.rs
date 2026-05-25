//! Détection et inventaire natif d'une boîte Lunii montée en USB.
//! Porté depuis lunii-studio/tauri/src-tauri/src/lunii_device.rs
//! + ajout du fallback détection par nom de volume.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LuniiDeviceProbe {
    pub connected: bool,
    pub mount: Option<String>,
    pub device_id: Option<String>,
    pub marker_found: bool,
    pub content_dir_present: bool,
    pub story_dir_count: usize,
    pub detection_method: Option<String>,
}

/// Métadonnées du sidecar `.lunii-studio.json` écrit par Studio au push.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SidecarData {
    pub story_id: String,
    pub hash: String,
    pub pushed_at: String,
    pub source: String,
}

/// Une story trouvée dans `.content/<short_uuid>/`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LuniiStoryEntry {
    pub short_uuid: String,
    /// Présent uniquement si `.lunii-studio.json` valide existe.
    pub sidecar: Option<SidecarData>,
    /// Titre lisible : depuis le sidecar, story.json ou titre.txt.
    pub title: Option<String>,
    /// Chemin absolu de l'image de couverture si trouvée dans le dossier story.
    pub cover_path: Option<String>,
    /// Taille totale du dossier story en octets.
    pub size_bytes: u64,
}

/// Inventaire complet d'un device Lunii monté.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LuniiInventory {
    pub mount: String,
    pub stories: Vec<LuniiStoryEntry>,
    pub total_stories: usize,
    pub managed_stories: usize,
}

/// Discriminant explicite du résultat `get_lunii_inventory`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InventoryStatus {
    NotConnected,
    NoContentDir,
    ReadError,
    Ok,
}

/// Résultat toujours retourné de `get_lunii_inventory` — jamais None.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LuniiInventoryResult {
    pub status: InventoryStatus,
    pub mount: Option<String>,
    pub stories: Vec<LuniiStoryEntry>,
    pub total_stories: usize,
    pub managed_stories: usize,
    pub error: Option<String>,
}

/// Informations matérielles et firmware lues depuis `.md`.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LuniiDeviceInfo {
    pub hw_version: u8,
    pub fw_major: u8,
    pub fw_minor: u8,
    pub fw_subminor: u8,
    pub serial: String,
}

/// Statut d'une story Studio vis-à-vis du device connecté.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoryDeviceStatus {
    NotOnDevice,
    Present,
    UpToDate,
    Outdated,
}

/// Résultat de la comparaison d'une story Studio contre l'inventaire device.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryCompareResult {
    pub story_id: String,
    pub status: StoryDeviceStatus,
    pub device_short_uuid: Option<String>,
    pub device_hash: Option<String>,
}

/// Compare une story Studio contre la liste des stories du device.
pub fn compare_story(
    story_id: &str,
    local_hash: Option<&str>,
    stories: &[LuniiStoryEntry],
) -> StoryCompareResult {
    let found = stories.iter().find(|s| {
        s.sidecar
            .as_ref()
            .is_some_and(|sc| sc.story_id == story_id)
    });

    match found {
        None => StoryCompareResult {
            story_id: story_id.to_string(),
            status: StoryDeviceStatus::NotOnDevice,
            device_short_uuid: None,
            device_hash: None,
        },
        Some(entry) => {
            let device_hash = entry.sidecar.as_ref().map(|sc| sc.hash.clone());
            let status = match (local_hash, &device_hash) {
                (Some(lh), Some(dh)) if lh == dh => StoryDeviceStatus::UpToDate,
                (Some(_), Some(_)) => StoryDeviceStatus::Outdated,
                _ => StoryDeviceStatus::Present,
            };
            StoryCompareResult {
                story_id: story_id.to_string(),
                status,
                device_short_uuid: Some(entry.short_uuid.clone()),
                device_hash,
            }
        }
    }
}

/// Vérifie si une story Studio est présente sur le device, avec comparaison de hash.
pub fn check_story_on_device(
    story_id: String,
    local_hash: Option<String>,
) -> Option<StoryCompareResult> {
    let result = get_lunii_inventory();
    if result.status != InventoryStatus::Ok {
        return None;
    }
    Some(compare_story(&story_id, local_hash.as_deref(), &result.stories))
}

impl LuniiDeviceProbe {
    fn disconnected() -> Self {
        Self {
            connected: false,
            mount: None,
            device_id: None,
            marker_found: false,
            content_dir_present: false,
            story_dir_count: 0,
            detection_method: None,
        }
    }

    fn connected(mount: PathBuf, detection_method: &str, marker_found: bool) -> Self {
        let content_dir = mount.join(".content");
        let story_dir_count = count_story_dirs(&content_dir);
        let mount_str = mount.to_str().unwrap_or("");

        // Utilise le numéro de série hardware (stable) en priorité,
        // sinon le Volume UUID macOS (peut changer sur FAT entre montages).
        let info = read_device_info(mount_str);
        let device_id = if !info.serial.is_empty() && info.serial != "000000000000000000" {
            Some(format!("serial-{}", info.serial))
        } else {
            get_volume_id(mount_str)
        };

        Self {
            connected: true,
            mount: Some(mount.to_string_lossy().into_owned()),
            device_id,
            marker_found,
            content_dir_present: content_dir.is_dir(),
            story_dir_count,
            detection_method: Some(detection_method.to_string()),
        }
    }
}

/// Retourne un identifiant stable pour le volume monté (UUID macOS, device Linux).
pub fn get_volume_id(mount: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("diskutil")
            .args(["info", mount])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("Volume UUID") {
                if let Some(id) = line.split(':').nth(1) {
                    let id = id.trim().to_string();
                    if !id.is_empty() && id != "none" { return Some(id); }
                }
            }
        }
        // Fallback: serial number from disk
        for line in text.lines() {
            if line.to_lowercase().contains("disk identifier") {
                if let Some(id) = line.split(':').nth(1) {
                    return Some(format!("disk-{}", id.trim()));
                }
            }
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    { Some(format!("vol-{}", mount.replace('/', "_"))) }
}

fn dir_size_bytes(path: &Path) -> u64 {
    match fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| {
                let p = e.path();
                if p.is_dir() { dir_size_bytes(&p) }
                else { e.metadata().map(|m| m.len()).unwrap_or(0) }
            })
            .sum(),
        Err(_) => 0,
    }
}

fn short_uuid_from_uuid_bytes(uuid_bytes: &[u8; 16]) -> String {
    uuid_bytes[12..]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect()
}

fn short_uuid_to_uuid_bytes(short_uuid: &str) -> Result<[u8; 16], String> {
    let normalized = short_uuid.trim().to_uppercase();
    if normalized.len() != 8 || !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Short UUID invalide : {short_uuid}"));
    }

    let mut uuid = [0u8; 16];
    for i in 0..4 {
        let start = i * 2;
        let byte = u8::from_str_radix(&normalized[start..start + 2], 16)
            .map_err(|_| format!("Short UUID invalide : {short_uuid}"))?;
        uuid[12 + i] = byte;
    }
    Ok(uuid)
}

fn is_short_uuid_dir_name(name: &str) -> bool {
    name.len() == 8 && name.chars().all(|c| c.is_ascii_hexdigit())
}

fn collect_content_short_uuids(content_dir: &Path) -> Result<Vec<String>, String> {
    let mut short_uuids: Vec<String> = fs::read_dir(content_dir)
        .map_err(|e| format!("Lecture {:?} échouée : {e}", content_dir))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_uppercase();
            is_short_uuid_dir_name(&name).then_some(name)
        })
        .collect();

    short_uuids.sort();
    Ok(short_uuids)
}

fn filter_known_short_uuids(entries: &[[u8; 16]], known_short_uuids: &HashSet<String>) -> Vec<String> {
    let mut seen = HashSet::new();

    entries
        .iter()
        .map(short_uuid_from_uuid_bytes)
        .filter(|short_uuid| known_short_uuids.contains(short_uuid))
        .filter(|short_uuid| seen.insert(short_uuid.clone()))
        .collect()
}

fn read_pack_index_entries(index_path: &Path) -> Result<Vec<[u8; 16]>, String> {
    let data = fs::read(index_path)
        .map_err(|e| format!("Lecture {:?} échouée : {e}", index_path))?;

    if data.len() % 16 != 0 {
        return Err(format!("Index {:?} corrompu : taille invalide", index_path));
    }

    Ok(data
        .chunks_exact(16)
        .map(|chunk| {
            let mut uuid = [0u8; 16];
            uuid.copy_from_slice(chunk);
            uuid
        })
        .collect())
}

fn write_pack_index_entries(index_path: &Path, entries: &[[u8; 16]]) -> Result<(), String> {
    let mut out = Vec::with_capacity(entries.len() * 16);
    for entry in entries {
        out.extend_from_slice(entry);
    }
    fs::write(index_path, out).map_err(|e| format!("Écriture {:?} échouée : {e}", index_path))
}

fn read_all_pack_index_entries(mount: &Path) -> Result<(Vec<[u8; 16]>, Vec<[u8; 16]>), String> {
    let visible_path = mount.join(".pi");
    let hidden_path = mount.join(".pi.hidden");

    let visible = if visible_path.is_file() {
        read_pack_index_entries(&visible_path)?
    } else {
        Vec::new()
    };

    let hidden = if hidden_path.is_file() {
        read_pack_index_entries(&hidden_path)?
    } else {
        Vec::new()
    };

    Ok((visible, hidden))
}

fn write_all_pack_index_entries(
    mount: &Path,
    visible_entries: &[[u8; 16]],
    hidden_entries: &[[u8; 16]],
) -> Result<(), String> {
    write_pack_index_entries(&mount.join(".pi"), visible_entries)?;
    write_pack_index_entries(&mount.join(".pi.hidden"), hidden_entries)?;
    Ok(())
}

pub fn reorder_story_in_pack_index(
    mount: &str,
    short_uuid: &str,
    new_index: usize,
) -> Result<(), String> {
    let mount_path = Path::new(mount);
    let (mut visible_entries, hidden_entries) = read_all_pack_index_entries(mount_path)?;

    if visible_entries.is_empty() {
        return Err("Index .pi vide".to_string());
    }

    if new_index >= visible_entries.len() {
        return Err(format!("Position cible invalide : {new_index}"));
    }

    let wanted = short_uuid.to_uppercase();
    let current_idx = visible_entries
        .iter()
        .position(|entry| short_uuid_from_uuid_bytes(entry) == wanted)
        .ok_or_else(|| format!("Histoire introuvable dans l'index : {wanted}"))?;

    if current_idx == new_index {
        return Ok(());
    }

    let entry = visible_entries.remove(current_idx);
    visible_entries.insert(new_index, entry);

    write_all_pack_index_entries(mount_path, &visible_entries, &hidden_entries)
}

pub fn repair_pack_index_native(mount: &str) -> Result<(), String> {
    let mount_path = Path::new(mount);
    if !mount_path.is_dir() {
        return Err(format!("Montage Lunii introuvable : {mount}"));
    }

    let content_dir = mount_path.join(".content");
    if !content_dir.is_dir() {
        return Err("Dossier .content introuvable sur la boîte".to_string());
    }

    let content_short_uuids = collect_content_short_uuids(&content_dir)?;
    let known_short_uuids: HashSet<String> = content_short_uuids.iter().cloned().collect();

    let visible_entries = read_pack_index_entries(&mount_path.join(".pi")).unwrap_or_default();
    let hidden_entries = read_pack_index_entries(&mount_path.join(".pi.hidden")).unwrap_or_default();

    let hidden_short_uuids = filter_known_short_uuids(&hidden_entries, &known_short_uuids);
    let hidden_short_set: HashSet<String> = hidden_short_uuids.iter().cloned().collect();

    let mut visible_short_uuids = filter_known_short_uuids(&visible_entries, &known_short_uuids)
        .into_iter()
        .filter(|short_uuid| !hidden_short_set.contains(short_uuid))
        .collect::<Vec<_>>();

    let mut present_short_uuids: HashSet<String> = visible_short_uuids.iter().cloned().collect();
    present_short_uuids.extend(hidden_short_uuids.iter().cloned());

    for short_uuid in content_short_uuids {
        if present_short_uuids.insert(short_uuid.clone()) {
            visible_short_uuids.push(short_uuid);
        }
    }

    let visible_uuid_entries = visible_short_uuids
        .iter()
        .map(|short_uuid| short_uuid_to_uuid_bytes(short_uuid))
        .collect::<Result<Vec<_>, _>>()?;
    let hidden_uuid_entries = hidden_short_uuids
        .iter()
        .map(|short_uuid| short_uuid_to_uuid_bytes(short_uuid))
        .collect::<Result<Vec<_>, _>>()?;

    write_all_pack_index_entries(mount_path, &visible_uuid_entries, &hidden_uuid_entries)
}

pub fn move_story_in_pack_index(mount: &str, short_uuid: &str, direction: i32) -> Result<(), String> {
    if direction == 0 {
        return Ok(());
    }

    let mount_path = Path::new(mount);
    if !mount_path.join(".pi").is_file() {
        return Err("Index .pi introuvable sur la boîte".to_string());
    }

    let (entries, _) = read_all_pack_index_entries(mount_path)?;
    if entries.is_empty() {
        return Err("Index .pi vide".to_string());
    }

    let wanted = short_uuid.to_uppercase();
    let idx = entries
        .iter()
        .position(|entry| short_uuid_from_uuid_bytes(entry) == wanted)
        .ok_or_else(|| format!("Histoire introuvable dans l'index : {wanted}"))?;

    let target_idx = if direction < 0 {
        idx.checked_sub(1)
            .ok_or_else(|| "Cette histoire est déjà en première position".to_string())?
    } else {
        let next = idx + 1;
        if next >= entries.len() {
            return Err("Cette histoire est déjà en dernière position".to_string());
        }
        next
    };

    reorder_story_in_pack_index(mount, short_uuid, target_idx)
}

fn count_story_dirs(content_dir: &Path) -> usize {
    match fs::read_dir(content_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .count(),
        Err(_) => 0,
    }
}

fn sorted_child_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = match fs::read_dir(root) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    dirs.sort();
    dirs
}

fn probe_mount_candidate(path: &Path) -> Option<LuniiDeviceProbe> {
    // Méthode 1 : fichier marqueur `.md` à la racine (Lunii officiel)
    if path.join(".md").exists() {
        return Some(LuniiDeviceProbe::connected(path.to_path_buf(), "marker", true));
    }

    // Méthode 2 : nom de volume contient "LUNII" (fallback macOS/Windows)
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    if name.contains("LUNII") {
        return Some(LuniiDeviceProbe::connected(path.to_path_buf(), "volume-name", false));
    }

    None
}

fn probe_root(root: &Path, nested_levels: usize) -> Option<LuniiDeviceProbe> {
    for child in sorted_child_dirs(root) {
        if let Some(probe) = probe_mount_candidate(&child) {
            return Some(probe);
        }
        if nested_levels > 0 {
            for nested in sorted_child_dirs(&child) {
                if let Some(probe) = probe_mount_candidate(&nested) {
                    return Some(probe);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn probe_platform() -> LuniiDeviceProbe {
    probe_root(Path::new("/Volumes"), 0).unwrap_or_else(LuniiDeviceProbe::disconnected)
}

#[cfg(target_os = "linux")]
fn probe_platform() -> LuniiDeviceProbe {
    for (root, nested) in [
        (Path::new("/run/media"), 1usize),
        (Path::new("/media"), 1usize),
        (Path::new("/mnt"), 0usize),
        (Path::new("/Volumes"), 0usize),
    ] {
        if let Some(probe) = probe_root(root, nested) {
            return probe;
        }
    }
    LuniiDeviceProbe::disconnected()
}

#[cfg(target_os = "windows")]
fn probe_platform() -> LuniiDeviceProbe {
    for letter in b'A'..=b'Z' {
        let mount = PathBuf::from(format!("{}:\\", letter as char));
        if !mount.exists() {
            continue;
        }
        if let Some(probe) = probe_mount_candidate(&mount) {
            return probe;
        }
    }
    LuniiDeviceProbe::disconnected()
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn probe_platform() -> LuniiDeviceProbe {
    LuniiDeviceProbe::disconnected()
}

pub fn probe_lunii_device() -> LuniiDeviceProbe {
    probe_platform()
}

/// Parse `.lunii-studio.json` depuis un dossier story.
/// Accepte les sidecars écrits par LuniiSync ("luniisync") et lunii-studio ("lunii-studio").
fn read_sidecar(story_dir: &Path) -> Option<SidecarData> {
    let text = fs::read_to_string(story_dir.join(".lunii-studio.json")).ok()?;
    let val: serde_json::Value = serde_json::from_str(&text).ok()?;
    let source = val.get("source").and_then(|v| v.as_str()).unwrap_or("");
    if source != "lunii-studio" && source != "luniisync" {
        return None;
    }
    // Accepte camelCase ("storyId") et snake_case ("story_id")
    let story_id = val.get("storyId")
        .or_else(|| val.get("story_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let pushed_at = val.get("pushedAt")
        .or_else(|| val.get("pushed_at"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some(SidecarData {
        story_id,
        hash: val.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        pushed_at,
        source: source.to_string(),
    })
}

/// Détecte si un fichier est une image lisible via ses magic bytes.
/// Retourne Some("png"|"jpg"|"bmp") ou None si pas reconnu.
/// Lit les infos matérielles/firmware depuis le fichier `.md` d'une Lunii.
pub fn read_device_info(mount: &str) -> LuniiDeviceInfo {
    let path = Path::new(mount).join(".md");
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(_) => return LuniiDeviceInfo::default(),
    };
    if data.len() < 32 {
        return LuniiDeviceInfo::default();
    }
    // md_version = premier octet (little-endian 2 bytes, on prend le 1er)
    let md_version = data[0];
    let hw_version = if md_version >= 6 { 3 } else if md_version >= 3 { 2 } else { 1 };

    // Pour versions 6/7 : fw à offset 2 (format ASCII 0x30+digit)
    // Pour versions 1-5 : fw aux offsets 4,5,6 (big-endian words)
    let (fw_major, fw_minor, fw_subminor) = if md_version >= 6 && data.len() > 7 {
        (
            data[2].saturating_sub(0x30),
            data[4].saturating_sub(0x30),
            data[6].saturating_sub(0x30),
        )
    } else if data.len() >= 8 {
        let maj = u16::from_be_bytes([data[4], data[5]]);
        let min = u16::from_be_bytes([data[6], data[7]]);
        (maj as u8, min as u8, 0)
    } else {
        (0, 0, 0)
    };

    // Numéro de série : pour V3+ à l'offset 0x1A (14 bytes hex ASCII)
    let serial = if md_version >= 6 && data.len() >= 0x1A + 14 {
        String::from_utf8_lossy(&data[0x1A..0x1A + 14]).to_string()
    } else {
        // V1/V2 : bytes 2-9 en hex
        data[2..data.len().min(10)]
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>()
    };

    LuniiDeviceInfo { hw_version, fw_major, fw_minor, fw_subminor, serial }
}

fn detect_image_format(path: &Path) -> Option<&'static str> {
    use std::io::Read;
    let mut buf = [0u8; 8];
    let n = fs::File::open(path).ok()?.read(&mut buf).ok()?;
    if n < 2 { return None; }
    if buf[0] == 0x89 && buf[1] == b'P' { return Some("png"); }
    if buf[0] == 0xFF && buf[1] == 0xD8 { return Some("jpg"); }
    if buf[0] == b'B' && buf[1] == b'M' { return Some("bmp"); }
    None
}

/// Cherche une image de couverture dans le dossier d'une story.
/// Priorité : li/ri (Lunii natif), puis PNG/JPG/BMP dans assets/ et racine.
fn find_cover_image(story_dir: &Path) -> Option<String> {
    // 1. Fichiers Lunii natifs sans extension (li = list image, ri = root image)
    for name in &["li", "ri"] {
        let p = story_dir.join(name);
        if p.is_file() && detect_image_format(&p).is_some() {
            return Some(p.to_string_lossy().into_owned());
        }
    }

    // 2. Fichiers image avec extension dans : racine, assets/, rf/
    let search_dirs = [
        story_dir.to_path_buf(),
        story_dir.join("assets"),
        story_dir.join("rf"),
    ];
    const EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp"];

    for dir in &search_dirs {
        if !dir.is_dir() { continue; }
        // Noms courants en premier
        for name in &["cover", "thumbnail", "0", "image"] {
            for ext in EXTS {
                let p = dir.join(format!("{}.{}", name, ext));
                if p.exists() { return Some(p.to_string_lossy().into_owned()); }
            }
        }
        // Tous les fichiers image du dossier
        if let Ok(entries) = fs::read_dir(dir) {
            let mut images: Vec<_> = entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path().extension()
                        .and_then(|x| x.to_str())
                        .map(|x| EXTS.contains(&x.to_lowercase().as_str()))
                        .unwrap_or(false)
                })
                .map(|e| e.path())
                .collect();
            images.sort();
            if let Some(p) = images.into_iter().next() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }

    // 3. Scan exhaustif : tout fichier sans extension reconnu comme image
    if let Ok(entries) = fs::read_dir(story_dir) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if p.is_file() && p.extension().is_none() {
                if detect_image_format(&p).is_some() {
                    return Some(p.to_string_lossy().into_owned());
                }
            }
        }
    }

    None
}

/// Supprime le suffixe de hash `_XXXXXXXX` (underscore + 8 chiffres hex) si présent.
fn strip_hash_suffix(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() > 9 {
        let tail = &s[s.len() - 9..];
        if tail.starts_with('_') && tail[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return &s[..s.len() - 9];
        }
    }
    s
}

/// Retourne true si la chaîne ressemble à un UUID Lunii (hex + tirets, pas un titre lisible).
fn looks_like_uuid(s: &str) -> bool {
    let stripped = s.replace('-', "");
    stripped.len() >= 16 && stripped.chars().all(|c| c.is_ascii_hexdigit())
}

/// Tente de lire un titre lisible depuis le dossier d'une story.
/// Cherche (dans l'ordre) : sidecar storyId, story.json/title.json, titre.txt.
/// Filtre les UUID Lunii officiels qui ne sont pas des titres humains.
fn read_story_title(story_dir: &Path, sidecar: &Option<SidecarData>) -> Option<String> {
    // 1. Depuis le sidecar LuniiSync (priorité absolue)
    if let Some(sc) = sidecar {
        if !sc.story_id.is_empty() && !looks_like_uuid(&sc.story_id) {
            let name = strip_hash_suffix(&sc.story_id).replace('_', " ");
            return Some(name);
        }
    }
    // 2. Depuis story.json / title.json / metadata.json
    for filename in &["story.json", "title.json", "metadata.json"] {
        if let Ok(text) = fs::read_to_string(story_dir.join(filename)) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(t) = val.get("title").and_then(|v| v.as_str()) {
                    if !t.is_empty() && !looks_like_uuid(t) {
                        return Some(t.to_string());
                    }
                }
            }
        }
    }
    // 3. Depuis titre.txt / title.txt
    for filename in &["titre.txt", "title.txt"] {
        if let Ok(text) = fs::read_to_string(story_dir.join(filename)) {
            let t = text.trim().to_string();
            if !t.is_empty() && !looks_like_uuid(&t) {
                return Some(t);
            }
        }
    }
    None
}

/// Lit l'inventaire complet depuis `.content/` sur un device monté.
pub fn read_inventory(mount: &Path) -> Option<LuniiInventory> {
    let content_dir = mount.join(".content");
    if !content_dir.is_dir() {
        return None;
    }

    let order_map: HashMap<String, usize> = read_pack_index_entries(&mount.join(".pi"))
        .ok()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(idx, entry)| (short_uuid_from_uuid_bytes(&entry), idx))
        .collect();

    let mut dir_entries: Vec<_> = fs::read_dir(&content_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    dir_entries.sort_by(|a, b| {
        let a_short = a.file_name().to_string_lossy().to_uppercase().to_string();
        let b_short = b.file_name().to_string_lossy().to_uppercase().to_string();
        let a_idx = order_map.get(&a_short).copied().unwrap_or(usize::MAX);
        let b_idx = order_map.get(&b_short).copied().unwrap_or(usize::MAX);
        a_idx.cmp(&b_idx).then_with(|| a.path().cmp(&b.path()))
    });

    let stories: Vec<LuniiStoryEntry> = dir_entries
        .iter()
        .filter_map(|e| {
            let short_uuid = e.file_name().to_string_lossy().to_uppercase().to_string();
            if short_uuid.is_empty() {
                return None;
            }
            let sidecar = read_sidecar(&e.path());
            let title = read_story_title(&e.path(), &sidecar);
            let cover_path = find_cover_image(&e.path());
            let size_bytes = dir_size_bytes(&e.path());
            Some(LuniiStoryEntry { short_uuid, sidecar, title, cover_path, size_bytes })
        })
        .collect();

    let managed_stories = stories.iter().filter(|s| s.sidecar.is_some()).count();
    let total_stories = stories.len();

    Some(LuniiInventory {
        mount: mount.to_string_lossy().into_owned(),
        stories,
        total_stories,
        managed_stories,
    })
}

/// Détecte le device et retourne un inventaire discriminé.
pub fn get_lunii_inventory() -> LuniiInventoryResult {
    let probe = probe_platform();

    let mount_str = match probe.mount {
        Some(m) if probe.connected => m,
        _ => {
            return LuniiInventoryResult {
                status: InventoryStatus::NotConnected,
                mount: None,
                stories: vec![],
                total_stories: 0,
                managed_stories: 0,
                error: None,
            }
        }
    };

    let content_dir = Path::new(&mount_str).join(".content");
    if !content_dir.is_dir() {
        return LuniiInventoryResult {
            status: InventoryStatus::NoContentDir,
            mount: Some(mount_str),
            stories: vec![],
            total_stories: 0,
            managed_stories: 0,
            error: None,
        };
    }

    match read_inventory(Path::new(&mount_str)) {
        Some(inv) => LuniiInventoryResult {
            status: InventoryStatus::Ok,
            mount: Some(mount_str),
            stories: inv.stories,
            total_stories: inv.total_stories,
            managed_stories: inv.managed_stories,
            error: None,
        },
        None => LuniiInventoryResult {
            status: InventoryStatus::ReadError,
            mount: Some(mount_str),
            stories: vec![],
            total_stories: 0,
            managed_stories: 0,
            error: Some("Lecture de .content/ échouée — permissions ou erreur I/O".to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compare_story, move_story_in_pack_index, probe_root, read_inventory,
        read_sidecar, repair_pack_index_native, reorder_story_in_pack_index,
        write_pack_index_entries,
        LuniiDeviceProbe, LuniiStoryEntry, SidecarData, StoryDeviceStatus,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock drift")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn connected_probe(root: &Path) -> LuniiDeviceProbe {
        probe_root(root, 0).expect("expected a connected Lunii probe")
    }

    #[test]
    fn probe_root_detects_marker_and_counts_story_dirs() {
        let root = TempDir::new("lunii-probe-marker");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(mount.join(".content").join("A1B2C3D4")).unwrap();
        fs::create_dir_all(mount.join(".content").join("E5F6G7H8")).unwrap();
        fs::write(mount.join(".md"), b"marker").unwrap();

        let probe = connected_probe(root.path());
        assert!(probe.connected);
        assert_eq!(probe.mount, Some(mount.to_string_lossy().into_owned()));
        assert!(probe.marker_found);
        assert!(probe.content_dir_present);
        assert_eq!(probe.story_dir_count, 2);
        assert_eq!(probe.detection_method.as_deref(), Some("marker"));
    }

    #[test]
    fn probe_root_falls_back_to_candidate_volume_name() {
        let root = TempDir::new("lunii-probe-name");
        let mount = root.path().join("Ma LUNII");
        fs::create_dir_all(&mount).unwrap();

        let probe = connected_probe(root.path());
        assert!(probe.connected);
        assert_eq!(probe.mount, Some(mount.to_string_lossy().into_owned()));
        assert!(!probe.marker_found);
        assert_eq!(probe.detection_method.as_deref(), Some("volume-name"));
    }

    #[test]
    fn probe_root_ignores_unrelated_volumes() {
        let root = TempDir::new("lunii-probe-ignore");
        fs::create_dir_all(root.path().join("Macintosh HD")).unwrap();
        assert_eq!(probe_root(root.path(), 0), None);
    }

    #[test]
    fn inventory_none_when_no_content_dir() {
        let root = TempDir::new("lunii-inv-no-content");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(&mount).unwrap();
        assert!(read_inventory(&mount).is_none());
    }

    #[test]
    fn inventory_stories_without_sidecar() {
        let root = TempDir::new("lunii-inv-no-sidecar");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(mount.join(".content").join("AABBCCDD")).unwrap();
        fs::create_dir_all(mount.join(".content").join("11223344")).unwrap();

        let inv = read_inventory(&mount).expect("inventory should exist");
        assert_eq!(inv.total_stories, 2);
        assert_eq!(inv.managed_stories, 0);
        assert!(inv.stories.iter().all(|s| s.sidecar.is_none()));
        let uuids: Vec<_> = inv.stories.iter().map(|s| s.short_uuid.as_str()).collect();
        assert!(uuids.contains(&"AABBCCDD"));
        assert!(uuids.contains(&"11223344"));
    }

    #[test]
    fn inventory_stories_with_valid_sidecar() {
        let root = TempDir::new("lunii-inv-sidecar");
        let mount = root.path().join("LUNII");
        let story_dir = mount.join(".content").join("DEADBEEF");
        fs::create_dir_all(&story_dir).unwrap();
        fs::write(
            story_dir.join(".lunii-studio.json"),
            r#"{"story_id":"abc-123","hash":"sha256:deadbeef","pushed_at":"2024-01-01T00:00:00Z","source":"lunii-studio"}"#,
        ).unwrap();

        let inv = read_inventory(&mount).expect("inventory");
        assert_eq!(inv.total_stories, 1);
        assert_eq!(inv.managed_stories, 1);
        let sc = inv.stories[0].sidecar.as_ref().expect("sidecar");
        assert_eq!(sc.story_id, "abc-123");
        assert_eq!(sc.hash, "sha256:deadbeef");
    }

    #[test]
    fn inventory_respects_pack_index_order() {
        let root = TempDir::new("lunii-inv-order");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(mount.join(".content").join("AABBCCDD")).unwrap();
        fs::create_dir_all(mount.join(".content").join("11223344")).unwrap();

        let mut first = [0u8; 16];
        first[12..].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        let mut second = [0u8; 16];
        second[12..].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        write_pack_index_entries(&mount.join(".pi"), &[first, second]).unwrap();

        let inv = read_inventory(&mount).expect("inventory");
        let ordered: Vec<_> = inv.stories.iter().map(|s| s.short_uuid.as_str()).collect();
        assert_eq!(ordered, vec!["11223344", "AABBCCDD"]);
    }

    #[test]
    fn move_story_updates_pack_index_order() {
        let root = TempDir::new("lunii-move-order");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(&mount).unwrap();

        let mut first = [0u8; 16];
        first[12..].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let mut second = [0u8; 16];
        second[12..].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        let pi_path = mount.join(".pi");
        write_pack_index_entries(&pi_path, &[first, second]).unwrap();

        move_story_in_pack_index(&mount.to_string_lossy(), "11223344", -1).unwrap();

        let entries = super::read_pack_index_entries(&pi_path).unwrap();
        let ordered: Vec<_> = entries
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect();
        assert_eq!(ordered, vec!["11223344", "AABBCCDD"]);
    }

    #[test]
    fn reorder_story_rewrites_visible_index_and_preserves_hidden_index() {
        let root = TempDir::new("lunii-reorder-order");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(&mount).unwrap();

        let mut first = [0u8; 16];
        first[12..].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let mut second = [0u8; 16];
        second[12..].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        let mut third = [0u8; 16];
        third[12..].copy_from_slice(&[0x55, 0x66, 0x77, 0x88]);
        let mut hidden = [0u8; 16];
        hidden[12..].copy_from_slice(&[0x99, 0xAA, 0xBB, 0xCC]);

        let pi_path = mount.join(".pi");
        let pi_hidden_path = mount.join(".pi.hidden");
        write_pack_index_entries(&pi_path, &[first, second, third]).unwrap();
        write_pack_index_entries(&pi_hidden_path, &[hidden]).unwrap();

        reorder_story_in_pack_index(&mount.to_string_lossy(), "55667788", 0).unwrap();

        let visible_entries = super::read_pack_index_entries(&pi_path).unwrap();
        let visible_order: Vec<_> = visible_entries
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect();
        assert_eq!(visible_order, vec!["55667788", "AABBCCDD", "11223344"]);

        let hidden_entries = super::read_pack_index_entries(&pi_hidden_path).unwrap();
        let hidden_order: Vec<_> = hidden_entries
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect();
        assert_eq!(hidden_order, vec!["99AABBCC"]);
    }

    #[test]
    fn repair_pack_index_rebuilds_visible_entries_from_content_and_preserves_hidden() {
        let root = TempDir::new("lunii-repair-index");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(mount.join(".content").join("11223344")).unwrap();
        fs::create_dir_all(mount.join(".content").join("AABBCCDD")).unwrap();
        fs::create_dir_all(mount.join(".content").join("55667788")).unwrap();

        let mut visible = [0u8; 16];
        visible[12..].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let mut hidden = [0u8; 16];
        hidden[12..].copy_from_slice(&[0x55, 0x66, 0x77, 0x88]);

        write_pack_index_entries(&mount.join(".pi"), &[visible]).unwrap();
        write_pack_index_entries(&mount.join(".pi.hidden"), &[hidden]).unwrap();

        repair_pack_index_native(&mount.to_string_lossy()).unwrap();

        let visible_entries = super::read_pack_index_entries(&mount.join(".pi")).unwrap();
        let visible_order: Vec<_> = visible_entries
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect();
        assert_eq!(visible_order, vec!["AABBCCDD", "11223344"]);

        let hidden_entries = super::read_pack_index_entries(&mount.join(".pi.hidden")).unwrap();
        let hidden_order: Vec<_> = hidden_entries
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect();
        assert_eq!(hidden_order, vec!["55667788"]);
    }

    #[test]
    fn repair_pack_index_ignores_missing_and_invalid_existing_entries() {
        let root = TempDir::new("lunii-repair-index-invalid");
        let mount = root.path().join("LUNII");
        fs::create_dir_all(mount.join(".content").join("CAFEBABE")).unwrap();
        fs::create_dir_all(mount.join(".content").join("DEADBEEF")).unwrap();
        fs::create_dir_all(mount.join(".content").join("bonjour")).unwrap();

        let mut stale = [0u8; 16];
        stale[12..].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        let mut kept = [0u8; 16];
        kept[12..].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        write_pack_index_entries(&mount.join(".pi"), &[stale, kept]).unwrap();
        fs::write(mount.join(".pi.hidden"), b"broken").unwrap();

        repair_pack_index_native(&mount.to_string_lossy()).unwrap();

        let visible_entries = super::read_pack_index_entries(&mount.join(".pi")).unwrap();
        let visible_order: Vec<_> = visible_entries
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect();
        assert_eq!(visible_order, vec!["DEADBEEF", "CAFEBABE"]);

        let hidden_entries = super::read_pack_index_entries(&mount.join(".pi.hidden")).unwrap();
        assert!(hidden_entries.is_empty());
    }

    #[test]
    fn inventory_skips_sidecar_with_invalid_json() {
        let root = TempDir::new("lunii-inv-bad-json");
        let mount = root.path().join("LUNII");
        let story_dir = mount.join(".content").join("BADBADBAD");
        fs::create_dir_all(&story_dir).unwrap();
        fs::write(story_dir.join(".lunii-studio.json"), b"not json {{{").unwrap();

        let inv = read_inventory(&mount).expect("inventory");
        assert_eq!(inv.managed_stories, 0);
        assert!(inv.stories[0].sidecar.is_none());
    }

    #[test]
    fn inventory_skips_sidecar_with_wrong_source() {
        let root = TempDir::new("lunii-inv-wrong-source");
        let mount = root.path().join("LUNII");
        let story_dir = mount.join(".content").join("CAFECAFE");
        fs::create_dir_all(&story_dir).unwrap();
        fs::write(
            story_dir.join(".lunii-studio.json"),
            r#"{"story_id":"x","hash":"sha256:x","pushed_at":"2024-01-01T00:00:00Z","source":"other-tool"}"#,
        ).unwrap();

        let inv = read_inventory(&mount).expect("inventory");
        assert_eq!(inv.managed_stories, 0);
        assert!(inv.stories[0].sidecar.is_none());
    }

    #[test]
    fn read_sidecar_returns_none_when_file_absent() {
        let root = TempDir::new("lunii-sidecar-absent");
        let story_dir = root.path().join("AABB1122");
        fs::create_dir_all(&story_dir).unwrap();
        assert!(read_sidecar(&story_dir).is_none());
    }

    fn make_entry(short_uuid: &str, story_id: &str, hash: &str) -> LuniiStoryEntry {
        LuniiStoryEntry {
            short_uuid: short_uuid.to_string(),
            sidecar: Some(SidecarData {
                story_id: story_id.to_string(),
                hash: hash.to_string(),
                pushed_at: "2024-01-01T00:00:00Z".to_string(),
                source: "lunii-studio".to_string(),
            }),
            title: Some(story_id.replace('_', " ")),
            cover_path: None,
            size_bytes: 0,
        }
    }

    fn make_unmanaged(short_uuid: &str) -> LuniiStoryEntry {
        LuniiStoryEntry { short_uuid: short_uuid.to_string(), sidecar: None, title: None, cover_path: None, size_bytes: 0 }
    }

    #[test]
    fn compare_not_on_device() {
        let stories = vec![make_entry("AABBCCDD", "other-story", "sha256:aaa")];
        let result = compare_story("my-story", None, &stories);
        assert_eq!(result.status, StoryDeviceStatus::NotOnDevice);
        assert!(result.device_short_uuid.is_none());
    }

    #[test]
    fn compare_present_when_no_local_hash() {
        let stories = vec![make_entry("AABBCCDD", "my-story", "sha256:abc")];
        let result = compare_story("my-story", None, &stories);
        assert_eq!(result.status, StoryDeviceStatus::Present);
        assert_eq!(result.device_short_uuid.as_deref(), Some("AABBCCDD"));
    }

    #[test]
    fn compare_up_to_date_when_hashes_match() {
        let stories = vec![make_entry("DEADBEEF", "my-story", "sha256:matching")];
        let result = compare_story("my-story", Some("sha256:matching"), &stories);
        assert_eq!(result.status, StoryDeviceStatus::UpToDate);
    }

    #[test]
    fn compare_outdated_when_hashes_differ() {
        let stories = vec![make_entry("DEADBEEF", "my-story", "sha256:old")];
        let result = compare_story("my-story", Some("sha256:new"), &stories);
        assert_eq!(result.status, StoryDeviceStatus::Outdated);
        assert_eq!(result.device_hash.as_deref(), Some("sha256:old"));
    }

    #[test]
    fn compare_ignores_entries_without_sidecar() {
        let stories = vec![make_unmanaged("OFFICIAL1"), make_unmanaged("OFFICIAL2")];
        let result = compare_story("any-story", None, &stories);
        assert_eq!(result.status, StoryDeviceStatus::NotOnDevice);
    }
}
