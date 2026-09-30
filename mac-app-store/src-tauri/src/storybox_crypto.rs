//! Chiffrement boîte à histoires — XXTEA avec la formule de rounds spécifique à StoryBox.QT.
//!
//! Différence critique : StoryBox.QT utilise `rounds = 1 + 52/n` (division entière),
//! PAS la formule XXTEA standard `6 + 52/n`.
//!
//! Sources : o-daneel/StoryBox.QT pkg/api/device_storybox.py + ifduyue/xxtea.

// ── Constantes ────────────────────────────────────────────────────────────────

/// Clé générique boîte à histoires (hardcodée dans StoryBox.QT, commune à tous les appareils V2).
const STORYBOX_GENERIC_KEY: [u32; 4] = [0x91BD7A0A, 0xA75440A9, 0xBBD49D6C, 0xE0DCC0E3];

const DELTA: u32 = 0x9e3779b9;

// ── Utilitaires de conversion ─────────────────────────────────────────────────

fn bytes_to_u32_le(data: &[u8]) -> Vec<u32> {
    data.chunks(4)
        .map(|c| {
            let mut buf = [0u8; 4];
            buf[..c.len()].copy_from_slice(c);
            u32::from_le_bytes(buf)
        })
        .collect()
}

fn u32_to_bytes_le(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

// ── XXTEA ─────────────────────────────────────────────────────────────────────

/// Calcule le nombre de rounds selon la formule StoryBox.QT : `int(1 + 52 / (len/4))`.
fn storybox_tea_rounds(buf_len: usize) -> usize {
    let n = buf_len / 4;
    if n < 2 {
        return 1;
    }
    1 + 52 / n
}

#[inline(always)]
fn mx(z: u32, y: u32, sum: u32, key: &[u32; 4], p: usize, e: usize) -> u32 {
    ((z.wrapping_shr(5) ^ y.wrapping_shl(2)).wrapping_add(y.wrapping_shr(3) ^ z.wrapping_shl(4)))
        ^ ((sum ^ y).wrapping_add(key[(p & 3) ^ e] ^ z))
}

pub fn xxtea_encrypt(v: &mut [u32], key: &[u32; 4], rounds: usize) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut sum: u32 = 0;
    let mut z = v[n - 1];
    for _ in 0..rounds {
        sum = sum.wrapping_add(DELTA);
        let e = ((sum >> 2) & 3) as usize;
        for p in 0..n - 1 {
            let y = v[p + 1];
            v[p] = v[p].wrapping_add(mx(z, y, sum, key, p, e));
            z = v[p];
        }
        let y = v[0];
        let p = n - 1;
        v[p] = v[p].wrapping_add(mx(z, y, sum, key, p, e));
        z = v[p];
    }
}

pub fn xxtea_decrypt(v: &mut [u32], key: &[u32; 4], rounds: usize) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut sum = (rounds as u32).wrapping_mul(DELTA);
    let mut y = v[0];
    for _ in 0..rounds {
        let e = ((sum >> 2) & 3) as usize;
        for p in (1..n).rev() {
            let z = v[p - 1];
            v[p] = v[p].wrapping_sub(mx(z, y, sum, key, p, e));
            y = v[p];
        }
        let z = v[n - 1];
        v[0] = v[0].wrapping_sub(mx(z, y, sum, key, 0, e));
        y = v[0];
        sum = sum.wrapping_sub(DELTA);
    }
}

// ── Chiffrement d'un buffer (premiers N octets) ───────────────────────────────

/// Chiffre les `enc_len` premiers octets d'un buffer avec XXTEA + la clé donnée.
/// Les octets restants sont copiés tels quels. Padde à un multiple de 4 pour XXTEA.
fn cipher_leading_bytes(data: &[u8], key: &[u32; 4], enc_len: usize) -> Vec<u8> {
    let actual = enc_len.min(data.len());
    if actual < 8 {
        // Moins de 2 mots u32 — pas assez pour XXTEA, retourne tel quel
        return data.to_vec();
    }
    let padded_len = (actual + 3) & !3;
    let mut block = vec![0u8; padded_len];
    block[..actual].copy_from_slice(&data[..actual]);

    let mut words = bytes_to_u32_le(&block);
    let rounds = storybox_tea_rounds(block.len());
    xxtea_encrypt(&mut words, key, rounds);

    let enc_bytes = u32_to_bytes_le(&words);
    let mut result = data.to_vec();
    result[..actual].copy_from_slice(&enc_bytes[..actual]);
    result
}

// ── API publique ──────────────────────────────────────────────────────────────

/// Chiffre les 512 premiers octets d'un fichier story (audio, image, ri, si, li)
/// avec la clé générique boîte à histoires.
pub fn cipher_story_data(data: &[u8]) -> Vec<u8> {
    cipher_leading_bytes(data, &STORYBOX_GENERIC_KEY, 512)
}

/// Génère le fichier `bt` (authorization token) pour un appareil V2.
/// `bt` = chiffrement des 64 premiers octets de `ri_data` avec la device key.
pub fn make_bt_v2(ri_data: &[u8], device_key: &[u32; 4]) -> Vec<u8> {
    let mut input = vec![0u8; 64];
    let copy_len = ri_data.len().min(64);
    input[..copy_len].copy_from_slice(&ri_data[..copy_len]);
    cipher_leading_bytes(&input, device_key, 64)
}

/// Chiffre un fichier story pour une boîte V3 : AES-128-CBC sur les 512 premiers octets.
///
/// Réplique exacte de StoryBox.QT `aes_cipher(buffer, key, iv, 0, 512)` :
/// - `enc_len = min(512, len)` ; les octets au-delà restent en clair ;
/// - si `enc_len` n'est pas multiple de 16 (seulement possible si `len < 512`), le buffer
///   est complété par des `0x00` jusqu'au multiple de 16 suivant : le fichier grandit ;
/// - pas de padding PKCS : CBC brut, IV réinitialisé pour chaque fichier.
pub fn cipher_story_data_v3(data: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Vec<u8> {
    use aes::cipher::{block_padding::NoPadding, BlockEncryptMut, KeyIvInit};

    let mut buf = data.to_vec();
    let mut enc_len = buf.len().min(512);
    if enc_len % 16 != 0 {
        let pad = 16 - buf.len() % 16;
        buf.resize(buf.len() + pad, 0);
        enc_len += pad;
    }
    if enc_len > 0 {
        cbc::Encryptor::<aes::Aes128>::new(key.into(), iv.into())
            .encrypt_padded_mut::<NoPadding>(&mut buf[..enc_len], enc_len)
            .expect("enc_len est un multiple de 16 : NoPadding ne peut pas échouer");
    }
    buf
}

/// Dérive la device key d'un appareil boîte à histoires V2 depuis le contenu binaire du fichier `.md`.
///
/// Algorithme (StoryBox.QT `__md1to5_parse`) :
/// 1. Lire 256 octets à l'offset 0x100 (`raw_devkey`)
/// 2. XXTEA-déchiffrer avec la clé générique, rounds = storybox_tea_rounds(256) = 1
/// 3. Swap : device_key = dec[8..16] + dec[0..8]
pub fn derive_v2_device_key(md_data: &[u8]) -> Result<[u32; 4], String> {
    if md_data.len() < 0x200 {
        return Err(format!(
            "Fichier .md trop court pour V2 (taille : {} octets, attendu ≥ 512)",
            md_data.len()
        ));
    }
    let raw = &md_data[0x100..0x200]; // 256 octets
    let mut words = bytes_to_u32_le(raw);
    let rounds = storybox_tea_rounds(raw.len()); // = 1
    xxtea_decrypt(&mut words, &STORYBOX_GENERIC_KEY, rounds);
    let dec = u32_to_bytes_le(&words);

    // Swap : [8..16] + [0..8]
    let mut key_bytes = [0u8; 16];
    key_bytes[..8].copy_from_slice(&dec[8..16]);
    key_bytes[8..].copy_from_slice(&dec[0..8]);

    let w = bytes_to_u32_le(&key_bytes);
    Ok([w[0], w[1], w[2], w[3]])
}

/// Retourne la version hardware boîte à histoires depuis l'octet 0 du fichier `.md`.
/// - md_version < 6 → V2 (XXTEA)
/// - md_version >= 6 → V3 (AES-128-CBC, voir `storybox_v3` : v6/v7 supportés, v8+ refusé)
pub fn md_hw_version(md_data: &[u8]) -> u8 {
    if md_data.is_empty() {
        return 0;
    }
    if md_data[0] >= 6 {
        3
    } else {
        2
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storybox_tea_rounds_matches_python_formula() {
        // Python: int(1 + 52 / (len/4))  [division flottante puis trunc]
        assert_eq!(storybox_tea_rounds(512), 1);  // n=128 → 1+52//128=1
        assert_eq!(storybox_tea_rounds(256), 1);  // n=64  → 1+52//64=1
        assert_eq!(storybox_tea_rounds(64),  4);  // n=16  → 1+52//16=4
        assert_eq!(storybox_tea_rounds(32),  7);  // n=8   → 1+52//8=7
        assert_eq!(storybox_tea_rounds(16),  14); // n=4   → 1+52//4=14
        assert_eq!(storybox_tea_rounds(8),   27); // n=2   → 1+52//2=27
    }

    #[test]
    fn xxtea_encrypt_decrypt_roundtrip() {
        let key: [u32; 4] = [0xDEADBEEF, 0xCAFEBABE, 0x12345678, 0x9ABCDEF0];
        let original: Vec<u32> = vec![0x11223344, 0x55667788, 0x99AABBCC, 0xDDEEFF00];
        let rounds = storybox_tea_rounds(original.len() * 4);

        let mut encrypted = original.clone();
        xxtea_encrypt(&mut encrypted, &key, rounds);
        assert_ne!(encrypted, original, "encrypt should change data");

        xxtea_decrypt(&mut encrypted, &key, rounds);
        assert_eq!(encrypted, original, "decrypt should restore original");
    }

    #[test]
    fn cipher_story_data_modifies_first_512_bytes() {
        let data = vec![0xABu8; 600];
        let result = cipher_story_data(&data);
        // Premiers 512 octets doivent avoir changé
        assert_ne!(&result[..512], &data[..512]);
        // Les octets après 512 restent inchangés
        assert_eq!(&result[512..], &data[512..]);
    }

    #[test]
    fn cipher_story_data_roundtrip_with_decrypt() {
        let key = STORYBOX_GENERIC_KEY;
        let original = b"Hello StoryBox World! This is a test audio data padding to 512+ bytes"
            .iter()
            .cycle()
            .take(600)
            .copied()
            .collect::<Vec<u8>>();

        let enc_len = 512_usize.min(original.len());
        let padded_len = (enc_len + 3) & !3;
        let mut block = vec![0u8; padded_len];
        block[..enc_len].copy_from_slice(&original[..enc_len]);

        let mut words_enc = bytes_to_u32_le(&block);
        let rounds = storybox_tea_rounds(block.len());
        xxtea_encrypt(&mut words_enc, &key, rounds);

        xxtea_decrypt(&mut words_enc, &key, rounds);
        let dec_bytes = u32_to_bytes_le(&words_enc);
        assert_eq!(&dec_bytes[..enc_len], &original[..enc_len]);
    }

    #[test]
    fn make_bt_v2_produces_64_bytes() {
        let ri_data = b"000\\ABCD1234000\\EFG12345";
        let key: [u32; 4] = [1, 2, 3, 4];
        let bt = make_bt_v2(ri_data, &key);
        assert_eq!(bt.len(), 64);
    }

    #[test]
    fn make_bt_v2_differs_from_input() {
        let ri_data = vec![0xABu8; 64];
        let key = STORYBOX_GENERIC_KEY;
        let bt = make_bt_v2(&ri_data, &key);
        assert_ne!(bt, ri_data);
    }

    #[test]
    fn derive_v2_device_key_requires_512_bytes() {
        let short = vec![0u8; 256];
        assert!(derive_v2_device_key(&short).is_err());
    }

    #[test]
    fn derive_v2_device_key_succeeds_with_512_bytes() {
        let md = vec![0u8; 512];
        let result = derive_v2_device_key(&md);
        assert!(result.is_ok(), "should succeed with 512-byte .md");
    }

    #[test]
    fn md_hw_version_v2_and_v3() {
        assert_eq!(md_hw_version(&[0x05]), 2); // md_version 5 → V2
        assert_eq!(md_hw_version(&[0x06]), 3); // md_version 6 → V3
        assert_eq!(md_hw_version(&[0x07]), 3); // md_version 7 → V3
        assert_eq!(md_hw_version(&[0x00]), 2); // md_version 0 → V2
        assert_eq!(md_hw_version(&[]), 0);     // vide
    }
}
