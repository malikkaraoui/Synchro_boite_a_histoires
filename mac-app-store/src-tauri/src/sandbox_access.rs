//! Accès sandbox App Store à la boîte et au dossier audio.
//!
//! Sous sandbox, `stat` d'un fichier hors conteneur est autorisé mais sa lecture est refusée
//! (`EPERM`) tant que l'utilisateur ne l'a pas choisi dans le NSOpenPanel. Ce choix ne vaut que
//! pour la durée du processus : on en garde un bookmark security-scoped (NSURL) dans les réglages,
//! que l'on résout au lancement suivant pour retrouver l'accès sans redemander.

use crate::storybox_device::{self, StoryBoxDeviceProbe};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Un accès security-scoped ouvert. `stopAccessingSecurityScopedResource` est appelé au drop.
pub struct ScopedAccess {
    #[cfg(target_os = "macos")]
    url: objc2::rc::Retained<objc2_foundation::NSURL>,
    #[cfg(target_os = "macos")]
    started: bool,
}

impl Drop for ScopedAccess {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if self.started {
            // SAFETY : appel apparié au `startAccessingSecurityScopedResource` réussi de `resolve`.
            unsafe { self.url.stopAccessingSecurityScopedResource() };
        }
    }
}

/// Résultat de la résolution d'un bookmark.
pub struct Resolved {
    pub path: PathBuf,
    /// Le bookmark est périmé : il faut le régénérer (l'accès, lui, est ouvert).
    pub stale: bool,
    pub access: ScopedAccess,
}

#[cfg(target_os = "macos")]
fn ns_error_text(err: &objc2_foundation::NSError) -> String {
    err.localizedDescription().to_string()
}

/// Crée un bookmark security-scoped pour un chemin auquel le processus a déjà accès
/// (sélection NSOpenPanel dans cette session, ou bookmark résolu).
#[cfg(target_os = "macos")]
pub fn create_bookmark(path: &Path) -> Result<Vec<u8>, String> {
    use objc2_foundation::{NSString, NSURL, NSURLBookmarkCreationOptions};

    let ns_path = NSString::from_str(&path.to_string_lossy());
    let url = NSURL::fileURLWithPath_isDirectory(&ns_path, path.is_dir());
    url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
        NSURLBookmarkCreationOptions::WithSecurityScope,
        None,
        None,
    )
    .map(|data| data.to_vec())
    .map_err(|e| format!("Création du bookmark échouée : {}", ns_error_text(&e)))
}

/// Résout un bookmark et ouvre l'accès security-scoped au chemin qu'il désigne.
/// Ne monte aucun volume et n'affiche aucune UI : une boîte débranchée échoue simplement.
#[cfg(target_os = "macos")]
pub fn resolve(bookmark: &[u8]) -> Result<Resolved, String> {
    use objc2::runtime::Bool;
    use objc2_foundation::{NSData, NSURLBookmarkResolutionOptions, NSURL};

    let data = NSData::with_bytes(bookmark);
    let mut is_stale = Bool::NO;
    let options = NSURLBookmarkResolutionOptions::WithSecurityScope
        | NSURLBookmarkResolutionOptions::WithoutUI
        | NSURLBookmarkResolutionOptions::WithoutMounting;
    // SAFETY : `is_stale` pointe vers une variable locale valide pendant tout l'appel.
    let url = unsafe {
        NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
            &data,
            options,
            None,
            &mut is_stale,
        )
    }
    .map_err(|e| format!("Bookmark non résolu : {}", ns_error_text(&e)))?;

    let path = url
        .path()
        .map(|p| PathBuf::from(p.to_string()))
        .ok_or_else(|| "Bookmark résolu sans chemin".to_string())?;
    // SAFETY : méthode sans précondition ; l'appel `stop…` apparié est fait au drop.
    let started = unsafe { url.startAccessingSecurityScopedResource() };
    Ok(Resolved {
        path,
        stale: is_stale.as_bool(),
        access: ScopedAccess { url, started },
    })
}

#[cfg(not(target_os = "macos"))]
pub fn create_bookmark(_path: &Path) -> Result<Vec<u8>, String> {
    Err("Bookmarks security-scoped disponibles sur macOS uniquement".to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn resolve(_bookmark: &[u8]) -> Result<Resolved, String> {
    Err("Bookmarks security-scoped disponibles sur macOS uniquement".to_string())
}

/// Boîte validée : son point de montage ET son identité (numéro de série) font foi pour
/// toutes les commandes device, qui ne reprobent plus `/Volumes`. Une autre boîte montée
/// au même endroit n'est pas la boîte validée.
pub struct DeviceAccess {
    pub mount: String,
    /// `serial-…` (ou identifiant de volume à défaut), lu sur la boîte à la validation.
    pub device_id: Option<String>,
    /// `None` quand l'accès vient du NSOpenPanel de cette session (ou hors sandbox).
    _scoped: Option<ScopedAccess>,
}

const ERR_NOT_ACCESSIBLE: &str =
    "La boîte n'est plus accessible : rebranchez-la, ou autorisez l'accès à la boîte.";
const ERR_OTHER_DEVICE: &str =
    "La boîte branchée n'est plus celle qui a été validée : rien n'a été écrit. Attendez qu'elle soit de nouveau détectée.";

/// État géré par Tauri : accès ouverts pendant la vie du processus.
#[derive(Default)]
pub struct SandboxAccess {
    device: Mutex<Option<DeviceAccess>>,
    audio: Mutex<Option<ScopedAccess>>,
}

impl SandboxAccess {
    pub fn validated_mount(&self) -> Option<String> {
        self.device.lock().ok()?.as_ref().map(|d| d.mount.clone())
    }

    /// Montage et identité de la boîte validée.
    pub fn validated_device(&self) -> Option<(String, Option<String>)> {
        self.device.lock().ok()?.as_ref().map(|d| (d.mount.clone(), d.device_id.clone()))
    }

    /// Mémorise la boîte validée. Un accès précédent vers un autre montage ou une autre
    /// boîte est refermé.
    pub fn set_device(&self, mount: String, device_id: Option<String>, scoped: Option<ScopedAccess>) {
        if let Ok(mut device) = self.device.lock() {
            if let Some(current) = device.as_mut() {
                if current.mount == mount && current.device_id == device_id && scoped.is_none() {
                    return; // même boîte : garde l'accès security-scoped déjà ouvert
                }
            }
            *device = Some(DeviceAccess { mount, device_id, _scoped: scoped });
        }
    }

    /// Resonde la boîte validée seule. `Some` si elle est toujours là, lisible, avec le même
    /// numéro de série ; sinon l'accès est refermé et `None` est renvoyé.
    pub fn reprobe_validated(&self) -> Option<StoryBoxDeviceProbe> {
        let (mount, device_id) = self.validated_device()?;
        let probe = storybox_device::probe_mount(Path::new(&mount));
        if probe.connected && device_id.is_some() && probe.device_id == device_id {
            return Some(probe);
        }
        // Débranchée, éjectée, redevenue illisible, ou autre boîte au même point de montage.
        self.clear_device();
        None
    }

    /// Boîte débranchée ou éjectée : referme l'accès (`stop…` au drop).
    pub fn clear_device(&self) {
        if let Ok(mut device) = self.device.lock() {
            *device = None;
        }
    }

    pub fn set_audio(&self, scoped: Option<ScopedAccess>) {
        if let Ok(mut audio) = self.audio.lock() {
            *audio = scoped;
        }
    }

    /// Fermeture de l'app : referme tous les accès.
    pub fn release_all(&self) {
        self.clear_device();
        self.set_audio(None);
    }

    /// Vérifie que la boîte désignée par le front (montage + identité) est la boîte validée,
    /// et que c'est bien elle qui est physiquement branchée : le numéro de série est relu.
    pub fn require_mount(&self, mount: &str, device_id: &str) -> Result<(), String> {
        let Some((validated_mount, validated_id)) = self.validated_device() else {
            return Err(ERR_NOT_ACCESSIBLE.to_string());
        };
        if validated_mount != mount {
            return Err(ERR_NOT_ACCESSIBLE.to_string());
        }
        if validated_id.as_deref() != Some(device_id)
            || storybox_device::read_device_id(Path::new(mount)).as_deref() != Some(device_id)
        {
            return Err(ERR_OTHER_DEVICE.to_string());
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::{create_bookmark, resolve};

    /// Hors sandbox, un bookmark security-scoped se crée et se résout vers le même dossier :
    /// exerce le pont objc2 réel (pas une simulation).
    #[test]
    fn bookmark_roundtrip_resolves_same_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().canonicalize().unwrap();
        let data = create_bookmark(&dir).expect("bookmark");
        assert!(!data.is_empty());

        let resolved = resolve(&data).expect("résolution");
        assert_eq!(resolved.path.canonicalize().unwrap(), dir);
        assert!(!resolved.stale);
    }

    #[test]
    fn resolve_rejects_garbage() {
        assert!(resolve(b"pas un bookmark").is_err());
    }
}

#[cfg(test)]
mod device_identity_tests {
    use super::SandboxAccess;
    use std::fs;
    use std::path::Path;

    /// `.md` V2 de 512 octets : le numéro de série est lu dans les octets 2 à 9.
    fn write_md_v2(mount: &Path, serial: [u8; 8]) {
        let mut md = vec![0u8; 512];
        md[0] = 3;
        md[2..10].copy_from_slice(&serial);
        fs::write(mount.join(".md"), md).unwrap();
    }

    const A: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    const B: [u8; 8] = [0x0F, 0xED, 0xCB, 0xA9, 0x87, 0x65, 0x43, 0x21];

    fn validated_box_a() -> (tempfile::TempDir, String, SandboxAccess) {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".content")).unwrap();
        write_md_v2(tmp.path(), A);
        let mount = tmp.path().to_string_lossy().into_owned();
        let access = SandboxAccess::default();
        access.set_device(mount.clone(), Some("serial-123456789ABCDEF0".to_string()), None);
        (tmp, mount, access)
    }

    #[test]
    fn same_box_is_accepted() {
        let (_tmp, mount, access) = validated_box_a();
        access.require_mount(&mount, "serial-123456789ABCDEF0").unwrap();
        let probe = access.reprobe_validated().expect("boîte A toujours validée");
        assert_eq!(probe.device_id.as_deref(), Some("serial-123456789ABCDEF0"));
    }

    #[test]
    fn other_serial_on_same_mount_is_refused() {
        let (tmp, mount, access) = validated_box_a();
        // Boîte B branchée au même point de montage, entre deux polls ou pendant une synchro.
        write_md_v2(tmp.path(), B);

        let err = access.require_mount(&mount, "serial-123456789ABCDEF0").unwrap_err();
        assert!(err.contains("n'est plus celle"), "{err}");
        // Le front qui annoncerait B est refusé aussi : B n'a jamais été validée.
        assert!(access.require_mount(&mount, "serial-0FEDCBA987654321").is_err());

        // Le chemin rapide de la détection ne renvoie pas B comme boîte validée.
        assert!(access.reprobe_validated().is_none());
        assert!(access.validated_device().is_none(), "accès refermé");
    }

    #[test]
    fn front_device_id_must_match_validated_one() {
        let (_tmp, mount, access) = validated_box_a();
        assert!(access.require_mount(&mount, "serial-0FEDCBA987654321").is_err());
        assert!(access.require_mount("/Volumes/AUTRE", "serial-123456789ABCDEF0").is_err());
    }

    #[test]
    fn unreadable_marker_is_refused() {
        let (tmp, mount, access) = validated_box_a();
        fs::remove_file(tmp.path().join(".md")).unwrap();
        assert!(access.require_mount(&mount, "serial-123456789ABCDEF0").is_err());
        assert!(access.reprobe_validated().is_none());
    }
}
