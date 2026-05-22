<div align="center">
  <img src="src/logo.png" width="120" alt="LuniiSync Logo"/>
  <h1>LuniiSync</h1>
  <p><strong>Transférez vos histoires audio sur  "Ma Fabrique à Histoires" — facilement, sans abonnement.</strong></p>
  <p><em>La conteuse mobile pour vos histoires et podcasts préférés... et bien plus !</em></p>
  <p>
    <img src="https://img.shields.io/badge/version-2.0.0-00b49e?style=flat-square"/>
    <img src="https://img.shields.io/badge/plateforme-macOS-lightgrey?style=flat-square&logo=apple"/>
    <img src="https://img.shields.io/badge/Tauri-2.0-blue?style=flat-square"/>
    <img src="https://img.shields.io/badge/Rust-%23000000?style=flat-square&logo=rust"/>
    <img src="https://img.shields.io/badge/Python-3.10+-yellow?style=flat-square&logo=python"/>
  </p>
</div>

---

## Présentation

LuniiSync est une application de bureau macOS permettant de transférer des fichiers audio personnels (MP3, M4A, WAV, OGG, FLAC) vers une **Fabrique à Histoires Lunii**, sans passer par le logiciel officiel.

L'application génère automatiquement un story-pack compatible Lunii à partir de vos fichiers audio, puis l'importe directement sur votre boîte.

## Fonctionnalités

- **Détection automatique** de la Lunii branchée en USB
- **Transfert ciblé** — sélectionnez uniquement les histoires à transférer
- **Suppression** d'histoires directement depuis l'interface
- **Noms personnalisés** par boîte (identifiées par UUID volume)
- **Mode sombre / clair / automatique**
- **Splash screen** avec vérification automatique des mises à jour au démarrage
- **Informations firmware** (HW version, FW major.minor.subminor)
- **Noms lisibles** pour les histoires importées (depuis sidecar `.lunii-studio.json`)
- **Journal de synchronisation** intégré avec animation de progression en temps réel

## Stack technique

| Couche | Technologie | Rôle |
|--------|-------------|------|
| App desktop | **Tauri 2.0** + Rust | Fenêtre native, commandes système |
| Détection device | **Rust** | Lecture UUID volume, inventaire `.content/` |
| Dédup & sidecar | **Rust** | Hash SHA-256, `.lunii-studio.json` |
| Génération story pack | **Python** (lunii-bridge.py) | SPG + Lunii.QT (crypto Lunii) |
| Frontend | **Vanilla JS/HTML/CSS** | UI deux colonnes, sans framework |

## Prérequis

- macOS 12+ (Apple Silicon ou Intel)
- Python 3.10+ avec PySide6 installé
- Connexion Internet au premier transfert (télécharge SPG et Lunii.QT automatiquement)

```bash
pip3 install PySide6 psutil py7zr xxtea pycryptodome
```

## Installation

1. Téléchargez `LuniiSync.app` depuis la [dernière release](https://github.com/malikkaraoui/Lunii_Synchro/releases/latest)
2. Copiez-la dans `/Applications` ou sur votre bureau
3. Lancez — les dépendances Python sont téléchargées automatiquement au premier transfert

## Utilisation

1. **Branchez** votre Lunii en USB — détectée automatiquement
2. **Nommez** votre boîte au premier branchement (max 15 caractères)
3. **Sélectionnez** un dossier audio via "Parcourir…"
4. **Cochez** les fichiers à transférer (ou "+ Tout ajouter")
5. Cliquez **Synchroniser** — animation de progression en temps réel
6. Pour **supprimer** une histoire : cliquez 🗑 sur la ligne, puis Synchroniser

## Structure du projet

```
Lunii_Synchro/
├── src/                    # Frontend (HTML/CSS/JS)
│   ├── index.html
│   ├── main.js
│   ├── styles.css
│   └── logo.png
├── src-tauri/              # Backend Rust + config Tauri
│   ├── src/
│   │   ├── main.rs         # Point d'entrée + commandes Tauri
│   │   ├── lunii_device.rs # Détection + inventaire Lunii
│   │   ├── lunii_sync.rs   # Scan audio + hash + sidecar
│   │   └── app_settings.rs # Persistance réglages
│   ├── icons/              # Icônes app (logo fourni)
│   └── tauri.conf.json
├── lunii-bridge.py         # Sidecar Python (génération + import story pack)
├── TODO.md                 # Fonctionnalités à venir
└── requirements.txt
```

## Mises à jour

LuniiSync vérifie automatiquement les mises à jour au démarrage via les [GitHub Releases](https://github.com/malikkaraoui/Lunii_Synchro/releases). Le bouton **"Rechercher une mise à jour"** est également disponible dans les Réglages (⚙).

## Crédits

Développé par **Malik Karaoui**

Basé sur les travaux open source de :
- [Lunii.QT](https://github.com/o-daneel/Lunii.QT) — API Python Lunii
- [studio-pack-generator](https://github.com/jersou/studio-pack-generator) — Génération de story packs

---

*Usage personnel uniquement. Non affilié à Lunii SAS.*
