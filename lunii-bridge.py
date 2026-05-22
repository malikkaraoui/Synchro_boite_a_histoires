#!/usr/bin/env python3
"""
lunii-bridge.py — V2 Python sidecar : génération de story packs + import Lunii.
Appelé par Tauri/Rust : python3 lunii-bridge.py <audio_folder> <device_mount>
Chaque ligne stdout est un objet JSON (type: "progress"|"error"|"done"|"stderr").
"""

import sys
import os
import json
import subprocess
import tempfile
import shutil
import hashlib
import io
import zipfile
import urllib.request
import platform
from pathlib import Path
from typing import Optional
from datetime import datetime, timezone

# ── Chemins ───────────────────────────────────────────────────────────────────
SCRIPT_DIR   = Path(__file__).resolve().parent
# Dépendances dans ~/.luniisync/ (répertoire utilisateur, toujours accessible en écriture)
DEPS_DIR     = Path.home() / ".luniisync"
DEPS_DIR.mkdir(exist_ok=True)
LUNII_QT_DIR = DEPS_DIR / "Lunii.QT"
SPG_BIN      = DEPS_DIR / "studio-pack-generator"
SPG_VERSION  = "0.5.14"
AUDIO_EXTS   = {".mp3", ".m4a", ".wav", ".ogg", ".flac"}


# ── Émission JSON ─────────────────────────────────────────────────────────────
def emit(msg_type: str, **kwargs) -> None:
    print(json.dumps({"type": msg_type, **kwargs}), flush=True)


# ── Bootstrap Lunii.QT ────────────────────────────────────────────────────────
def _bootstrap_lunii_qt() -> None:
    if LUNII_QT_DIR.is_dir():
        return
    emit("progress", step="setup", message="Clonage de Lunii.QT…")
    subprocess.run(
        ["git", "clone", "--quiet", "https://github.com/o-daneel/Lunii.QT.git", str(LUNII_QT_DIR)],
        check=True,
    )
    emit("progress", step="setup", message="Lunii.QT cloné.")


# ── Bootstrap SPG ─────────────────────────────────────────────────────────────
def _bootstrap_spg() -> None:
    if SPG_BIN.exists():
        return

    arch   = platform.machine().lower()
    system = platform.system().lower()

    if system == "darwin":
        tag = "aarch64-apple" if arch == "arm64" else "x86_64-apple"
    elif system == "windows":
        tag = "x86_64-windows"
    else:
        tag = "x86_64-linux"

    zip_name = f"studio-pack-generator-{SPG_VERSION}-{tag}.zip"
    bin_name = f"studio-pack-generator-{tag}"
    url = (
        f"https://github.com/jersou/studio-pack-generator/releases/"
        f"download/v{SPG_VERSION}/{zip_name}"
    )

    emit("progress", step="setup", message=f"Téléchargement studio-pack-generator {SPG_VERSION}…")
    with urllib.request.urlopen(url) as resp:  # noqa: S310 — URL vérifiée ci-dessus
        data = resp.read()

    with zipfile.ZipFile(io.BytesIO(data)) as z:
        z.extract(bin_name, DEPS_DIR)

    src = DEPS_DIR / bin_name
    src.rename(SPG_BIN)
    SPG_BIN.chmod(0o755)
    emit("progress", step="setup", message="studio-pack-generator prêt.")


# ── Patch lecture directe ─────────────────────────────────────────────────────
def _patch_direct_play(story_dir: Path) -> None:
    """Supprime le nœud TTS titre dans story.json pour lecture immédiate."""
    story_json_path = story_dir / "story.json"
    if not story_json_path.exists():
        return
    try:
        with open(story_json_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        stages = data.get("stageNodes", [])
        if stages and stages[0].get("type") == "stage" and not stages[0].get("audio"):
            stages[0].pop("okTransition", None)
        with open(story_json_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
    except Exception:
        pass  # Non bloquant


# ── Couverture PNG ────────────────────────────────────────────────────────────
def _inject_cover(story_dir: Path, title: str) -> None:
    """Génère une couverture PNG minimale si absente (Pillow optionnel)."""
    cover_path = story_dir / "thumbnail.png"
    if cover_path.exists():
        return
    try:
        from PIL import Image, ImageDraw  # type: ignore
        img = Image.new("RGB", (320, 240), color=(30, 80, 180))
        draw = ImageDraw.Draw(img)
        draw.text((20, 100), title[:40], fill=(255, 255, 255))
        img.save(cover_path)
    except Exception:
        pass  # Non bloquant ; SPG peut continuer sans couverture


# ── Génération ZIP via SPG ────────────────────────────────────────────────────
def _generate_zip(audio_path: Path, work_dir: Path) -> Optional[Path]:
    story_dir = work_dir / audio_path.stem
    story_dir.mkdir(parents=True, exist_ok=True)

    # Copier fichier audio
    shutil.copy2(audio_path, story_dir / audio_path.name)

    # Couverture optionnelle
    _inject_cover(story_dir, audio_path.stem)

    # Appel SPG
    env = os.environ.copy()
    if sys.platform == "darwin":
        # Homebrew ARM/Intel — résoudre ffmpeg
        env["PATH"] = "/opt/homebrew/bin:/usr/local/bin:" + env.get("PATH", "")

    out_dir = work_dir / "out"
    out_dir.mkdir(exist_ok=True)

    try:
        result = subprocess.run(
            [
                str(SPG_BIN),
                "--skip-extract-image-from-mp-3",
                "--output-folder", str(out_dir),
                str(story_dir),
            ],
            capture_output=True,
            text=True,
            env=env,
            timeout=180,
        )
    except subprocess.TimeoutExpired:
        emit("error", file=audio_path.name, message="Timeout studio-pack-generator (>180s)")
        return None
    except Exception as exc:
        emit("error", file=audio_path.name, message=str(exc))
        return None

    if result.returncode != 0:
        msg = result.stderr.strip() or result.stdout.strip() or "SPG a échoué"
        emit("error", file=audio_path.name, message=msg)
        return None

    # Trouver le ZIP généré
    zips = sorted(out_dir.glob("*.zip"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not zips:
        emit("error", file=audio_path.name, message="Aucun ZIP généré par SPG")
        return None

    return zips[0]


# ── Sidecar LuniiSync ─────────────────────────────────────────────────────────
def _write_sidecar(device, story_id: str, before_uuids: set) -> None:
    """Écrit .lunii-studio.json pour l'histoire nouvellement importée."""
    mount = Path(device.mount_point)
    content_dir = mount / ".content"
    if not content_dir.is_dir():
        return
    current_uuids = {d.name for d in content_dir.iterdir() if d.is_dir()}
    new_uuids = current_uuids - before_uuids
    if not new_uuids:
        return
    short_uuid = new_uuids.pop()
    sidecar_path = content_dir / short_uuid / ".lunii-studio.json"
    data = {
        "storyId": story_id,
        "hash": "",
        "pushedAt": datetime.now(timezone.utc).isoformat(),
        "source": "luniisync",
    }
    try:
        with open(sidecar_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
    except Exception:
        pass


# ── Import vers device ────────────────────────────────────────────────────────
def _import_one(device, audio_path: Path, work_dir: Path) -> bool:
    zip_path = _generate_zip(audio_path, work_dir)
    if zip_path is None:
        return False

    # Patch lecture directe sur le ZIP extrait
    with tempfile.TemporaryDirectory(prefix="lunii-patch-") as patch_tmp:
        patch_dir = Path(patch_tmp)
        import zipfile as zf
        with zf.ZipFile(zip_path, "r") as z:
            z.extractall(patch_dir)
        _patch_direct_play(patch_dir)
        # Remballer
        patched_zip = zip_path.with_suffix(".patched.zip")
        with zf.ZipFile(patched_zip, "w", zf.ZIP_DEFLATED) as z:
            for p in patch_dir.rglob("*"):
                if p.is_file():
                    z.write(p, p.relative_to(patch_dir))
        zip_path = patched_zip

    # Vérifier que le device est encore accessible avant d'appeler import_story
    mount = Path(device.mount_point)
    if not mount.is_dir():
        emit("error", file=audio_path.name, message="Boîte déconnectée — rebranchez et relancez.")
        return False

    content_dir = mount / ".content"
    before_uuids = {d.name for d in content_dir.iterdir() if d.is_dir()} if content_dir.is_dir() else set()

    import concurrent.futures as _cf
    import errno as _errno

    def _do_import():
        return device.import_story(str(zip_path))

    try:
        with _cf.ThreadPoolExecutor(max_workers=1) as ex:
            future = ex.submit(_do_import)
            try:
                ok = future.result(timeout=120)
            except _cf.TimeoutError:
                emit("error", file=audio_path.name, message="Timeout import (>120s) — boîte trop lente ou déconnectée.")
                return False
        if ok:
            _write_sidecar(device, audio_path.stem, before_uuids)
        return bool(ok)
    except OSError as exc:
        if exc.errno == _errno.EIO:
            emit("error", file=audio_path.name, message="Boîte éjectée pendant le transfert — rebranchez et relancez.")
        else:
            emit("error", file=audio_path.name, message=str(exc))
        return False
    except Exception as exc:
        emit("error", file=audio_path.name, message=str(exc))
        return False


# ── Réparation index ──────────────────────────────────────────────────────────
def _repair_index(device_mount: Path) -> None:
    """Recrée le fichier .pi à partir du contenu de .content/"""
    if not device_mount.is_dir():
        emit("error", message=f"Montage Lunii introuvable : {device_mount}")
        sys.exit(1)

    _bootstrap_lunii_qt()
    sys.path.insert(0, str(LUNII_QT_DIR))
    try:
        try:
            from PySide6.QtCore import QCoreApplication  # type: ignore
        except ImportError:
            from PyQt6.QtCore import QCoreApplication    # type: ignore
        if QCoreApplication.instance() is None:
            _q_app = QCoreApplication(sys.argv[:1])
        from pkg.api.device_lunii import LuniiDevice  # type: ignore
    except ImportError as exc:
        emit("error", message=f"Import Lunii.QT échoué : {exc}")
        sys.exit(1)

    try:
        device = LuniiDevice(str(device_mount))
        device.update_pack_index()
        emit("done", added=0, errors=0, message="Index réparé — redémarre la Lunii.")
    except Exception as exc:
        emit("error", message=f"Réparation échouée : {exc}")
        sys.exit(1)


# ── Point d'entrée ────────────────────────────────────────────────────────────
def main() -> None:
    # Mode réparation d'index
    if len(sys.argv) >= 3 and sys.argv[1] == "--repair-index":
        _repair_index(Path(sys.argv[2]))
        return

    if len(sys.argv) < 3:
        emit("error", message=f"Usage: {sys.argv[0]} <audio_folder> <device_mount>")
        sys.exit(1)

    audio_folder  = Path(sys.argv[1])
    device_mount  = Path(sys.argv[2])

    if not audio_folder.is_dir():
        emit("error", message=f"Dossier audio introuvable : {audio_folder}")
        sys.exit(1)
    if not device_mount.is_dir():
        emit("error", message=f"Montage Lunii introuvable : {device_mount}")
        sys.exit(1)

    # ── Setup ─────────────────────────────────────────────────────────────────
    try:
        _bootstrap_lunii_qt()
        _bootstrap_spg()
    except Exception as exc:
        emit("error", message=f"Setup échoué : {exc}")
        sys.exit(1)

    # ── Charger Lunii.QT ──────────────────────────────────────────────────────
    sys.path.insert(0, str(LUNII_QT_DIR))
    try:
        # QCoreApplication est requis par Lunii.QT avant tout import de pkg.*
        try:
            from PySide6.QtCore import QCoreApplication  # type: ignore
        except ImportError:
            from PyQt6.QtCore import QCoreApplication    # type: ignore
        if QCoreApplication.instance() is None:
            _q_app = QCoreApplication(sys.argv[:1])

        from pkg.api.device_lunii import LuniiDevice  # type: ignore  # noqa: E402
    except ImportError as exc:
        emit("error", message=f"Import Lunii.QT échoué : {exc}")
        sys.exit(1)

    # ── Charger device ────────────────────────────────────────────────────────
    try:
        device = LuniiDevice(str(device_mount))
    except Exception as exc:
        emit("error", message=f"Chargement device échoué : {exc}")
        sys.exit(1)

    # ── Scanner dossier audio ─────────────────────────────────────────────────
    if len(sys.argv) > 3:
        # Fichiers spécifiques passés en arguments (sélection utilisateur)
        audio_files = sorted(
            Path(p) for p in sys.argv[3:]
            if Path(p).is_file() and Path(p).suffix.lower() in AUDIO_EXTS
        )
    else:
        # Fallback : scanner tout le dossier
        audio_files = sorted(
            p for p in audio_folder.iterdir()
            if p.is_file() and p.suffix.lower() in AUDIO_EXTS
        )

    if not audio_files:
        emit("done", added=0, errors=0, message="Aucun fichier audio trouvé.")
        return

    emit("progress", step="scan",
         message=f"{len(audio_files)} fichier(s) audio trouvé(s).")

    # ── Import ────────────────────────────────────────────────────────────────
    added  = 0
    errors = 0

    with tempfile.TemporaryDirectory(prefix="luniisync-") as tmp:
        for i, audio_path in enumerate(audio_files, 1):
            work_dir = Path(tmp) / audio_path.stem
            work_dir.mkdir(exist_ok=True)

            emit("progress", step="import",
                 file=audio_path.name,
                 current=i, total=len(audio_files),
                 message=f"[{i}/{len(audio_files)}] {audio_path.name}…")

            ok = _import_one(device, audio_path, work_dir)
            if ok:
                added += 1
                emit("progress", step="import",
                     file=audio_path.name, current=i, total=len(audio_files),
                     message=f"✓ {audio_path.name}")
            else:
                errors += 1

    # ── Mise à jour index ─────────────────────────────────────────────────────
    try:
        device.update_pack_index()
    except OSError as exc:
        import errno as _errno
        if exc.errno == _errno.EIO:
            emit("error", message="Boîte éjectée pendant la synchronisation — rebranchez et relancez.")
        else:
            emit("error", message=f"update_pack_index échoué : {exc}")
    except Exception as exc:
        emit("error", message=f"update_pack_index échoué : {exc}")

    emit("done", added=added, errors=errors,
         message=f"Terminé : {added} ajouté(s), {errors} erreur(s).")


if __name__ == "__main__":
    main()
