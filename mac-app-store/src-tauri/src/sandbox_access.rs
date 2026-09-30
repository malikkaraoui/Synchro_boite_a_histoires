//! Accès sandbox App Store à la boîte et au dossier audio.
//!
//! Sous sandbox, `stat` d'un fichier hors conteneur est autorisé mais sa lecture est refusée
//! (`EPERM`) tant que l'utilisateur ne l'a pas choisi dans le NSOpenPanel. Ce choix ne vaut que
//! pour la durée du processus : on en garde un bookmark security-scoped (NSURL) dans les réglages,
//! que l'on résout au lancement suivant pour retrouver l'accès sans redemander.

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

/// Boîte validée : son point de montage fait foi pour toutes les commandes device,
/// qui ne reprobent plus `/Volumes`.
pub struct DeviceAccess {
    pub mount: String,
    /// `None` quand l'accès vient du NSOpenPanel de cette session (ou hors sandbox).
    _scoped: Option<ScopedAccess>,
}

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

    /// Mémorise la boîte validée. Un accès précédent vers un autre montage est refermé.
    pub fn set_device(&self, mount: String, scoped: Option<ScopedAccess>) {
        if let Ok(mut device) = self.device.lock() {
            if let Some(current) = device.as_mut() {
                if current.mount == mount && scoped.is_none() {
                    return; // garde l'accès security-scoped déjà ouvert
                }
            }
            *device = Some(DeviceAccess { mount, _scoped: scoped });
        }
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

    /// Vérifie qu'un montage transmis par le front est bien la boîte validée.
    pub fn require_mount(&self, mount: &str) -> Result<(), String> {
        match self.validated_mount() {
            Some(m) if m == mount => Ok(()),
            _ => Err("La boîte n'est plus accessible : rebranchez-la, ou autorisez l'accès à la boîte.".to_string()),
        }
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
