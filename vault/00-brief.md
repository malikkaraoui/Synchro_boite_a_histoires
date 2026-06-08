# Brief projet

> ⛔ **RÈGLE 1 — ANTI-HALLUCINATION ABSOLUE**
> Interdiction totale d'inventer, de mentir, d'halluciner.
> Si incertain → « Je ne peux pas l'affirmer » + 2-3 hypothèses + comment vérifier.

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## État court

- Projet : **Synchro Boîte à histoires** (repo local actif de la synchro audio vers la boîte à histoires)
- Phase : **Production + Mac App Store en préparation** (dernière release : v2.1.12)
- Stack : **Tauri 2.0 (Rust) + Vanilla JS/HTML/CSS + Python sidecar (boite-bridge.py)**
- Objectif courant : **finaliser la variante Mac App Store** — validation device physique V2 ✅ FAITE
- Prochaine action utile : soumission App Store Connect (build universel signé)

## Deux variantes de distribution

| Variante      | Dossier            | Distribution                                          | État                                                                 |
|---------------|--------------------|-------------------------------------------------------|----------------------------------------------------------------------|
| Directe       | `src-tauri/`       | `.dmg` (macOS) + `.exe` (Windows) via GitHub Releases | ✅ En production                                                     |
| Mac App Store | `mac-app-store/`   | Soumission App Store Connect                          | 🟡 Prêt à soumettre — import V2 validé sur device physique           |

## Décisions actives

- Architecture hybride Rust+Python maintenue pour la variante directe (boite-bridge.py)
- Variante App Store : 100% Rust natif, zéro Python, zéro téléchargement runtime
- Identification device par `serial-*` (plus stable que UUID FAT32)
- Signature macOS ad-hoc (`-`) pour les builds directs
- Distribution via GitHub Releases avec updater intégré (variante directe uniquement)

## Risques / angles morts

- ~~Validation XXTEA sur device physique V2~~ ✅ Résolu (session 2026-06-03/05)
- Support boîte V3 (AES-128-CBC, md_version ≥ 6) non implémenté dans la variante App Store
- Builds Windows non testés en production (workflow GitHub Actions ajouté, non validé en réel)
