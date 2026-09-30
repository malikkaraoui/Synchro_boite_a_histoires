//! Détection et inventaire natif d'une boîte boîte à histoires montée en USB.
//! Porté depuis la-forge-a-histoires/tauri/src-tauri/src/storybox_device.rs
//! + ajout du fallback détection par nom de volume.

use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// État d'une boîte vue par la détection.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    NotConnected,
    /// Boîte présente (`stat` autorisé) mais illisible : sous sandbox, l'utilisateur doit
    /// l'autoriser une fois dans le NSOpenPanel.
    AccessRequired,
    /// Boîte présente et `.md` lisible.
    Connected,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoryBoxDeviceProbe {
    pub state: DeviceState,
    /// Vrai seulement si la boîte est lisible (`state == Connected`).
    pub connected: bool,
    pub mount: Option<String>,
    pub device_id: Option<String>,
    pub marker_found: bool,
    pub content_dir_present: bool,
    pub story_dir_count: usize,
    pub detection_method: Option<String>,
}

/// Métadonnées du sidecar `.la-forge-a-histoires.json` écrit par Studio au push.
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
pub struct StoryBoxStoryEntry {
    pub short_uuid: String,
    /// Présent uniquement si `.la-forge-a-histoires.json` valide existe.
    pub sidecar: Option<SidecarData>,
    /// Titre lisible : depuis le sidecar, story.json ou titre.txt.
    pub title: Option<String>,
    /// Chemin absolu de l'image de couverture si trouvée dans le dossier story.
    pub cover_path: Option<String>,
    /// Taille totale du dossier story en octets.
    pub size_bytes: u64,
}

/// Inventaire complet d'un device boîte à histoires monté.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBoxInventory {
    pub mount: String,
    pub stories: Vec<StoryBoxStoryEntry>,
    pub total_stories: usize,
    pub managed_stories: usize,
}

/// Discriminant explicite du résultat `get_storybox_inventory`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InventoryStatus {
    NotConnected,
    NoContentDir,
    ReadError,
    Ok,
}

/// Résultat toujours retourné de `get_storybox_inventory` — jamais None.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBoxInventoryResult {
    pub status: InventoryStatus,
    pub mount: Option<String>,
    pub stories: Vec<StoryBoxStoryEntry>,
    pub total_stories: usize,
    pub managed_stories: usize,
    pub error: Option<String>,
}

/// Informations matérielles et firmware lues depuis `.md`.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoryBoxDeviceInfo {
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
    stories: &[StoryBoxStoryEntry],
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

/// Vérifie si une story Studio est présente sur la boîte validée, avec comparaison de hash.
pub fn check_story_on_device(
    mount: Option<&str>,
    story_id: String,
    local_hash: Option<String>,
) -> Option<StoryCompareResult> {
    let result = get_storybox_inventory(mount);
    if result.status != InventoryStatus::Ok {
        return None;
    }
    Some(compare_story(&story_id, local_hash.as_deref(), &result.stories))
}

impl StoryBoxDeviceProbe {
    fn disconnected() -> Self {
        Self {
            state: DeviceState::NotConnected,
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
        let device_id = read_device_id(&mount);

        Self {
            state: DeviceState::Connected,
            connected: true,
            mount: Some(mount.to_string_lossy().into_owned()),
            device_id,
            marker_found,
            content_dir_present: content_dir.is_dir(),
            story_dir_count,
            detection_method: Some(detection_method.to_string()),
        }
    }

    /// Boîte vue mais illisible : rien n'est lu, seul le point de montage est renvoyé.
    fn access_required(mount: PathBuf, detection_method: &str, marker_found: bool) -> Self {
        Self {
            state: DeviceState::AccessRequired,
            connected: false,
            mount: Some(mount.to_string_lossy().into_owned()),
            device_id: None,
            marker_found,
            content_dir_present: false,
            story_dir_count: 0,
            detection_method: Some(detection_method.to_string()),
        }
    }
}

/// Identité de la boîte montée en `mount`, relue sur le matériel à chaque appel :
/// le numéro de série (stable) en priorité, sinon un identifiant dérivé du montage.
pub fn read_device_id(mount: &Path) -> Option<String> {
    let mount_str = mount.to_str().unwrap_or("");
    let info = read_device_info(mount_str);
    if !info.serial.is_empty() && info.serial != "000000000000000000" {
        Some(format!("serial-{}", info.serial))
    } else {
        get_volume_id(mount_str)
    }
}

/// Retourne un identifiant stable pour le volume monté.
/// Sandbox App Store : pas de `diskutil`, on dérive depuis le chemin de montage.
/// L'identification primaire est faite ailleurs via le serial-number lu sur le device.
pub fn get_volume_id(mount: &str) -> Option<String> {
    Some(format!("vol-{}", mount.replace('/', "_")))
}

/// Fichiers que macOS pose de lui-même : `._<nom>` (AppleDouble, sur FAT/exFAT, à côté de
/// chaque fichier ou dossier écrit) et `.DS_Store`. Jamais une histoire, un audio ni une image.
pub(crate) fn is_macos_metadata(name: &str) -> bool {
    name.starts_with("._") || name == ".DS_Store"
}

fn dir_size_bytes(path: &Path) -> u64 {
    match fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| !is_macos_metadata(&e.file_name().to_string_lossy()))
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

/// Un dossier caché de `.content/` n'est jamais une histoire : transit d'un import en cours
/// ou interrompu (`.<SHORT>.tmp`, `.<SHORT>.old`), ou dossier système.
fn is_hidden_entry(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

/// Dossiers d'histoires de `root` (`.content/` ou `.content.hidden/`) : UUID court en
/// majuscules → chemin réel. Les entrées cachées (transit, AppleDouble `._*`) sont ignorées.
fn collect_story_dirs(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    Ok(fs::read_dir(root)
        .map_err(|e| format!("Lecture {:?} échouée : {e}", root))?
        .filter_map(Result::ok)
        .filter(|entry| !is_hidden_entry(entry) && entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_uppercase();
            is_short_uuid_dir_name(&name).then(|| (name, entry.path()))
        })
        .collect())
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

/// Copie brute de `.pi` et `.pi.hidden`, remise à l'identique si une mise à jour échoue.
/// Un fichier illisible au moment de la copie n'est pas touché à la restauration.
pub struct PackIndexSnapshot {
    mount: PathBuf,
    /// `None` : illisible ; `Some(None)` : absent ; `Some(Some(octets))` : contenu.
    files: [(&'static str, Option<Option<Vec<u8>>>); 2],
}

impl PackIndexSnapshot {
    pub fn take(mount: &Path) -> Self {
        let read = |name: &str| match fs::read(mount.join(name)) {
            Ok(data) => Some(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(None),
            Err(_) => None,
        };
        Self {
            mount: mount.to_path_buf(),
            files: [(".pi", read(".pi")), (".pi.hidden", read(".pi.hidden"))],
        }
    }

    pub fn restore(&self) -> Result<(), String> {
        for (name, saved) in &self.files {
            let path = self.mount.join(name);
            match saved {
                Some(Some(data)) => fs::write(&path, data)
                    .map_err(|e| format!("Restauration {name} échouée : {e}"))?,
                Some(None) if path.exists() => fs::remove_file(&path)
                    .map_err(|e| format!("Retrait {name} échoué : {e}"))?,
                _ => {}
            }
        }
        Ok(())
    }
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

/// Fichiers qu'exige la référence pour rattacher un dossier à l'index (`__valid_story`).
const REFERENCE_STORY_FILES: [&str; 4] = ["ni", "li", "ri", "si"];
const SIDECAR_FILE: &str = ".la-forge-a-histoires.json";

/// Histoire complète : `ni`, `li`, `ri`, `si` (critère de la référence, qui régénère `bt` en V2
/// et s'en passe en V3), plus `bt` si le dossier porte le sidecar, c'est-à-dire s'il a été écrit
/// par l'app, qui écrit toujours `bt`.
pub(crate) fn is_complete_story_dir(story_dir: &Path) -> bool {
    REFERENCE_STORY_FILES.iter().all(|f| story_dir.join(f).is_file())
        && (!story_dir.join(SIDECAR_FILE).exists() || story_dir.join("bt").is_file())
}

/// Entrées d'un index pour la réparation, et signalement éventuel :
/// - absent → vide (reconstruit depuis les dossiers) ;
/// - taille non multiple de 16 (écriture coupée) → les entrées entières du début, dans leur
///   ordre ; une entrée sans dossier est retirée ensuite, comme pour tout index abîmé (R004 P5b) ;
/// - illisible → erreur, pour ne rien retirer sur une lecture ratée : l'import échoue fermé, et
///   le message nomme la sortie (« Réparer l'index », qui met l'index de côté d'abord).
fn read_pack_index_for_repair(index_path: &Path) -> Result<(Vec<[u8; 16]>, Option<String>), String> {
    let name = index_path.file_name().unwrap_or_default().to_string_lossy();
    let data = match fs::read(index_path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((Vec::new(), None)),
        Err(e) => return Err(format!(
            "L'index de la boîte est illisible ({name} : {e}). Utilisez « Réparer l'index » pour le reconstruire \
             (l'ancien sera gardé en {name}.bak). Index non modifié."
        )),
    };
    let entries: Vec<[u8; 16]> = data.chunks_exact(16).map(|c| c.try_into().unwrap()).collect();
    let notice = (data.len() % 16 != 0).then(|| format!(
        "L'index {name} était tronqué : {} entrée(s) entière(s) gardée(s) dans leur ordre, le reste est reconstruit à partir des histoires présentes",
        entries.len()
    ));
    Ok((entries, notice))
}

/// « Réparer l'index » seulement (jamais pendant un import) : un index illisible (erreur
/// d'E/S, dossier à sa place) est renommé en `<nom>.bak`, puis la réparation le reconstruit à
/// partir des dossiers. Renvoie les signalements pour le journal.
pub(crate) fn set_aside_unreadable_pack_indexes(mount: &Path) -> Result<Vec<String>, String> {
    let mut notices = Vec::new();
    for name in [".pi", ".pi.hidden"] {
        let path = mount.join(name);
        let Err(e) = fs::read(&path) else { continue };
        if e.kind() == std::io::ErrorKind::NotFound {
            continue;
        }
        fs::rename(&path, mount.join(format!("{name}.bak")))
            .map_err(|r| format!("Mise de côté de l'index illisible {name} échouée : {r}. Index non modifié."))?;
        notices.push(format!(
            "L'index {name} était illisible ({e}) : gardé en {name}.bak, puis reconstruit à partir des histoires présentes (ordre d'origine perdu)"
        ));
    }
    Ok(notices)
}

/// Résultat de la réparation de `.pi`.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackIndexRepair {
    /// Histoires indexées (`.pi` et `.pi.hidden`).
    pub indexed: usize,
    /// Dossiers incomplets : gardés dans l'index s'ils y étaient, jamais ajoutés, jamais
    /// supprimés automatiquement.
    pub incomplete: Vec<String>,
    /// Restes d'un import interrompu traités ou laissés avant la réparation (signalements).
    pub leftovers: Vec<String>,
    /// Index tronqué ou illisible mis de côté : signalements pour le journal.
    pub notices: Vec<String>,
}

/// Répare `.pi` et `.pi.hidden` avec le critère de la référence (`recover_stories`) :
/// - une entrée dont le dossier existe (`.content/` ou `.content.hidden/`) est **gardée**, à sa
///   place, même si le dossier est incomplet ;
/// - une entrée dont le dossier n'existe plus est retirée ;
/// - un dossier absent des deux index n'est ajouté que s'il est complet (`is_complete_story_dir`),
///   à `.pi` depuis `.content/`, à `.pi.hidden` depuis `.content.hidden/`.
/// Format des entrées inchangé (UUID court, cf. NATIVE_IMPORT.md).
pub fn repair_pack_index_native(mount: &str) -> Result<PackIndexRepair, String> {
    let mount_path = Path::new(mount);
    if !mount_path.is_dir() {
        return Err(format!("Montage boîte à histoires introuvable : {mount}"));
    }

    let content_dir = mount_path.join(".content");
    if !content_dir.is_dir() {
        return Err("Dossier .content introuvable sur la boîte".to_string());
    }
    let visible_dirs = collect_story_dirs(&content_dir)?;
    let hidden_content_dir = mount_path.join(".content.hidden");
    let hidden_dirs = if hidden_content_dir.exists() {
        collect_story_dirs(&hidden_content_dir)?
    } else {
        BTreeMap::new()
    };
    let exists = |short_uuid: &String| visible_dirs.contains_key(short_uuid) || hidden_dirs.contains_key(short_uuid);

    let (visible_entries, visible_notice) = read_pack_index_for_repair(&mount_path.join(".pi"))?;
    let (hidden_entries, hidden_notice) = read_pack_index_for_repair(&mount_path.join(".pi.hidden"))?;

    // Entrées gardées, dans leur ordre ; une entrée à la fois dans les deux index reste cachée.
    let mut indexed: HashSet<String> = HashSet::new();
    let mut keep = |entries: &[[u8; 16]]| -> Vec<String> {
        entries
            .iter()
            .map(short_uuid_from_uuid_bytes)
            .filter(|short_uuid| exists(short_uuid) && indexed.insert(short_uuid.clone()))
            .collect()
    };
    let mut hidden_short_uuids = keep(&hidden_entries);
    let mut visible_short_uuids = keep(&visible_entries);

    // Ajouts : seulement des dossiers complets, absents des deux index.
    for (dirs, out) in [(&visible_dirs, &mut visible_short_uuids), (&hidden_dirs, &mut hidden_short_uuids)] {
        for (short_uuid, path) in dirs {
            if !indexed.contains(short_uuid) && is_complete_story_dir(path) {
                indexed.insert(short_uuid.clone());
                out.push(short_uuid.clone());
            }
        }
    }

    let mut incomplete: Vec<String> = visible_dirs
        .iter()
        .chain(hidden_dirs.iter())
        .filter(|(_, path)| !is_complete_story_dir(path))
        .map(|(short_uuid, _)| short_uuid.clone())
        .collect();
    incomplete.sort();
    incomplete.dedup();

    let visible_uuid_entries = visible_short_uuids
        .iter()
        .map(|short_uuid| short_uuid_to_uuid_bytes(short_uuid))
        .collect::<Result<Vec<_>, _>>()?;
    let hidden_uuid_entries = hidden_short_uuids
        .iter()
        .map(|short_uuid| short_uuid_to_uuid_bytes(short_uuid))
        .collect::<Result<Vec<_>, _>>()?;

    write_all_pack_index_entries(mount_path, &visible_uuid_entries, &hidden_uuid_entries)?;
    Ok(PackIndexRepair {
        indexed: visible_uuid_entries.len() + hidden_uuid_entries.len(),
        incomplete,
        leftovers: Vec::new(),
        notices: visible_notice.into_iter().chain(hidden_notice).collect(),
    })
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
            .filter(|e| !is_hidden_entry(e) && e.path().is_dir())
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

fn probe_mount_candidate(path: &Path) -> Option<StoryBoxDeviceProbe> {
    // Méthode 1 : fichier marqueur `.md` à la racine (boîte à histoires officiel).
    // `exists()` n'est qu'un `stat`, autorisé sous sandbox même sans accès : la boîte n'est
    // « connectée » que si `.md` se lit réellement.
    let marker = path.join(".md");
    if marker.exists() {
        return Some(if fs::read(&marker).is_ok() {
            StoryBoxDeviceProbe::connected(path.to_path_buf(), "marker", true)
        } else {
            StoryBoxDeviceProbe::access_required(path.to_path_buf(), "marker", true)
        });
    }

    // Méthode 2 : nom de volume contient "STORYBOX" (fallback macOS/Windows)
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    if name.contains("STORYBOX") {
        return Some(if fs::read_dir(path).is_ok() {
            StoryBoxDeviceProbe::connected(path.to_path_buf(), "volume-name", false)
        } else {
            StoryBoxDeviceProbe::access_required(path.to_path_buf(), "volume-name", false)
        });
    }

    None
}

fn probe_root(root: &Path, nested_levels: usize) -> Option<StoryBoxDeviceProbe> {
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
fn probe_platform() -> StoryBoxDeviceProbe {
    probe_root(Path::new("/Volumes"), 0).unwrap_or_else(StoryBoxDeviceProbe::disconnected)
}

#[cfg(target_os = "linux")]
fn probe_platform() -> StoryBoxDeviceProbe {
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
    StoryBoxDeviceProbe::disconnected()
}

#[cfg(target_os = "windows")]
fn probe_platform() -> StoryBoxDeviceProbe {
    for letter in b'A'..=b'Z' {
        let mount = PathBuf::from(format!("{}:\\", letter as char));
        if !mount.exists() {
            continue;
        }
        if let Some(probe) = probe_mount_candidate(&mount) {
            return probe;
        }
    }
    StoryBoxDeviceProbe::disconnected()
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn probe_platform() -> StoryBoxDeviceProbe {
    StoryBoxDeviceProbe::disconnected()
}

pub fn probe_storybox_device() -> StoryBoxDeviceProbe {
    probe_platform()
}

/// Sonde un point de montage connu (boîte validée ou choisie), sans parcourir `/Volumes`.
pub fn probe_mount(mount: &Path) -> StoryBoxDeviceProbe {
    probe_mount_candidate(mount).unwrap_or_else(StoryBoxDeviceProbe::disconnected)
}

/// Parse `.la-forge-a-histoires.json` depuis un dossier story.
/// Accepte les sidecars écrits par Synchro Boîte à histoires ("synchro_boite_a_histoires") et la-forge-a-histoires ("la-forge-a-histoires").
fn read_sidecar(story_dir: &Path) -> Option<SidecarData> {
    let text = fs::read_to_string(story_dir.join(".la-forge-a-histoires.json")).ok()?;
    let val: serde_json::Value = serde_json::from_str(&text).ok()?;
    let source = val.get("source").and_then(|v| v.as_str()).unwrap_or("");
    if source != "la-forge-a-histoires" && source != "synchro_boite_a_histoires" {
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
/// Lit les infos matérielles/firmware depuis le fichier `.md` d'une boîte à histoires.
pub fn read_device_info(mount: &str) -> StoryBoxDeviceInfo {
    let path = Path::new(mount).join(".md");
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(_) => return StoryBoxDeviceInfo::default(),
    };
    if data.len() < 32 {
        return StoryBoxDeviceInfo::default();
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

    StoryBoxDeviceInfo { hw_version, fw_major, fw_minor, fw_subminor, serial }
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
/// Priorité : li/ri (boîte à histoires natif), puis PNG/JPG/BMP dans assets/ et racine.
fn find_cover_image(story_dir: &Path) -> Option<String> {
    // 1. Fichiers boîte à histoires natifs sans extension (li = list image, ri = root image)
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
                .filter(|e| !is_macos_metadata(&e.file_name().to_string_lossy()))
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
            if is_macos_metadata(&entry.file_name().to_string_lossy()) {
                continue;
            }
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

/// Retourne true si la chaîne ressemble à un UUID boîte à histoires (hex + tirets, pas un titre lisible).
fn looks_like_uuid(s: &str) -> bool {
    let stripped = s.replace('-', "");
    stripped.len() >= 16 && stripped.chars().all(|c| c.is_ascii_hexdigit())
}

/// Tente de lire un titre lisible depuis le dossier d'une story.
/// Cherche (dans l'ordre) : sidecar storyId, story.json/title.json, titre.txt.
/// Filtre les UUID boîte à histoires officiels qui ne sont pas des titres humains.
fn read_story_title(story_dir: &Path, sidecar: &Option<SidecarData>) -> Option<String> {
    // 1. Depuis le sidecar Synchro Boîte à histoires (priorité absolue)
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
pub fn read_inventory(mount: &Path) -> Option<StoryBoxInventory> {
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
        .filter(|e| !is_hidden_entry(e) && e.path().is_dir())
        .collect();
    dir_entries.sort_by(|a, b| {
        let a_short = a.file_name().to_string_lossy().to_uppercase().to_string();
        let b_short = b.file_name().to_string_lossy().to_uppercase().to_string();
        let a_idx = order_map.get(&a_short).copied().unwrap_or(usize::MAX);
        let b_idx = order_map.get(&b_short).copied().unwrap_or(usize::MAX);
        a_idx.cmp(&b_idx).then_with(|| a.path().cmp(&b.path()))
    });

    let stories: Vec<StoryBoxStoryEntry> = dir_entries
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
            Some(StoryBoxStoryEntry { short_uuid, sidecar, title, cover_path, size_bytes })
        })
        .collect();

    let managed_stories = stories.iter().filter(|s| s.sidecar.is_some()).count();
    let total_stories = stories.len();

    Some(StoryBoxInventory {
        mount: mount.to_string_lossy().into_owned(),
        stories,
        total_stories,
        managed_stories,
    })
}

/// Inventaire discriminé de la boîte validée (`mount`), sans reprober `/Volumes`.
pub fn get_storybox_inventory(mount: Option<&str>) -> StoryBoxInventoryResult {
    let mount_str = match mount {
        Some(m) => m.to_string(),
        None => {
            return StoryBoxInventoryResult {
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
        return StoryBoxInventoryResult {
            status: InventoryStatus::NoContentDir,
            mount: Some(mount_str),
            stories: vec![],
            total_stories: 0,
            managed_stories: 0,
            error: None,
        };
    }

    match read_inventory(Path::new(&mount_str)) {
        Some(inv) => StoryBoxInventoryResult {
            status: InventoryStatus::Ok,
            mount: Some(mount_str),
            stories: inv.stories,
            total_stories: inv.total_stories,
            managed_stories: inv.managed_stories,
            error: None,
        },
        None => StoryBoxInventoryResult {
            status: InventoryStatus::ReadError,
            mount: Some(mount_str),
            stories: vec![],
            total_stories: 0,
            managed_stories: 0,
            error: Some(
                fs::read_dir(&content_dir)
                    .err()
                    .map(|e| format!("Lecture de .content/ échouée : {e}"))
                    .unwrap_or_else(|| "Lecture de .content/ échouée — permissions ou erreur I/O".to_string()),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compare_story, get_storybox_inventory, move_story_in_pack_index, probe_mount, probe_root,
        read_inventory, read_sidecar, repair_pack_index_native, reorder_story_in_pack_index,
        write_pack_index_entries,
        DeviceState, InventoryStatus, StoryBoxDeviceProbe, StoryBoxStoryEntry, SidecarData,
        StoryDeviceStatus,
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

    fn connected_probe(root: &Path) -> StoryBoxDeviceProbe {
        probe_root(root, 0).expect("expected a connected boîte à histoires probe")
    }

    #[test]
    fn probe_root_detects_marker_and_counts_story_dirs() {
        let root = TempDir::new("storybox-probe-marker");
        let mount = root.path().join("STORYBOX");
        fs::create_dir_all(mount.join(".content").join("A1B2C3D4")).unwrap();
        fs::create_dir_all(mount.join(".content").join("E5F6G7H8")).unwrap();
        fs::write(mount.join(".md"), b"marker").unwrap();

        let probe = connected_probe(root.path());
        assert!(probe.connected);
        assert_eq!(probe.state, DeviceState::Connected);
        assert_eq!(probe.mount, Some(mount.to_string_lossy().into_owned()));
        assert!(probe.marker_found);
        assert!(probe.content_dir_present);
        assert_eq!(probe.story_dir_count, 2);
        assert_eq!(probe.detection_method.as_deref(), Some("marker"));
    }

    #[test]
    fn probe_root_falls_back_to_candidate_volume_name() {
        let root = TempDir::new("storybox-probe-name");
        let mount = root.path().join("Ma STORYBOX");
        fs::create_dir_all(&mount).unwrap();

        let probe = connected_probe(root.path());
        assert!(probe.connected);
        assert_eq!(probe.mount, Some(mount.to_string_lossy().into_owned()));
        assert!(!probe.marker_found);
        assert_eq!(probe.detection_method.as_deref(), Some("volume-name"));
    }

    /// Retire les droits d'un chemin et les remet au drop, même si le test panique.
    #[cfg(unix)]
    struct ChmodGuard {
        path: PathBuf,
        restore_mode: u32,
    }

    #[cfg(unix)]
    impl ChmodGuard {
        fn lock(path: &Path, restore_mode: u32) -> Self {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
            Self { path: path.to_path_buf(), restore_mode }
        }
    }

    #[cfg(unix)]
    impl Drop for ChmodGuard {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.restore_mode));
        }
    }

    /// Simule le piège sandbox : `.md` existe (`stat` OK) mais sa lecture est refusée.
    #[cfg(unix)]
    #[test]
    fn probe_reports_access_required_when_marker_unreadable() {
        let root = TempDir::new("storybox-probe-unreadable");
        let mount = root.path().join("STORYBOX");
        fs::create_dir_all(mount.join(".content").join("A1B2C3D4")).unwrap();
        let md = mount.join(".md");
        fs::write(&md, b"marker").unwrap();

        {
            let _locked = ChmodGuard::lock(&md, 0o644);
            if fs::read(&md).is_ok() {
                eprintln!("exécuté en root : chmod 000 n'empêche pas la lecture, test non significatif");
                return;
            }
            assert!(md.exists(), "le stat doit rester possible");

            let probe = connected_probe(root.path());
            assert_eq!(probe.state, DeviceState::AccessRequired);
            assert!(!probe.connected);
            assert_eq!(probe.mount, Some(mount.to_string_lossy().into_owned()));
            assert!(probe.marker_found);
            assert_eq!(probe.device_id, None);
            assert_eq!(probe.story_dir_count, 0);

            // Même verdict par la sonde directe d'un montage connu.
            assert_eq!(probe_mount(&mount).state, DeviceState::AccessRequired);
        }

        // Droits remis : la même boîte redevient « connectée ».
        assert!(fs::read(&md).is_ok(), "les droits de .md doivent être restaurés");
        let probe = connected_probe(root.path());
        assert_eq!(probe.state, DeviceState::Connected);
        assert!(probe.connected);
    }

    #[cfg(unix)]
    #[test]
    fn probe_reports_access_required_when_named_volume_unreadable() {
        let root = TempDir::new("storybox-probe-name-unreadable");
        let mount = root.path().join("MA STORYBOX");
        fs::create_dir_all(&mount).unwrap();
        {
            let _locked = ChmodGuard::lock(&mount, 0o755);
            if fs::read_dir(&mount).is_ok() {
                return; // root
            }
            let probe = connected_probe(root.path());
            assert_eq!(probe.state, DeviceState::AccessRequired);
            assert_eq!(probe.detection_method.as_deref(), Some("volume-name"));
        }
        assert!(fs::read_dir(&mount).is_ok(), "les droits du volume doivent être restaurés");
        assert_eq!(connected_probe(root.path()).state, DeviceState::Connected);
    }

    #[test]
    fn probe_mount_reports_disconnected_for_plain_folder() {
        let root = TempDir::new("storybox-probe-mount-plain");
        let folder = root.path().join("Musique");
        fs::create_dir_all(&folder).unwrap();
        assert_eq!(probe_mount(&folder).state, DeviceState::NotConnected);
    }

    #[test]
    fn inventory_uses_given_mount_and_reports_not_connected_without_it() {
        let none = get_storybox_inventory(None);
        assert_eq!(none.status, InventoryStatus::NotConnected);

        let root = TempDir::new("storybox-inv-mount");
        let mount = root.path().join("STORYBOX");
        fs::create_dir_all(mount.join(".content").join("AABBCCDD")).unwrap();
        let inv = get_storybox_inventory(Some(&mount.to_string_lossy()));
        assert_eq!(inv.status, InventoryStatus::Ok);
        assert_eq!(inv.total_stories, 1);
    }

    #[test]
    fn probe_root_ignores_unrelated_volumes() {
        let root = TempDir::new("storybox-probe-ignore");
        fs::create_dir_all(root.path().join("Macintosh HD")).unwrap();
        assert_eq!(probe_root(root.path(), 0), None);
    }

    #[test]
    fn inventory_none_when_no_content_dir() {
        let root = TempDir::new("storybox-inv-no-content");
        let mount = root.path().join("STORYBOX");
        fs::create_dir_all(&mount).unwrap();
        assert!(read_inventory(&mount).is_none());
    }

    #[test]
    fn inventory_stories_without_sidecar() {
        let root = TempDir::new("storybox-inv-no-sidecar");
        let mount = root.path().join("STORYBOX");
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
        let root = TempDir::new("storybox-inv-sidecar");
        let mount = root.path().join("STORYBOX");
        let story_dir = mount.join(".content").join("DEADBEEF");
        fs::create_dir_all(&story_dir).unwrap();
        fs::write(
            story_dir.join(".la-forge-a-histoires.json"),
            r#"{"story_id":"abc-123","hash":"sha256:deadbeef","pushed_at":"2024-01-01T00:00:00Z","source":"la-forge-a-histoires"}"#,
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
        let root = TempDir::new("storybox-inv-order");
        let mount = root.path().join("STORYBOX");
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
        let root = TempDir::new("storybox-move-order");
        let mount = root.path().join("STORYBOX");
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
        let root = TempDir::new("storybox-reorder-order");
        let mount = root.path().join("STORYBOX");
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

    /// Dossier d'histoire complet (`ni`, `li`, `ri`, `si`, `bt`).
    fn make_complete_story(story_dir: &Path) {
        fs::create_dir_all(story_dir).unwrap();
        for f in ["ni", "li", "ri", "si", "bt"] {
            fs::write(story_dir.join(f), b"x").unwrap();
        }
    }

    #[test]
    fn repair_pack_index_rebuilds_visible_entries_from_content_and_preserves_hidden() {
        let root = TempDir::new("storybox-repair-index");
        let mount = root.path().join("STORYBOX");
        make_complete_story(&mount.join(".content").join("11223344"));
        make_complete_story(&mount.join(".content").join("AABBCCDD"));
        make_complete_story(&mount.join(".content").join("55667788"));

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
        let root = TempDir::new("storybox-repair-index-invalid");
        let mount = root.path().join("STORYBOX");
        make_complete_story(&mount.join(".content").join("CAFEBABE"));
        make_complete_story(&mount.join(".content").join("DEADBEEF"));
        make_complete_story(&mount.join(".content").join("bonjour"));

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
    fn hidden_import_dirs_are_ignored_by_inventory_count_and_index() {
        let root = TempDir::new("storybox-hidden-dirs");
        let mount = root.path().join("STORYBOX");
        for dir in ["AABBCCDD", ".11223344.tmp", ".AABBCCDD.old"] {
            make_complete_story(&mount.join(".content").join(dir));
        }

        let inv = read_inventory(&mount).unwrap();
        let listed: Vec<_> = inv.stories.iter().map(|s| s.short_uuid.as_str()).collect();
        assert_eq!(listed, vec!["AABBCCDD"]);
        assert_eq!(super::count_story_dirs(&mount.join(".content")), 1);

        repair_pack_index_native(&mount.to_string_lossy()).unwrap();
        let pi = super::read_pack_index_entries(&mount.join(".pi")).unwrap();
        let order: Vec<_> = pi.iter().map(super::short_uuid_from_uuid_bytes).collect();
        assert_eq!(order, vec!["AABBCCDD"]);
    }

    fn index_order(path: &Path) -> Vec<String> {
        super::read_pack_index_entries(path)
            .unwrap()
            .iter()
            .map(super::short_uuid_from_uuid_bytes)
            .collect()
    }

    fn short_entry(short_uuid: &str) -> [u8; 16] {
        super::short_uuid_to_uuid_bytes(short_uuid).unwrap()
    }

    #[test]
    fn repair_pack_index_never_adds_incomplete_story_dirs() {
        let root = TempDir::new("storybox-repair-incomplete");
        let mount = root.path().join("STORYBOX");
        let content = mount.join(".content");
        make_complete_story(&content.join("AABBCCDD"));
        // Écrit par l'app (sidecar) sans `bt` : import interrompu, non ajouté.
        make_complete_story(&content.join("11223344"));
        fs::write(content.join("11223344").join(super::SIDECAR_FILE), b"{}").unwrap();
        fs::remove_file(content.join("11223344").join("bt")).unwrap();
        // Vide ; sans sidecar mais sans `si` : non ajoutés.
        fs::create_dir_all(content.join("55667788")).unwrap();
        make_complete_story(&content.join("99AABBCC"));
        fs::remove_file(content.join("99AABBCC").join("si")).unwrap();
        // Sans sidecar ni `bt` : complet pour la référence (qui régénère `bt`), ajouté.
        make_complete_story(&content.join("CAFE0001"));
        fs::remove_file(content.join("CAFE0001").join("bt")).unwrap();

        let report = repair_pack_index_native(&mount.to_string_lossy()).unwrap();
        assert_eq!(report.indexed, 2);
        assert_eq!(report.incomplete, vec!["11223344", "55667788", "99AABBCC"]);
        assert_eq!(index_order(&mount.join(".pi")), vec!["AABBCCDD", "CAFE0001"]);
        // Jamais supprimés automatiquement
        for dir in ["11223344", "55667788", "99AABBCC"] {
            assert!(content.join(dir).is_dir(), "{dir} doit rester sur la boîte");
        }
        assert!(content.join("11223344").join("ni").is_file());
    }

    /// R003 R3-2 : une entrée n'est retirée que si son dossier n'existe plus ; incomplète, elle
    /// est gardée à sa place et seulement signalée (référence : « Already in list but invalid »).
    #[test]
    fn repair_pack_index_keeps_indexed_story_dirs_even_incomplete() {
        let root = TempDir::new("storybox-repair-keep");
        let mount = root.path().join("STORYBOX");
        let content = mount.join(".content");
        make_complete_story(&content.join("AABBCCDD"));
        make_complete_story(&content.join("11223344"));
        fs::remove_file(content.join("11223344").join("ni")).unwrap();
        make_complete_story(&content.join("55667788"));
        fs::write(content.join("55667788").join(super::SIDECAR_FILE), b"{}").unwrap();
        fs::remove_file(content.join("55667788").join("bt")).unwrap();
        fs::create_dir_all(content.join("99AABBCC")).unwrap();
        let entries = ["11223344", "00000001", "55667788", "AABBCCDD", "99AABBCC"].map(short_entry);
        write_pack_index_entries(&mount.join(".pi"), &entries).unwrap();

        let report = repair_pack_index_native(&mount.to_string_lossy()).unwrap();
        assert_eq!(
            index_order(&mount.join(".pi")),
            vec!["11223344", "55667788", "AABBCCDD", "99AABBCC"],
            "seule l'entrée sans dossier (00000001) sort, l'ordre est gardé"
        );
        assert_eq!(report.incomplete, vec!["11223344", "55667788", "99AABBCC"]);
        assert_eq!(report.indexed, 4);
    }

    /// Relevé R003 : la référence range les histoires cachées dans `.content.hidden/`
    /// (`HIDDEN_STORIES_BASEDIR`). Leur entrée de `.pi.hidden` n'en sort jamais.
    #[test]
    fn repair_pack_index_keeps_hidden_stories_of_content_hidden() {
        let root = TempDir::new("storybox-repair-content-hidden");
        let mount = root.path().join("STORYBOX");
        make_complete_story(&mount.join(".content").join("AABBCCDD"));
        let hidden_content = mount.join(".content.hidden");
        make_complete_story(&hidden_content.join("11223344"));
        make_complete_story(&hidden_content.join("55667788"));
        fs::remove_file(hidden_content.join("55667788").join("li")).unwrap();
        // Caché par la référence mais absent de `.pi.hidden` : rattaché s'il est complet.
        make_complete_story(&hidden_content.join("99AABBCC"));
        write_pack_index_entries(&mount.join(".pi"), &[short_entry("AABBCCDD")]).unwrap();
        write_pack_index_entries(&mount.join(".pi.hidden"), &["55667788", "11223344"].map(short_entry)).unwrap();

        let report = repair_pack_index_native(&mount.to_string_lossy()).unwrap();
        assert_eq!(index_order(&mount.join(".pi")), vec!["AABBCCDD"]);
        assert_eq!(index_order(&mount.join(".pi.hidden")), vec!["55667788", "11223344", "99AABBCC"]);
        assert_eq!(report.incomplete, vec!["55667788"]);
    }

    /// Index illisible (ici un dossier à la place de `.pi` : échec de lecture sur tout volume,
    /// FAT compris, qui ignore `chmod`) : la réparation interne échoue fermé, sans rien écrire,
    /// et le message nomme la sortie.
    #[test]
    fn repair_pack_index_fails_closed_on_unreadable_index() {
        let root = TempDir::new("storybox-repair-unreadable");
        let mount = root.path().join("STORYBOX");
        make_complete_story(&mount.join(".content").join("AABBCCDD"));
        fs::create_dir_all(mount.join(".content").join("11223344")).unwrap();
        fs::create_dir_all(mount.join(".pi")).unwrap();
        let hidden = mount.join(".pi.hidden");
        write_pack_index_entries(&hidden, &["11223344"].map(short_entry)).unwrap();

        let err = repair_pack_index_native(&mount.to_string_lossy()).unwrap_err();
        assert!(err.starts_with("L'index de la boîte est illisible (.pi :"), "{err}");
        assert!(err.contains("Utilisez « Réparer l'index » pour le reconstruire"), "{err}");
        assert!(err.ends_with("Index non modifié."), "{err}");
        assert!(mount.join(".pi").is_dir());
        assert_eq!(index_order(&hidden), vec!["11223344"], "entrée incomplète non retirée");
    }

    /// R004 P5b : `.pi` tronqué (écriture coupée) → les entrées entières gardent leur ordre, même
    /// incomplètes ; seuls les octets en trop sont perdus, et c'est signalé.
    #[test]
    fn repair_pack_index_keeps_whole_entries_of_truncated_index() {
        let root = TempDir::new("storybox-repair-truncated");
        let mount = root.path().join("STORYBOX");
        let content = mount.join(".content");
        for short in ["55667788", "11223344", "99AABBCC"] {
            make_complete_story(&content.join(short));
        }
        fs::create_dir_all(content.join("AABBCCDD")).unwrap();
        let pi = mount.join(".pi");
        write_pack_index_entries(&pi, &["55667788", "AABBCCDD", "11223344"].map(short_entry)).unwrap();
        let mut data = fs::read(&pi).unwrap();
        data.extend_from_slice(&[1, 2, 3]);
        fs::write(&pi, &data).unwrap();

        let report = repair_pack_index_native(&mount.to_string_lossy()).unwrap();
        assert_eq!(index_order(&pi), vec!["55667788", "AABBCCDD", "11223344", "99AABBCC"]);
        assert_eq!(report.incomplete, vec!["AABBCCDD"]);
        assert_eq!(report.notices.len(), 1, "{:?}", report.notices);
        assert!(report.notices[0].contains(".pi était tronqué : 3 entrée(s)"), "{:?}", report.notices);
    }

    /// R003 : sur FAT, macOS pose `._<nom>` à côté de chaque fichier et dossier écrit.
    #[test]
    fn inventory_ignores_macos_metadata_files() {
        let root = TempDir::new("storybox-appledouble");
        let mount = root.path().join("STORYBOX");
        let story = mount.join(".content").join("AABBCCDD");
        make_complete_story(&story);
        fs::write(story.join("zz.png"), b"png").unwrap();
        fs::write(story.join("._zz.png"), vec![0u8; 4096]).unwrap();
        fs::write(story.join(".DS_Store"), vec![0u8; 100]).unwrap();
        fs::write(mount.join(".content").join("._AABBCCDD"), vec![0u8; 4096]).unwrap();

        let inv = read_inventory(&mount).unwrap();
        let listed: Vec<_> = inv.stories.iter().map(|s| s.short_uuid.as_str()).collect();
        assert_eq!(listed, vec!["AABBCCDD"]);
        assert_eq!(inv.stories[0].size_bytes, 5 + 3, "5 fichiers d'1 octet + zz.png");
        let cover = inv.stories[0].cover_path.clone().unwrap();
        assert!(cover.ends_with("/zz.png"), "{cover}");
        let report = repair_pack_index_native(&mount.to_string_lossy()).unwrap();
        assert_eq!(report.indexed, 1);
        assert!(report.incomplete.is_empty());
    }

    #[test]
    fn inventory_skips_sidecar_with_invalid_json() {
        let root = TempDir::new("storybox-inv-bad-json");
        let mount = root.path().join("STORYBOX");
        let story_dir = mount.join(".content").join("BADBADBAD");
        fs::create_dir_all(&story_dir).unwrap();
        fs::write(story_dir.join(".la-forge-a-histoires.json"), b"not json {{{").unwrap();

        let inv = read_inventory(&mount).expect("inventory");
        assert_eq!(inv.managed_stories, 0);
        assert!(inv.stories[0].sidecar.is_none());
    }

    #[test]
    fn inventory_skips_sidecar_with_wrong_source() {
        let root = TempDir::new("storybox-inv-wrong-source");
        let mount = root.path().join("STORYBOX");
        let story_dir = mount.join(".content").join("CAFECAFE");
        fs::create_dir_all(&story_dir).unwrap();
        fs::write(
            story_dir.join(".la-forge-a-histoires.json"),
            r#"{"story_id":"x","hash":"sha256:x","pushed_at":"2024-01-01T00:00:00Z","source":"other-tool"}"#,
        ).unwrap();

        let inv = read_inventory(&mount).expect("inventory");
        assert_eq!(inv.managed_stories, 0);
        assert!(inv.stories[0].sidecar.is_none());
    }

    #[test]
    fn read_sidecar_returns_none_when_file_absent() {
        let root = TempDir::new("storybox-sidecar-absent");
        let story_dir = root.path().join("AABB1122");
        fs::create_dir_all(&story_dir).unwrap();
        assert!(read_sidecar(&story_dir).is_none());
    }

    fn make_entry(short_uuid: &str, story_id: &str, hash: &str) -> StoryBoxStoryEntry {
        StoryBoxStoryEntry {
            short_uuid: short_uuid.to_string(),
            sidecar: Some(SidecarData {
                story_id: story_id.to_string(),
                hash: hash.to_string(),
                pushed_at: "2024-01-01T00:00:00Z".to_string(),
                source: "la-forge-a-histoires".to_string(),
            }),
            title: Some(story_id.replace('_', " ")),
            cover_path: None,
            size_bytes: 0,
        }
    }

    fn make_unmanaged(short_uuid: &str) -> StoryBoxStoryEntry {
        StoryBoxStoryEntry { short_uuid: short_uuid.to_string(), sidecar: None, title: None, cover_path: None, size_bytes: 0 }
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
