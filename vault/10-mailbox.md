# Mailbox projet

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Courrier entrant

### 2026-05-25 — Session Mac App Store [auto]

- Source : Claude (claude-sonnet-4-6)
- Statut : archivé
- Résumé : Session complète de préparation Mac App Store. Pipeline import natif Rust implémenté (storybox_crypto.rs + storybox_import.rs + start_sync_native). 7 bloqueurs App Store corrigés : gate des 3 spawns process interdits (diskutil info, df -k, diskutil eject), création PrivacyInfo.xcprivacy, correction versions, retrait network.client entitlement, bouton "Sélectionner la Lunii" + import audio débloqué dans le frontend. Renommage complet lunii_ vers storybox_ et lunii-*.py vers boite-*.py. 45/45 tests passent. Commits : 7f2f797, 0b50a0c, 8f9c642, 7b5c53d.
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
