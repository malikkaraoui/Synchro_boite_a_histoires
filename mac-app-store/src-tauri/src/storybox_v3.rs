//! Import natif pour boîtes à histoires V3 (`.md` v6 et v7) — AES-128-CBC.
//!
//! Aucune action du parent : les clés d'histoire et le fichier `bt` sont dérivés du seul
//! fichier `.md` de la boîte, exactement comme StoryBox.QT (`device_storybox.py`,
//! `__md6to7_parse`, `load_md_fakestory_keys`, `import_studio_zip`).
//!
//! - v6 : `bt` = `md[0x40..0x60]` ; `story_key`/`story_iv` forgés depuis le SNU.
//! - v7 : `story_key` = reverse(`md[0x40..0x50]`), `story_iv` = reverse(`md[0x50..0x60]`) ;
//!   `bt` forgé depuis le SNU.
//! - v8+ : refus explicite. La référence n'importe qu'avec une sauvegarde `.md` v6/v7
//!   antérieure ou un fichier de clés externe, que cette variante ne gère pas.
//!
//! La lecture du `.md` et le refus éventuel ont lieu AVANT toute écriture sur la boîte.

use crate::storybox_crypto;
use crate::storybox_import::{self, ImportResult};
use crate::studio_story::StudioStory;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Tailles de `.md` acceptées par la référence pour v6/v7 (`md_size in [112, 128]`).
const MD_V3_SIZES: [usize; 2] = [112, 128];
const MD_SNU_OFFSET: usize = 0x1A;
const MD_SNU_LEN: usize = 14;

/// Message renvoyé pour un `.md` v8 et plus (préfixe stable, testé).
pub const ERR_V3_RECENT: &str = "Boîte V3 récente : clés non disponibles";

/// Clés et jeton d'autorisation d'une boîte V3, dérivés du `.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V3Keys {
    pub md_version: u16,
    pub story_key: [u8; 16],
    pub story_iv: [u8; 16],
    /// Contenu du fichier `bt`, écrit tel quel (non chiffré) dans chaque histoire.
    pub bt: [u8; 32],
}

/// StoryBox.QT `reverse_bytes` : inverse l'ordre des octets dans chaque mot de 4 octets.
fn reverse_words(input: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (dst, src) in out.chunks_exact_mut(4).zip(input.chunks_exact(4)) {
        dst.copy_from_slice(src);
        dst.reverse();
    }
    out
}

/// `binascii.hexlify(binascii.unhexlify(md[0x1A:0x28]))` : SNU en hexadécimal ASCII minuscule.
fn snu_hex_lower(md: &[u8]) -> Result<[u8; MD_SNU_LEN], String> {
    let raw = &md[MD_SNU_OFFSET..MD_SNU_OFFSET + MD_SNU_LEN];
    if !raw.iter().all(u8::is_ascii_hexdigit) {
        return Err(format!(
            "Fichier .md V3 illisible : numéro de série invalide ({})",
            String::from_utf8_lossy(raw)
        ));
    }
    let mut out = [0u8; MD_SNU_LEN];
    for (dst, src) in out.iter_mut().zip(raw) {
        *dst = src.to_ascii_lowercase();
    }
    Ok(out)
}

/// Lit un `.md` V3 et dérive `story_key`, `story_iv` et `bt`.
///
/// Même aiguillage que StoryBox.QT `__feed_device` : version = 2 premiers octets
/// little-endian, v6/v7 seulement si la taille vaut 112 ou 128 octets.
pub fn parse_v3_md(md: &[u8]) -> Result<V3Keys, String> {
    if md.len() < 2 {
        return Err("Fichier .md V3 illisible : trop court".to_string());
    }
    let md_version = u16::from_le_bytes([md[0], md[1]]);

    if md_version >= 8 {
        return Err(format!(
            "{ERR_V3_RECENT} (fichier .md v{md_version}). \
             Cette boîte n'est pas encore prise en charge ; rien n'a été écrit sur la boîte."
        ));
    }
    if !(6..=7).contains(&md_version) || !MD_V3_SIZES.contains(&md.len()) {
        return Err(format!(
            "Fichier .md V3 non reconnu (version {md_version}, {} octets). \
             Rien n'a été écrit sur la boîte.",
            md.len()
        ));
    }

    let snu = snu_hex_lower(md)?;
    let block = |start: usize| -> [u8; 16] { md[start..start + 16].try_into().unwrap() };

    let (story_key, story_iv, bt) = if md_version == 6 {
        // load_md_fakestory_keys : clé = hex(SNU) + 00 00 ; IV = 8 × 00 + hex(SNU)[:8]
        let mut key = [0u8; 16];
        key[..MD_SNU_LEN].copy_from_slice(&snu);
        let mut iv = [0u8; 16];
        iv[8..].copy_from_slice(&snu[..8]);
        let bt: [u8; 32] = md[0x40..0x60].try_into().unwrap();
        (reverse_words(&key), reverse_words(&iv), bt)
    } else {
        // bt = hex(SNU) + 10 × 00 + hex(SNU)[:8]
        let mut bt = [0u8; 32];
        bt[..MD_SNU_LEN].copy_from_slice(&snu);
        bt[24..].copy_from_slice(&snu[..8]);
        (reverse_words(&block(0x40)), reverse_words(&block(0x50)), bt)
    };

    Ok(V3Keys { md_version, story_key, story_iv, bt })
}

// ── Lecture ZIP (mêmes règles que storybox_import) ────────────────────────────

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
    if let Some(data) = entries.get(target) {
        return Ok(data);
    }

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

/// Importe un story pack ZIP vers une boîte V3 (`.md` v6/v7) montée.
///
/// Même arborescence et mêmes noms que le chemin V2 (`.content/<SHORT_UUID>/`) ;
/// seuls changent le chiffrement (AES-128-CBC, clés du `.md`) et le fichier `bt`.
pub fn import_story_v3(
    mount: &str,
    md_data: &[u8],
    zip_path: &Path,
    story_id: &str,
    hash: &str,
    on_progress: &dyn Fn(&str),
) -> Result<ImportResult, String> {
    // ── 1. Clés : en cas de refus (v8+, .md inattendu), rien n'est écrit ────
    let keys = parse_v3_md(md_data)?;

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

    // ── 4. Écriture en transit, puis remplacement (storybox_import::install_story) ──
    storybox_import::install_story(mount, &story.short_uuid(), story_id, hash, on_progress, |dir| {
        write_story_files_v3(&story, dir, &zip_entries, &keys, on_progress)
    })
}

/// Écrit les fichiers d'une histoire V3 (référence : `import_studio_zip` en V3).
fn write_story_files_v3(
    story: &StudioStory,
    story_dir: &Path,
    zip_entries: &BTreeMap<String, Vec<u8>>,
    keys: &V3Keys,
    on_progress: &dyn Fn(&str),
) -> Result<(), String> {
    let cipher = |data: &[u8]| {
        storybox_crypto::cipher_story_data_v3(data, &keys.story_key, &keys.story_iv)
    };

    // ── Fichiers audio → sf/000/<NOM> ────────────────────────────────────────
    on_progress(&format!(
        "Transfert audio ({} fichier(s))…",
        story.si.len()
    ));
    for asset in &story.si {
        let data = find_in_zip(zip_entries, &asset.source_name)?;
        let dest = story_dir.join("sf").join("000").join(&asset.normalized_name);
        fs::write(&dest, cipher(data))
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
        let dest = story_dir.join("rf").join("000").join(&asset.normalized_name);
        fs::write(&dest, cipher(data))
            .map_err(|e| format!("Écriture rf/000/{} échouée : {e}", asset.normalized_name))?;
    }

    // ── Index files ──────────────────────────────────────────────────────────
    on_progress("Écriture des index…");

    // ri, si, li : AES avec la clé d'histoire (premiers 512 octets)
    fs::write(story_dir.join("ri"), cipher(&story.ri_data()))
        .map_err(|e| format!("Écriture ri échouée : {e}"))?;
    fs::write(story_dir.join("si"), cipher(&story.si_data()))
        .map_err(|e| format!("Écriture si échouée : {e}"))?;
    fs::write(story_dir.join("li"), cipher(&story.li_data()))
        .map_err(|e| format!("Écriture li échouée : {e}"))?;

    // ni : NON chiffré
    fs::write(story_dir.join("ni"), story.ni_data()?)
        .map_err(|e| format!("Écriture ni échouée : {e}"))?;

    // nm : fichier vide si nightMode activé, absent sinon (référence StoryBox.QT)
    if story.night_mode_available {
        fs::write(story_dir.join("nm"), b"")
            .map_err(|e| format!("Écriture nm échouée : {e}"))?;
    }

    // bt : dérivé du .md, écrit tel quel (la référence ne le chiffre pas en V3)
    fs::write(story_dir.join("bt"), keys.bt)
        .map_err(|e| format!("Écriture bt échouée : {e}"))?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────
// Vecteurs produits par StoryBox.QT (device_storybox.py) sur les mêmes `.md` synthétiques.

/// Fixtures partagées avec les tests de bout en bout de `storybox_import`.
#[cfg(test)]
pub(crate) mod test_fixtures {
    /// Réplique `make_md` du script de référence (entrée, pas résultat attendu).
    pub fn make_md(version: u8, size: usize, snu_ascii: &[u8; 14], fill: usize) -> Vec<u8> {
        let mut md: Vec<u8> = (0..size).map(|i| ((fill + i) & 0xFF) as u8).collect();
        md[0] = version;
        md[1] = 0;
        md[2..7].copy_from_slice(b"2.1.3");
        md[0x1A..0x1A + 14].copy_from_slice(snu_ascii);
        for i in 0..0x20 {
            md[0x40 + i] = (0xA0 + i) as u8;
        }
        if version <= 5 {
            md[18..22].copy_from_slice(&[0x83, 0x04, 0x41, 0xA3]); // VID/PID V2
        }
        md
    }

    pub fn md_v5() -> Vec<u8> { make_md(5, 512, b"0A1B2C3D4E5F60", 0x50) }
    pub fn md_v6() -> Vec<u8> { make_md(6, 128, b"0A1B2C3D4E5F60", 0x60) }
    pub fn md_v7() -> Vec<u8> { make_md(7, 112, b"23003400ABCDEF", 0x70) }
    pub fn md_v8() -> Vec<u8> { make_md(8, 128, b"11223344556677", 0x80) }

    pub fn sha256_hex(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(data))
    }
}

#[cfg(test)]
mod tests {
    use super::test_fixtures::*;
    use super::*;

    #[test]
    fn fixtures_match_reference_md_files() {
        // SHA-256 des .md passés au code de référence
        assert_eq!(sha256_hex(&md_v5()), "0f5a22732d9d8ffa5f8a19da77200dac3303a11ab559617a766ab461da624607");
        assert_eq!(sha256_hex(&md_v6()), "542a28c372421bb4a8e3c3981a1d4a8f76d8917544ffc95c44fa5d606f825a25");
        assert_eq!(sha256_hex(&md_v7()), "e5cbc0681b8eeaba49c6d0c882f75ef6a72a37f486f10a9c7c3341156dbca6b6");
        assert_eq!(sha256_hex(&md_v8()), "34438687753265b9d21071a702e1d5358fbc790accec461efb913dc4c7bdedce");
    }

    #[test]
    fn v6_keys_match_reference() {
        let k = parse_v3_md(&md_v6()).unwrap();
        assert_eq!(k.md_version, 6);
        assert_eq!(hex::encode(k.story_key), "62316130643363326635653400003036");
        assert_eq!(hex::encode(k.story_iv), "00000000000000006231613064336332");
        assert_eq!(
            hex::encode(k.bt),
            "a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebf"
        );
    }

    #[test]
    fn v7_keys_match_reference() {
        let k = parse_v3_md(&md_v7()).unwrap();
        assert_eq!(k.md_version, 7);
        assert_eq!(hex::encode(k.story_key), "a3a2a1a0a7a6a5a4abaaa9a8afaeadac");
        assert_eq!(hex::encode(k.story_iv), "b3b2b1b0b7b6b5b4bbbab9b8bfbebdbc");
        assert_eq!(
            hex::encode(k.bt),
            "3233303033343030616263646566000000000000000000003233303033343030"
        );
    }

    #[test]
    fn v8_is_refused_with_distinct_error() {
        let err = parse_v3_md(&md_v8()).unwrap_err();
        assert!(err.starts_with(ERR_V3_RECENT), "{err}");
        assert!(err.contains("v8"), "{err}");
        let mut md = md_v8();
        md[0] = 0x2A; // v42
        assert!(parse_v3_md(&md).unwrap_err().starts_with(ERR_V3_RECENT));
    }

    #[test]
    fn v6_v7_with_unexpected_size_are_refused() {
        // La référence ne dérive les clés v6/v7 que pour 112 ou 128 octets.
        for md in [make_md(6, 512, b"0A1B2C3D4E5F60", 0), make_md(7, 100, b"0A1B2C3D4E5F60", 0)] {
            let err = parse_v3_md(&md).unwrap_err();
            assert!(err.contains("non reconnu"), "{err}");
            assert!(!err.starts_with(ERR_V3_RECENT));
        }
        assert!(parse_v3_md(&make_md(6, 112, b"0A1B2C3D4E5F60", 0)).is_ok());
        assert!(parse_v3_md(&make_md(7, 128, b"0A1B2C3D4E5F60", 0)).is_ok());
        assert!(parse_v3_md(&[6]).is_err());
    }

    #[test]
    fn invalid_snu_is_refused() {
        let md = make_md(6, 128, b"0A1B2C3D4E5FZZ", 0);
        assert!(parse_v3_md(&md).unwrap_err().contains("numéro de série"));
    }

    #[test]
    fn snu_case_does_not_change_keys() {
        // unhexlify/hexlify : la référence normalise le SNU en minuscules.
        let upper = parse_v3_md(&make_md(7, 112, b"23003400ABCDEF", 0x70)).unwrap();
        let lower = parse_v3_md(&make_md(7, 112, b"23003400abcdef", 0x70)).unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn reverse_words_matches_reference() {
        let input: [u8; 16] = core::array::from_fn(|i| i as u8);
        assert_eq!(hex::encode(reverse_words(&input)), "03020100070605040b0a09080f0e0d0c");
    }
}
