//! Pipeline d'import natif Rust pour variante Mac App Store.
//!
//! Remplace boite-bridge.py pour les opérations compatibles V2 boîte à histoires.
//! V3 (AES-128-CBC) retourne une erreur explicite : utiliser Synchro Boîte à histoires direct.

use crate::storybox_crypto;
use crate::storybox_device;
use crate::storybox_sync;
use crate::studio_story::StudioStory;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, ZipArchive};

// ── Génération pack ZIP (remplace studio-pack-generator) ──────────────────────

/// Génère un UUID reproductible depuis une graine (nom de fichier audio).
/// Ne requiert pas la feature `v4` du crate uuid.
fn story_uuid_from_seed(seed: &str) -> Uuid {
    let hash = Sha256::digest(seed.as_bytes());
    let mut b = [0u8; 16];
    b.copy_from_slice(&hash[..16]);
    // Forcer version 4 + variant RFC 4122
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    Uuid::from_bytes(b)
}

/// Génère un story pack ZIP simple depuis un fichier audio MP3.
///
/// Remplace `studio-pack-generator` pour les histoires linéaires sans interactivité.
/// Le ZIP produit est compatible avec notre `StudioStory::from_json` + `import_story`.
///
/// Retourne le chemin du ZIP dans un dossier temporaire. Le dossier parent est à
/// supprimer après usage.
pub fn generate_simple_pack(audio_path: &Path) -> Result<PathBuf, String> {
    if !audio_path.is_file() {
        return Err(format!("Fichier audio introuvable : {}", audio_path.display()));
    }

    let audio_name = audio_path
        .file_name()
        .ok_or("Nom de fichier audio invalide")?
        .to_string_lossy()
        .into_owned();
    let story_stem = audio_path
        .file_stem()
        .ok_or("Nom de fichier audio invalide")?
        .to_string_lossy()
        .into_owned();

    if !audio_name.to_lowercase().ends_with(".mp3") {
        return Err(format!(
            "Format audio non supporté : '{}'. Seul MP3 est accepté.",
            audio_name
        ));
    }

    let uuid = story_uuid_from_seed(&story_stem);
    let story_json = serde_json::json!({
        "format": "v1",
        "version": 1,
        "title": story_stem,
        "description": "",
        "nightModeAvailable": false,
        "factoryPack": false,
        "stageNodes": [{
            "uuid": uuid.to_string(),
            "squareOne": true,
            "audio": audio_name,
            "image": "",
            "controlSettings": {
                "wheel": false,
                "ok": false,
                "home": true,
                "pause": true,
                "autoplay": true
            },
            "okTransition": null,
            "homeTransition": null
        }],
        "actionNodes": [],
        "listNodes": []
    });

    let audio_data = fs::read(audio_path)
        .map_err(|e| format!("Lecture '{}' échouée : {e}", audio_name))?;

    let tmp_dir = std::env::temp_dir().join(format!("synchro_boite_a_histoires-pack-{}", uuid.simple()));
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Création dossier temporaire échouée : {e}"))?;

    let zip_path = tmp_dir.join(format!("{story_stem}.zip"));
    let zip_file = fs::File::create(&zip_path)
        .map_err(|e| format!("Création ZIP échouée : {e}"))?;
    let mut writer = ZipWriter::new(zip_file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    writer
        .start_file("story.json", opts)
        .map_err(|e| format!("ZIP story.json header échoué : {e}"))?;
    writer
        .write_all(serde_json::to_string(&story_json).unwrap().as_bytes())
        .map_err(|e| format!("ZIP story.json write échoué : {e}"))?;

    writer
        .start_file(&audio_name, opts)
        .map_err(|e| format!("ZIP audio header échoué : {e}"))?;
    writer
        .write_all(&audio_data)
        .map_err(|e| format!("ZIP audio write échoué : {e}"))?;

    writer
        .finish()
        .map_err(|e| format!("ZIP finalisation échouée : {e}"))?;

    Ok(zip_path)
}

// ── Lecture ZIP ───────────────────────────────────────────────────────────────

fn read_all_zip_entries(zip_path: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Ouverture ZIP échouée : {e}"))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Lecture ZIP échouée : {e}"))?;

    let mut entries = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Lecture entrée ZIP #{i} échouée : {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)
            .map_err(|e| format!("Lecture contenu ZIP '{name}' échouée : {e}"))?;
        entries.insert(name, data);
    }
    Ok(entries)
}

/// Cherche un fichier dans les entrées ZIP par son nom exact, puis par basename.
fn find_in_zip<'a>(
    entries: &'a BTreeMap<String, Vec<u8>>,
    target: &str,
) -> Result<&'a [u8], String> {
    // 1. Correspondance exacte
    if let Some(data) = entries.get(target) {
        return Ok(data);
    }

    // 2. Chercher par basename (insensible à la casse)
    let target_base = target
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(target)
        .to_lowercase();

    for (name, data) in entries {
        let entry_base = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(name)
            .to_lowercase();
        if entry_base == target_base {
            return Ok(data);
        }
    }

    Err(format!("Fichier '{target}' introuvable dans le ZIP"))
}

// ── Import principal ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ImportResult {
    pub short_uuid: String,
}

/// Importe un story pack ZIP vers une boîte boîte à histoires V2 montée.
///
/// Retourne le `short_uuid` créé sur la boîte.
/// Rollback automatique du dossier en cas d'échec.
pub fn import_story(
    mount: &str,
    zip_path: &Path,
    story_id: &str,
    hash: &str,
    on_progress: &dyn Fn(&str),
) -> Result<ImportResult, String> {
    // ── 1. Lire et vérifier le fichier .md ───────────────────────────────────
    let md_path = Path::new(mount).join(".md");
    let md_data = fs::read(&md_path)
        .map_err(|e| format!("Fichier .md boîte à histoires introuvable : {e}"))?;

    if storybox_crypto::md_hw_version(&md_data) >= 3 {
        return Err(
            "boîte à histoires V3 détectée (firmware récent). \
             Cet appareil utilise AES-128-CBC non encore supporté dans la variante App Store. \
             Utilisez Synchro Boîte à histoires (distribution directe) pour cet appareil."
                .to_string(),
        );
    }

    let device_key = storybox_crypto::derive_v2_device_key(&md_data)?;

    // ── 2. Extraire le ZIP ───────────────────────────────────────────────────
    on_progress("Lecture du pack…");
    let zip_entries = read_all_zip_entries(zip_path)?;

    // ── 3. Parser story.json ─────────────────────────────────────────────────
    let story_json_bytes = find_in_zip(&zip_entries, "story.json")?;
    let story_json: serde_json::Value = serde_json::from_slice(story_json_bytes)
        .map_err(|e| format!("story.json invalide : {e}"))?;
    let story = StudioStory::from_json(&story_json)?;

    if !story.compatible {
        return Err(
            "L'histoire contient des fichiers audio non-MP3. \
             Seul le format MP3 est supporté."
                .to_string(),
        );
    }

    // ── 4. Créer le dossier story ────────────────────────────────────────────
    let short_uuid = story.short_uuid();
    let content_dir = Path::new(mount).join(".content");
    if !content_dir.is_dir() {
        return Err("Dossier .content introuvable sur la boîte".to_string());
    }

    let story_dir = content_dir.join(&short_uuid);

    // Nettoyage si dossier existant (réimport)
    if story_dir.exists() {
        fs::remove_dir_all(&story_dir)
            .map_err(|e| format!("Suppression dossier existant échouée : {e}"))?;
    }

    fs::create_dir_all(story_dir.join("rf").join("000"))
        .map_err(|e| format!("Création rf/000/ échouée : {e}"))?;
    fs::create_dir_all(story_dir.join("sf").join("000"))
        .map_err(|e| format!("Création sf/000/ échouée : {e}"))?;

    // À partir d'ici : rollback en cas d'échec
    let result = write_story_files(
        &story,
        &story_dir,
        &zip_entries,
        &device_key,
        on_progress,
    );

    if let Err(ref e) = result {
        let _ = fs::remove_dir_all(&story_dir);
        return Err(e.clone());
    }

    // ── 5. Mise à jour de l'index de la boîte ───────────────────────────────
    on_progress("Mise à jour de l'index…");
    if let Err(e) = storybox_device::repair_pack_index_native(mount) {
        let _ = fs::remove_dir_all(&story_dir);
        return Err(format!("Mise à jour index échouée : {e}"));
    }

    // ── 6. Écriture du sidecar Synchro Boîte à histoires ─────────────────────────────────────
    storybox_sync::write_sidecar(mount, &short_uuid, story_id, hash)?;

    Ok(ImportResult { short_uuid })
}

/// Écrit tous les fichiers du story pack dans le dossier déjà créé.
fn write_story_files(
    story: &StudioStory,
    story_dir: &Path,
    zip_entries: &BTreeMap<String, Vec<u8>>,
    device_key: &[u32; 4],
    on_progress: &dyn Fn(&str),
) -> Result<(), String> {
    // ── Fichiers audio → sf/000/<NOM> ────────────────────────────────────────
    on_progress(&format!(
        "Transfert audio ({} fichier(s))…",
        story.si.len()
    ));
    for asset in &story.si {
        let data = find_in_zip(zip_entries, &asset.source_name)?;
        let ciphered = storybox_crypto::cipher_story_data(data);
        let dest = story_dir.join("sf").join("000").join(&asset.normalized_name);
        fs::write(&dest, &ciphered)
            .map_err(|e| format!("Écriture sf/000/{} échouée : {e}", asset.normalized_name))?;
    }

    // ── Fichiers image → rf/000/<NOM> ─────────────────────────────────────────
    if !story.ri.is_empty() {
        on_progress(&format!(
            "Transfert images ({} fichier(s))…",
            story.ri.len()
        ));
    }
    for asset in &story.ri {
        let data = find_in_zip(zip_entries, &asset.source_name)?;
        let ciphered = storybox_crypto::cipher_story_data(data);
        let dest = story_dir.join("rf").join("000").join(&asset.normalized_name);
        fs::write(&dest, &ciphered)
            .map_err(|e| format!("Écriture rf/000/{} échouée : {e}", asset.normalized_name))?;
    }

    // ── Index files ──────────────────────────────────────────────────────────
    on_progress("Écriture des index…");

    let ri_data_raw = story.ri_data();
    let si_data = story.si_data();
    let li_data = story.li_data();
    let ni_data = story.ni_data()?;

    // ri, si, li : chiffrés avec la clé générique (premiers 512 octets)
    let ri_data_enc = storybox_crypto::cipher_story_data(&ri_data_raw);
    fs::write(story_dir.join("ri"), &ri_data_enc)
        .map_err(|e| format!("Écriture ri échouée : {e}"))?;
    fs::write(story_dir.join("si"), storybox_crypto::cipher_story_data(&si_data))
        .map_err(|e| format!("Écriture si échouée : {e}"))?;
    fs::write(story_dir.join("li"), storybox_crypto::cipher_story_data(&li_data))
        .map_err(|e| format!("Écriture li échouée : {e}"))?;

    // ni : NON chiffré
    fs::write(story_dir.join("ni"), &ni_data)
        .map_err(|e| format!("Écriture ni échouée : {e}"))?;

    // nm : fichier vide si nightMode activé, absent sinon (référence StoryBox.QT)
    if story.night_mode_available {
        fs::write(story_dir.join("nm"), b"")
            .map_err(|e| format!("Écriture nm échouée : {e}"))?;
    }

    // bt : cipher(ri_chiffré[:64], device_key) — token d'autorisation firmware
    let bt = storybox_crypto::make_bt_v2(&ri_data_enc, device_key);
    fs::write(story_dir.join("bt"), &bt)
        .map_err(|e| format!("Écriture bt échouée : {e}"))?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_entries(pairs: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect()
    }

    #[test]
    fn find_in_zip_exact_match() {
        let entries = make_entries(&[("story.json", b"{}"), ("audio.mp3", b"fake")]);
        assert_eq!(find_in_zip(&entries, "story.json").unwrap(), b"{}");
        assert_eq!(find_in_zip(&entries, "audio.mp3").unwrap(), b"fake");
    }

    #[test]
    fn find_in_zip_basename_fallback() {
        let entries = make_entries(&[("assets/cover.png", b"png")]);
        // Exact miss → basename match
        assert_eq!(find_in_zip(&entries, "cover.png").unwrap(), b"png");
    }

    #[test]
    fn find_in_zip_case_insensitive_basename() {
        let entries = make_entries(&[("assets/AUDIO.MP3", b"mp3")]);
        assert_eq!(find_in_zip(&entries, "audio.mp3").unwrap(), b"mp3");
    }

    #[test]
    fn find_in_zip_missing_returns_error() {
        let entries = make_entries(&[("other.txt", b"x")]);
        assert!(find_in_zip(&entries, "missing.mp3").is_err());
    }

    #[test]
    fn import_rejects_v3_device() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let mount = tmp.path();

        // Créer un .md V3 (md_version = 6)
        let mut md = vec![0u8; 512];
        md[0] = 6; // md_version 6 → V3
        fs::write(mount.join(".md"), &md).unwrap();
        fs::create_dir_all(mount.join(".content")).unwrap();

        // ZIP minimal
        let zip_path = mount.join("test.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("story.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap();

        let result = import_story(
            mount.to_str().unwrap(),
            &zip_path,
            "test-story",
            "sha256:abc",
            &|_| {},
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("V3"));
    }
}
