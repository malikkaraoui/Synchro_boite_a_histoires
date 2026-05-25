# Mailbox projet

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Courrier entrant

### 2026-05-25 — Session corrections prod + release v2.1.12 [auto]

- Source : Claude (claude-sonnet-4-6)
- Statut : archivé
- Résumé : 3 bugs corrigés et release v2.1.12 publiée sur GitHub. (1) Fausse détection device : `probe_mount_candidate` valide maintenant que le fichier `.md` fait ≥ 64 octets — empêche un DMG de l'app monté (ex: "LuniiSync") d'être confondu avec une boîte. (2) Boucle updater : `check_for_update` compare désormais la version GitHub à `CARGO_PKG_VERSION` et retourne `Err("already_up_to_date")` si l'app est déjà à jour — fin de la boucle infinie v2.1.11/v2.1.12. (3) Bundle identifier corrigé (underscores interdits par Apple). Commits : 93875cc, a646fb3, 59fab0b. Release GitHub : [v2.1.12](https://github.com/malikkaraoui/Synchro_boite_a_histoires/releases/tag/v2.1.12)
- Prochaine action : reprendre avec device physique V2 pour valider le crypto XXTEA (variante Mac App Store). Textes + visuels App Store prêts dans `mac-app-store/APP_STORE_CONTENT.md`.

### 2026-05-25 — Session Mac App Store [auto]

- Source : Claude (claude-sonnet-4-6)
- Statut : archivé
- Résumé : Session complète de préparation Mac App Store. Pipeline import natif Rust implémenté (storybox_crypto.rs + storybox_import.rs + start_sync_native). 7 bloqueurs App Store corrigés : gate des 3 spawns process interdits (diskutil info, df -k, diskutil eject), création PrivacyInfo.xcprivacy, correction versions, retrait network.client entitlement, bouton "Sélectionner la Lunii" + import audio débloqué dans le frontend. Renommage complet lunii_ vers storybox_ et lunii-\*.py vers boite-\*.py. 45/45 tests passent. Commits : 7f2f797, 0b50a0c, 8f9c642, 7b5c53d.
- Prochaine action : reprendre jeudi avec device physique V2 pour valider le crypto XXTEA. Textes App Store et visuels préparés pour soumission.

### 2026-05-22 — Session initialisation vault [auto]

- Source : Claude (claude-sonnet-4-6) — lecture complète des fichiers projet
- Statut : archivé
- Résumé : Projet Synchro Boîte à histoires v2.1.12 — application macOS Tauri 2.0 (Rust + Python sidecar) pour transférer des fichiers audio vers la boîte à histoires sans abonnement. Le projet est en phase production active, toutes les releases v2.0.0 → v2.1.12 ont été livrées le 2026-05-22. L'activité récente couvre : refonte drag-and-drop, réorganisation des histoires via `.pi`, repair d'index, UI sync inline, fiabilité I/O (retry/timeout), identification device par serial. La seule tâche planifiée restante est l'affichage des pochettes (TODO.md).
- Prochaine action : implémenter pochettes MP3 (tag APIC ou fichier image voisin)

### 2026-05-22 — Vault initialisé [auto]

- Source : setup-project-vaults.py
- Statut : archivé
- Résumé : Vault créé pour le projet Synchro_boite_a_histoires. Les sessions futures doivent alimenter ce fichier à chaque clôture significative.
- Prochaine action : première session → compléter vault/00-brief.md + vault/40-roadmap.md
