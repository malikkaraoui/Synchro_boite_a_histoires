# CARNET DE BORD — instantané (1 minute)

Dernière mise à jour : 2026-09-30 14:25 (orchestrateur Cowork) — chantier App Store Rust en cours

## Où on en est

- `main` = `8ce44ee` en local, `origin/main` = `ad56617`. Rien n'est mergé depuis. Branches poussées en attente de doublage : `chore/M0002-harnais-storybox`=a9cb2f3, `feat/M0003-appstore-rust`=a809030.
- Cible (fondateur, 2026-09-30) : **Mac App Store, 100 % Rust, zéro dépendance**. Spec : `vault/decisions/2026-09-30-SPEC-app-store-rust.md`. Audit : `vault/revues/2026-09-30-M0003-audit-app-store.md`.
- En vol : F01/M0004 (G2 accès sandbox à la boîte + G3 bundle id + E1/E2/E3/E6/E8/E9), branche `feat/M0004-sandbox-acces-boite`.
- Décisions actées 2026-09-30 : bundle id `com.malikkaraoui.synchro-boite-a-histoires` ; audio v1 MP3 + WAV ; **V3 obligatoire en v1** ; compte Apple Developer actif, pas de fiche ASC, aucun certificat Distribution.
- Échéance : boîte V3 physique chez le fondateur semaine du 2026-10-05.

## Références à ne pas toucher

- Aucune.

## Rappels

- ~111 fichiers diffèrent seulement par leurs droits (755) : jamais committés ; toujours `git -c core.fileMode=false`.
- Hook pre-push : rien sur `main` hors vault/ sans doublage GO.
- Variante directe (src-tauri + boite-bridge.py) : correctif StoryBox.QT sur 10373c2 non publié ; sur le Mac du fondateur, contournements manuels (pip, imagemagick, ~/.storybox-qt).
- Source de vérité : `vault/reprise/00_INDEX.md` + `vault/runtime/state.json` / `events.jsonl`
  — et, au-dessus d'eux, la preuve git rejouée à l'instant.
