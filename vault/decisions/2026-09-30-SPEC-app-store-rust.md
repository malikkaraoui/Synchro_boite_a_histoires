---
date: 2026-09-30
tags: [synchro-boite, spec, app-store, rust]
statut: actif
auteur: orchestrateur (Cowork), décision fondateur 2026-09-30
---

# SPEC — Synchro Boîte à histoires, Mac App Store, 100 % Rust, zéro dépendance

## Décision (fondateur, 2026-09-30, verbatim)
« On vise l'App Store ! version RUST ! AUCUNE dépendance ! »

## Critère d'acceptation produit (le seul qui compte)
Un utilisateur non technique installe l'app depuis le Mac App Store, branche sa boîte, choisit un MP3, clique « Synchroniser », et l'histoire se lit sur la boîte.
**Aucun terminal, aucun Python, aucun Homebrew, aucun téléchargement de code au runtime.**

## Périmètre : dossier `mac-app-store/` uniquement
La variante directe (`src-tauri/` + `boite-bridge.py`) n'est pas touchée par ce chantier.

## Déjà fait — [VÉRIFIÉ] dans le dépôt au 2026-09-30
| Brique | Fichier | Preuve |
|---|---|---|
| Détection boîte, inventaire, ordre, réparation index `.pi` | `storybox_device.rs` | commit 7f2f797 et suivants |
| Scan audio, hashes, sidecars, suppressions | `storybox_sync.rs` | idem |
| Post-traitement ZIP STUdio (couverture placeholder) | `story_pack.rs` | idem |
| Parsing `story.json`, buffers ri/si/li/ni | `studio_story.rs` | idem |
| Chiffrement XXTEA V2, dérivation clé device, `bt` | `storybox_crypto.rs` | NATIVE_IMPORT.md |
| MP3 → pack simple (remplace studio-pack-generator), import V2 complet + rollback | `storybox_import.rs` | NATIVE_IMPORT.md |
| Import V2 lisible sur boîte physique | — | commit 9ca90be (2026-06-03), vault/40-roadmap.md |
| Sandbox + entitlements `app-sandbox`, `files.user-selected.read-write`, `device.usb` | `boite-app-store.entitlements` | fichier |
| PrivacyInfo.xcprivacy, textes App Store | `PrivacyInfo.xcprivacy`, `APP_STORE_CONTENT.md` | commit a9c724d |
| **Travail NON COMMITÉ** (worktree racine) : retrait `boite-bridge.py`, feature `mac-app-store`, `reqwest`, `open` ; `main.rs` 724 → 333 lignes | `mac-app-store/…` | `git diff` au 2026-09-30 |

## Reste à faire — écarts (G = gap)
| # | Écart | Nature | Bloquant App Store ? |
|---|---|---|---|
| G1 | Travail non commité (retrait Python) : compile ? tests ? | vérif + commit | oui |
| G2 | **Accès à la boîte en sandbox** : la détection liste `/Volumes` (`storybox_device.rs:498`). [HYPOTHÈSE] refusé en sandbox → il faut un sélecteur du volume (NSOpenPanel via `tauri-plugin-dialog`) + **bookmark security-scoped persistant** pour ne pas redemander à chaque lancement | code Rust + UX | **oui, critique** |
| G3 | **Identifiant de bundle invalide** : `com.malikkaraoui.synchro_boite_a_histoires` (appstore conf) et `com.synchro_boite_a_histoires.app` (tauri.conf.json) contiennent `_`. [MÉMOIRE] Apple n'accepte que A-Z a-z 0-9 `-` `.` | config — **décision fondateur** (identifiant définitif App Store Connect) | oui |
| G4 | **Formats audio** : seul MP3 accepté, envoyé tel quel (pas de ré-encodage). [HYPOTHÈSE] un MP3 stéréo 48 kHz / VBR / avec pochette ID3 peut ne pas se lire sur la boîte. Matrice de test requise ; si échec → décodage + ré-encodage en Rust compilé dans l'app | test puis code | à mesurer |
| G5 | Pochette : placeholder PNG. Extraire le tag APIC du MP3 (crate `id3`) | code | non (qualité) |
| G6 | Boîtes V3 (AES-128-CBC, `md[0] >= 6`) non gérées : erreur explicite actuellement | code — **décision fondateur** (v1 V2 seule ?) | selon cible |
| G7 | Chaîne de soumission : compte Apple Developer, fiche App Store Connect, profil de provisioning Mac App Store, certificats « Apple Distribution » + « Mac Installer Distribution », build `universal-apple-darwin`, `.pkg` signé, upload (Transporter) | process + secrets — **fondateur** | oui |
| G8 | Validation sur machine propre (compte macOS neuf, sans Homebrew/Python) du build signé sandboxé | test | oui |
| G9 | Réseau : aucun appel réseau attendu → retirer toute trace (entitlement `network.client` absent au 2026-09-30, à reconfirmer) | vérif | oui (review) |
| G10 | Risque App Review [HYPOTHÈSE] : interopérabilité avec un matériel tiers via protocole rétro-ingénieré + marque tierce. Libellés sans marque tierce (renommage Lunii → fait, commit 8f9c642) | revue textes | risque rejet |

## Ordre du chantier (mandats)
1. **M0003** — finir le git (commit harnais), compiler + tester `mac-app-store`, committer le travail en cours, audit factuel G1–G10 avec preuves. *(ce mandat)*
2. **M0004** — G2 sandbox : sélection du volume + bookmark security-scoped ; build signé ad hoc avec entitlements pour test fondateur.
3. **M0005** — G4 matrice audio sur boîte physique (fondateur branche la boîte) ; ré-encodage natif si nécessaire.
4. **M0006** — G3 + G7 : identifiant définitif, script de build `.pkg` universel signé, checklist App Store Connect.
5. G5 / G6 selon décisions.
Chaque mandat : branche dédiée → doublage R0xx → merge.

## Questions ouvertes (au fondateur, non tranchées ici)
- Q1 (G3) identifiant de bundle définitif ? (ex. `com.malikkaraoui.synchro-boite-a-histoires`)
- Q2 (G6) v1 App Store = boîtes V2 seulement, ou V3 obligatoire ?
- Q3 (G7) compte Apple Developer actif ? fiche App Store Connect existante ?
- Q4 (G4) MP3 seulement en v1, ou aussi m4a/wav/ogg/flac ?
