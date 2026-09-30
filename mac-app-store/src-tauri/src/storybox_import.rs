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
        // V3 : .md v6/v7 → AES-128-CBC ; v8+ → erreur explicite, rien n'est écrit.
        return crate::storybox_v3::import_story_v3(
            mount, &md_data, zip_path, story_id, hash, on_progress,
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

    // ── Bout en bout contre StoryBox.QT `import_studio_zip` ──────────────────
    // Même .md, même story.json, mêmes octets audio/image : les fichiers écrits
    // par la référence Python sont figés ci-dessous (taille, SHA-256).

    use crate::storybox_v3::test_fixtures::{md_v5, md_v6, md_v7, md_v8, sha256_hex};

    const STORY_JSON: &str = r#"{"format": "v1", "version": 1, "title": "Histoire V3", "description": "", "nightModeAvailable": true, "factoryPack": false, "stageNodes": [{"uuid": "12345678-9abc-4def-8123-456789abcdef", "squareOne": true, "audio": "histoire.mp3", "image": "image001.bmp", "controlSettings": {"wheel": false, "ok": false, "home": true, "pause": true, "autoplay": true}, "okTransition": null, "homeTransition": null}], "actionNodes": [], "listNodes": []}"#;

    fn pattern(len: usize, a: usize, b: usize) -> Vec<u8> {
        (0..len).map(|i| ((i * a + b) & 0xFF) as u8).collect()
    }

    /// Boîte simulée : `.md`, `.content/` vide, et un pack ZIP STUdio hors montage.
    fn mount_with_pack(md: &[u8]) -> (tempfile::TempDir, tempfile::TempDir, PathBuf) {
        use std::io::Write;
        let mount = tempfile::tempdir().unwrap();
        fs::write(mount.path().join(".md"), md).unwrap();
        fs::create_dir_all(mount.path().join(".content")).unwrap();

        let packs = tempfile::tempdir().unwrap();
        let zip_path = packs.path().join("story.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        writer.start_file("story.json", opts).unwrap();
        writer.write_all(STORY_JSON.as_bytes()).unwrap();
        writer.start_file("assets/histoire.mp3", opts).unwrap();
        writer.write_all(&pattern(1000, 31, 17)).unwrap();
        writer.start_file("assets/image001.bmp", opts).unwrap();
        writer.write_all(&pattern(700, 19, 29)).unwrap();
        writer.finish().unwrap();
        (mount, packs, zip_path)
    }

    /// Tous les fichiers sous `root` : chemin relatif → (taille, SHA-256).
    fn tree(root: &Path) -> BTreeMap<String, (usize, String)> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, (usize, String)>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let data = fs::read(&path).unwrap();
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    out.insert(rel, (data.len(), sha256_hex(&data)));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    fn import(mount: &Path, zip_path: &Path) -> Result<ImportResult, String> {
        import_story(mount.to_str().unwrap(), zip_path, "histoire-v3", "sha256:abc", &|_| {})
    }

    const EMPTY_SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const NI_SHA: &str = "d1087949521203747b57994a023bca1eb1bf0d69cae5b7ee9b07f2d366c983e4";
    const SIDECAR: &str = "89ABCDEF/.la-forge-a-histoires.json";

    /// Compare l'arborescence écrite aux fichiers produits par la référence.
    fn assert_matches_reference(mount: &Path, expected: &[(&str, usize, &str)]) {
        let mut actual = tree(&mount.join(".content"));
        assert!(actual.remove(SIDECAR).is_some(), "sidecar Synchro absent");
        let expected: BTreeMap<String, (usize, String)> = expected
            .iter()
            .map(|(p, l, h)| (p.to_string(), (*l, h.to_string())))
            .collect();
        assert_eq!(actual, expected);
    }

    /// `.pi` : une entrée de 16 octets pour l'histoire importée.
    fn assert_pack_index(mount: &Path) {
        let pi = fs::read(mount.join(".pi")).unwrap();
        assert_eq!(pi.len(), 16);
        // Écart connu et antérieur à M0005 (chemin commun V2/V3, storybox_device.rs) :
        // la référence écrit l'UUID complet 123456789abc4def8123456789abcdef ;
        // repair_pack_index_native ne garde que les 4 derniers octets (short UUID).
        assert_eq!(hex::encode(&pi[12..]), "89abcdef");
    }

    #[test]
    fn import_v3_md6_matches_reference_end_to_end() {
        let (mount, _packs, zip_path) = mount_with_pack(&md_v6());
        let result = import(mount.path(), &zip_path).unwrap();
        assert_eq!(result.short_uuid, "89ABCDEF");
        assert_matches_reference(mount.path(), &[
            ("89ABCDEF/bt", 32, "00e988677eecf94c0bb9233371c7c0d6f4db8ebdcdecb7c5ebaa666f17249227"),
            ("89ABCDEF/li", 16, "0bfd31847310a1459b78a95227eb16145270de071ba414afd89b73b4d1cfed55"),
            ("89ABCDEF/ni", 556, NI_SHA),
            ("89ABCDEF/nm", 0, EMPTY_SHA),
            ("89ABCDEF/rf/000/IMAGE001", 700, "43a952c90c94606e3a942c209ed3c70fe0a9f2c78fe65740d37bff7e097df39d"),
            ("89ABCDEF/ri", 16, "ffa810ecaea86365b8002090244dab0d7d83e4dd7c4a2ce6918b80f7d4390f9e"),
            ("89ABCDEF/sf/000/HISTOIRE", 1000, "25f0ee19873244be4179b268563d5309474a0242b55810e23a65ac99845bce8d"),
            ("89ABCDEF/si", 16, "325792ed2d6b83a74d07cb6206de9bcf9e01fdcb3332e861e8d5aae8a0901b7e"),
        ]);
        assert_pack_index(mount.path());
        // bt v6 = md[0x40..0x60], en clair
        let bt = fs::read(mount.path().join(".content/89ABCDEF/bt")).unwrap();
        assert_eq!(bt, md_v6()[0x40..0x60]);
        // Au-delà de 512 octets, l'audio reste en clair
        let audio = fs::read(mount.path().join(".content/89ABCDEF/sf/000/HISTOIRE")).unwrap();
        assert_eq!(&audio[512..], &pattern(1000, 31, 17)[512..]);
    }

    #[test]
    fn import_v3_md7_matches_reference_end_to_end() {
        let (mount, _packs, zip_path) = mount_with_pack(&md_v7());
        import(mount.path(), &zip_path).unwrap();
        assert_matches_reference(mount.path(), &[
            ("89ABCDEF/bt", 32, "b352a9a2f56327cc35e0f43a5efa6dfb98023d135609f7f7724c3d25a0221e7a"),
            ("89ABCDEF/li", 16, "6df25c8af0e5f8c42776b1a61a0b31338640b5f6336934e0f983dc0ca1ed3959"),
            ("89ABCDEF/ni", 556, NI_SHA),
            ("89ABCDEF/nm", 0, EMPTY_SHA),
            ("89ABCDEF/rf/000/IMAGE001", 700, "371f8b8e9294bdfbb2af4f38d7b6d2465b319761f6b755c25a2f6ece2403cd33"),
            ("89ABCDEF/ri", 16, "a3a5c3f618a599131c595e398120051a2ffe0d2698843e5f1ced865a4b941d58"),
            ("89ABCDEF/sf/000/HISTOIRE", 1000, "90ca86a595df57251c0d43770bf354e2cae425f8e9d8861fe820dcb96cb0c045"),
            ("89ABCDEF/si", 16, "f5fdb9880f4f4fe2519972444da5448df59d6a3196b7c29c18b7392e5af7b9c0"),
        ]);
        assert_pack_index(mount.path());
    }

    #[test]
    fn import_v3_md8_is_refused_without_any_write() {
        let (mount, _packs, zip_path) = mount_with_pack(&md_v8());
        let before = tree(mount.path());
        let err = import(mount.path(), &zip_path).unwrap_err();
        assert!(err.starts_with(crate::storybox_v3::ERR_V3_RECENT), "{err}");
        assert_eq!(tree(mount.path()), before, "la boîte ne doit pas être modifiée");
        assert!(!mount.path().join(".pi").exists());
    }

    #[test]
    fn import_v3_reimport_replaces_story_dir() {
        let (mount, _packs, zip_path) = mount_with_pack(&md_v7());
        import(mount.path(), &zip_path).unwrap();
        fs::write(mount.path().join(".content/89ABCDEF/sf/000/RESIDU"), b"x").unwrap();
        import(mount.path(), &zip_path).unwrap();
        assert!(!mount.path().join(".content/89ABCDEF/sf/000/RESIDU").exists());
        assert_pack_index(mount.path());
    }

    #[test]
    fn import_v2_unchanged_and_matches_reference() {
        let (mount, _packs, zip_path) = mount_with_pack(&md_v5());
        import(mount.path(), &zip_path).unwrap();
        let mut actual = tree(&mount.path().join(".content"));
        actual.remove(SIDECAR).unwrap();
        // bt V2 : écart connu et antérieur à M0005 (hors périmètre, non modifié ici).
        // Référence : 12 octets (252bbd4d…), cipher(ri_chiffré[:64]) sans complément.
        // Rust : make_bt_v2 complète l'entrée à 64 octets. Valeur actuelle figée.
        let (bt_len, bt_sha) = actual.remove("89ABCDEF/bt").unwrap();
        assert_eq!(bt_len, 64);
        let ri_enc = fs::read(mount.path().join(".content/89ABCDEF/ri")).unwrap();
        let device_key = storybox_crypto::derive_v2_device_key(&md_v5()).unwrap();
        assert_eq!(bt_sha, sha256_hex(&storybox_crypto::make_bt_v2(&ri_enc, &device_key)));

        let expected: BTreeMap<String, (usize, String)> = [
            ("89ABCDEF/li", 8, "6acc15493144806f95815bd6c602d235c8118a756d45dce1e65bbf7e49edb510"),
            ("89ABCDEF/ni", 556, NI_SHA),
            ("89ABCDEF/nm", 0, EMPTY_SHA),
            ("89ABCDEF/rf/000/IMAGE001", 700, "8a092201142e1f18755b6dcc5ea3adf20f4b7c32fc4e954a34b69186a33d89d1"),
            ("89ABCDEF/ri", 12, "acaec261e616fa2c26284b3b79ffb839eb05741cb64edf54314d2720d63eb4da"),
            ("89ABCDEF/sf/000/HISTOIRE", 1000, "e74a7959a1aa2257363cb9574b702497f86cd283f0e471f4ec643433fd805cb5"),
            ("89ABCDEF/si", 12, "51e153185498ea3d6085314ef76661d1c489f132706a061f820d4db9574f66ad"),
        ]
        .iter()
        .map(|(p, l, h)| (p.to_string(), (*l, h.to_string())))
        .collect();
        assert_eq!(actual, expected);
        assert_pack_index(mount.path());
    }
}
