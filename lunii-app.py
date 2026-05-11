#!/usr/bin/env python3
"""
lunii-app.py — Interface graphique Lunii Sync
• Mode dev   : python3 lunii-app.py  (auto-installe les dépendances)
• Mode app   : LuniiSync.app  (tout est embarqué, aucune installation)
"""

import sys
import subprocess
import importlib.util
import os
import platform
import urllib.request
import urllib.error
from pathlib import Path

# ── Chemins (dev vs app bundle) ───────────────────────────────────────────────
_FROZEN = getattr(sys, "frozen", False)
_BUNDLE_DIR = Path(sys._MEIPASS) if _FROZEN else Path(__file__).parent

SCRIPT_DIR   = Path(__file__).parent if not _FROZEN else Path(sys.executable).parent.parent.parent
LUNII_QT_PATH = _BUNDLE_DIR / "Lunii.QT"
_spg_name    = "studio-pack-generator.exe" if platform.system() == "Windows" else "studio-pack-generator"
SPG_BINARY   = _BUNDLE_DIR / _spg_name
SPG_VERSION  = "0.5.14"

# Dossier de données utilisateur (manifests) — toujours dans ~/Library en mode app
if _FROZEN and platform.system() == "Darwin":
    _DATA_DIR = Path.home() / "Library" / "Application Support" / "LuniiSync"
else:
    _DATA_DIR = SCRIPT_DIR

# ── 1. Bootstrap Python deps (mode dev uniquement) ───────────────────────────
_PACKAGES = [
    ("PySide6",   "PySide6-Essentials"),
    ("psutil",    "psutil"),
    ("xxtea",     "xxtea"),
    ("requests",  "requests"),
    ("Crypto",    "pycryptodome"),
    ("PIL",       "Pillow"),
    ("mutagen",   "mutagen"),
    ("ffmpeg",    "ffmpeg-python"),
    ("unidecode", "unidecode"),
    ("py7zr",     "py7zr"),
]

def _bootstrap_python():
    missing = [pip for mod, pip in _PACKAGES if not importlib.util.find_spec(mod)]
    if not missing:
        return
    print(f"⏳  Installation : {', '.join(missing)}")
    subprocess.run([sys.executable, "-m", "pip", "install", "--quiet"] + missing, check=True)
    print("✅  Redémarrage...")
    os.execv(sys.executable, [sys.executable] + sys.argv)

if not _FROZEN:
    _bootstrap_python()

# ── 2. Imports principaux ─────────────────────────────────────────────────────
import shutil, tempfile, json, zipfile, hashlib, io, logging
from PySide6.QtCore import Qt, QThread, QObject, Signal, QTimer
from PySide6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QPushButton, QFileDialog, QTextEdit, QProgressBar,
    QGroupBox, QFrame, QDialog,
)
from PySide6.QtGui import QFont

sys.path.insert(0, str(LUNII_QT_PATH))
logging.basicConfig(level=logging.WARNING)

# ── 3. Setup automatique (Lunii.QT + SPG) ────────────────────────────────────
# (zip_filename, binary_path_inside_zip)
_SPG_TABLE = {
    ("Darwin",  "arm64"):   (f"studio-pack-generator-{SPG_VERSION}-aarch64-apple.zip",  "studio-pack-generator-aarch64-apple"),
    ("Darwin",  "x86_64"):  (f"studio-pack-generator-{SPG_VERSION}-x86_64-apple.zip",   "studio-pack-generator-x86_64-apple"),
    ("Windows", "AMD64"):   (f"studio-pack-generator-{SPG_VERSION}-x86_64-windows.zip", "Studio-Pack-Generator/studio-pack-generator-x86_64-windows.exe"),
}


class SetupWorker(QObject):
    progress = Signal(str)
    finished = Signal(bool, str)

    def run(self):
        try:
            if not LUNII_QT_PATH.exists():
                self.progress.emit("Clonage de Lunii.QT…")
                subprocess.run(
                    ["git", "clone", "--quiet",
                     "https://github.com/o-daneel/Lunii.QT.git", str(LUNII_QT_PATH)],
                    check=True,
                )
                self.progress.emit("Lunii.QT cloné.")

            if not SPG_BINARY.exists():
                import tempfile, zipfile, shutil
                key = (platform.system(), platform.machine())
                entry = _SPG_TABLE.get(key)
                if not entry:
                    self.finished.emit(False, f"Plateforme non supportée : {key}")
                    return
                zip_name, bin_in_zip = entry
                url = (f"https://github.com/jersou/studio-pack-generator"
                       f"/releases/download/v{SPG_VERSION}/{zip_name}")
                self.progress.emit("Téléchargement de studio-pack-generator…")

                def _hook(b, bs, total):
                    if total > 0:
                        self.progress.emit(f"Téléchargement… {min(b*bs*100//total, 100)}%")

                with tempfile.NamedTemporaryFile(suffix=".zip", delete=False) as tmp:
                    tmp_zip = Path(tmp.name)
                urllib.request.urlretrieve(url, tmp_zip, reporthook=_hook)
                self.progress.emit("Extraction…")
                with tempfile.TemporaryDirectory() as tmp_dir:
                    with zipfile.ZipFile(tmp_zip) as zf:
                        zf.extractall(tmp_dir)
                    src_bin = Path(tmp_dir) / Path(bin_in_zip)
                    shutil.move(str(src_bin), str(SPG_BINARY))
                    if platform.system() == "Windows":
                        src_tools = Path(tmp_dir) / "Studio-Pack-Generator" / "tools"
                        dst_tools = SPG_BINARY.parent / "tools"
                        if dst_tools.exists():
                            shutil.rmtree(dst_tools)
                        shutil.move(str(src_tools), str(dst_tools))
                tmp_zip.unlink(missing_ok=True)
                if platform.system() != "Windows":
                    SPG_BINARY.chmod(0o755)
                self.progress.emit("studio-pack-generator prêt.")

            self.finished.emit(True, "")
        except Exception as e:
            self.finished.emit(False, str(e))


class SetupDialog(QDialog):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Lunii Sync — Installation")
        self.setMinimumWidth(400)
        self.setWindowFlag(Qt.WindowType.WindowCloseButtonHint, False)
        lay = QVBoxLayout(self)
        lay.setSpacing(10)
        lay.setContentsMargins(20, 20, 20, 20)
        lay.addWidget(QLabel("Installation des composants…"))
        self._status = QLabel("Démarrage…")
        self._status.setWordWrap(True)
        lay.addWidget(self._status)
        self._bar = QProgressBar()
        self._bar.setRange(0, 0)
        lay.addWidget(self._bar)
        self._close_btn = QPushButton("Fermer")
        self._close_btn.setVisible(False)
        self._close_btn.clicked.connect(self.reject)
        lay.addWidget(self._close_btn)

        self._thread = QThread(self)
        self._worker = SetupWorker()
        self._worker.moveToThread(self._thread)
        self._thread.started.connect(self._worker.run)
        self._worker.progress.connect(self._status.setText)
        self._worker.finished.connect(self._on_done)
        self._thread.start()

    def _on_done(self, ok: bool, err: str):
        self._thread.quit()
        self._bar.setRange(0, 1)
        self._bar.setValue(1)
        if ok:
            self._status.setText("✅  Prêt.")
            QTimer.singleShot(700, self.accept)
        else:
            self._status.setText(f"❌  {err}")
            self._close_btn.setVisible(True)


# ── 4. Fonctions de synchronisation ──────────────────────────────────────────

def find_lunii() -> str | None:
    try:
        from pkg.api.device_lunii import is_lunii
        import psutil
        for part in psutil.disk_partitions(all=False):
            try:
                if is_lunii(part.mountpoint):
                    return part.mountpoint
            except (PermissionError, OSError):
                continue
    except Exception:
        pass
    # Fallback
    system = platform.system()
    if system == "Darwin":
        cands = list(Path("/Volumes").iterdir()) if Path("/Volumes").exists() else []
    elif system == "Linux":
        import getpass
        roots = [Path("/media") / getpass.getuser(), Path("/media"), Path("/mnt")]
        cands = [p for r in roots if r.exists() for p in r.iterdir()]
    elif system == "Windows":
        import string
        cands = [Path(f"{d}:\\") for d in string.ascii_uppercase if Path(f"{d}:\\").exists()]
    else:
        return None
    for v in cands:
        try:
            if (v / ".md").exists():
                return str(v)
        except (PermissionError, OSError):
            continue
    return None


def _manifest_path(snu: str) -> Path:
    _DATA_DIR.mkdir(parents=True, exist_ok=True)
    return _DATA_DIR / "manifests" / f"{snu}.json"


def _load_manifest(snu: str) -> dict:
    p = _manifest_path(snu)
    if p.exists():
        return json.loads(p.read_text())
    # Migration depuis l'ancien emplacement (mode dev)
    legacy = SCRIPT_DIR / "manifests" / f"{snu}.json"
    if legacy.exists():
        data = json.loads(legacy.read_text())
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(data, indent=2))
        return data
    return {}


def _save_manifest(snu: str, manifest: dict):
    p = _manifest_path(snu)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(manifest, indent=2))


def _patch_direct_play(zip_path):
    with zipfile.ZipFile(zip_path) as z:
        story_json = json.loads(z.read("story.json"))
        files = {n: z.read(n) for n in z.namelist()}
    nodes = story_json.get("stageNodes", [])
    sq = next((n for n in nodes if n.get("squareOne")), None)
    pod = next((n for n in nodes if not n.get("squareOne")), None)
    if not sq or not pod:
        return
    sq["audio"] = pod.get("audio", "")
    sq["controlSettings"] = {"autoplay": False, "home": True, "ok": False, "pause": True, "wheel": False}
    sq["okTransition"] = None
    sq["homeTransition"] = None
    story_json["stageNodes"] = [sq]
    story_json["actionNodes"] = []
    story_json["listNodes"] = []
    files["story.json"] = json.dumps(story_json).encode()
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as z:
        for n, d in files.items():
            z.writestr(n, d)


def _inject_cover(zip_path, title):
    from PIL import Image, ImageDraw
    with zipfile.ZipFile(zip_path) as z:
        sj = json.loads(z.read("story.json"))
        if any(sn.get("image") for sn in sj.get("stageNodes", [])):
            return
        files = {n: z.read(n) for n in z.namelist()}
    img = Image.new("RGB", (320, 240), (20, 50, 120))
    ImageDraw.Draw(img).text((160, 120), title[:28], fill=(255, 255, 255), anchor="mm")
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    data = buf.getvalue()
    cn = hashlib.sha1(data).hexdigest() + ".png"
    for sn in sj.get("stageNodes", []):
        sn["image"] = cn
    files["story.json"] = json.dumps(sj).encode()
    files[f"assets/{cn}"] = data
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as z:
        for n, d in files.items():
            z.writestr(n, d)


def _generate_zip(audio_file: Path, out_dir: Path) -> str:
    sd = out_dir / audio_file.stem[:50]
    sd.mkdir()
    shutil.copy2(audio_file, sd / audio_file.name)
    env = os.environ.copy()
    env["PATH"] = "/opt/homebrew/bin:" + env.get("PATH", "")
    r = subprocess.run(
        [str(SPG_BINARY), "--skip-extract-image-from-mp-3",
         "--output-folder", str(out_dir), str(sd)],
        env=env, capture_output=True,
    )
    if r.returncode != 0:
        raise RuntimeError(f"studio-pack-generator a échoué (code {r.returncode})")
    zips = sorted(out_dir.glob("*.zip"))
    if not zips:
        raise RuntimeError(f"Aucun ZIP généré pour {audio_file.name}")
    zp = str(zips[-1])
    _inject_cover(zp, audio_file.stem)
    _patch_direct_play(zp)
    return zp


# ── 5. Worker de synchronisation ──────────────────────────────────────────────

class SyncWorker(QObject):
    log = Signal(str)
    progress = Signal(int, int)
    finished = Signal(bool, str)

    def __init__(self, lunii_path: str, audio_dir: Path):
        super().__init__()
        self.lunii_path = lunii_path
        self.audio_dir = audio_dir

    def run(self):
        try:
            from pkg.api.device_lunii import LuniiDevice
        except ImportError as e:
            self.finished.emit(False, f"Lunii.QT inaccessible : {e}")
            return

        self.log.emit("Connexion au device…")
        device = LuniiDevice(self.lunii_path)
        if device.device_version == 0:
            self.finished.emit(False, "Impossible de lire le device. Vérifiez la connexion USB.")
            return

        snu = device.snu_str
        self.log.emit(f"Firmware {device.fw_vers_major}.{device.fw_vers_minor} — SNU {snu}")

        audio_files = sorted(
            f for f in self.audio_dir.iterdir()
            if f.suffix.lower() in (".m4a", ".mp3", ".wav", ".ogg", ".flac")
        )
        if not audio_files:
            self.finished.emit(False, "Aucun fichier audio dans le dossier.")
            return

        manifest = _load_manifest(snu)
        current = {f.name for f in audio_files}

        removed = 0
        for fname, suuid in list(manifest.items()):
            if fname not in current:
                story = next((s for s in device.stories if s.short_uuid == suuid), None)
                if story:
                    p = Path(self.lunii_path) / ".content" / suuid
                    if p.exists():
                        shutil.rmtree(p)
                    device.stories.remove(story)
                    self.log.emit(f"Retiré : {fname}")
                del manifest[fname]
                removed += 1

        to_import = [f for f in audio_files if f.name not in manifest]
        already = len(audio_files) - len(to_import)
        if already:
            self.log.emit(f"{already} fichier(s) déjà présent(s).")

        errors = []
        for i, af in enumerate(to_import):
            self.log.emit(f"[{i+1}/{len(to_import)}] {af.name}")
            with tempfile.TemporaryDirectory() as tmp:
                try:
                    self.log.emit("  → Génération du pack…")
                    zp = _generate_zip(af, Path(tmp))
                    self.log.emit("  → Import sur la Lunii…")
                    if device.import_story(zp) is False:
                        raise RuntimeError("Espace insuffisant ?")
                    manifest[af.name] = device.stories[-1].short_uuid
                    self.log.emit(f"  ✓ {manifest[af.name]}")
                except Exception as e:
                    errors.append(af.name)
                    self.log.emit(f"  ✗ {e}")
            self.progress.emit(i + 1, len(to_import))

        self.progress.emit(len(to_import), len(to_import))
        device.update_pack_index()
        _save_manifest(snu, manifest)

        ok_n = len(audio_files) - len(errors)
        parts = [f"{ok_n} histoire(s) active(s)"]
        if removed:    parts.append(f"{removed} retirée(s)")
        if to_import:  parts.append(f"{len(to_import)-len(errors)} ajoutée(s)")
        if errors:     parts.append(f"{len(errors)} erreur(s)")
        self.finished.emit(not errors, " — ".join(parts))


# ── 6. Fenêtre principale ─────────────────────────────────────────────────────

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Lunii Sync")
        self.setMinimumWidth(520)
        self._lunii_path: str | None = None
        self._audio_dir: Path | None = None
        self._thread: QThread | None = None
        self._build_ui()
        self._detect()
        t = QTimer(self)
        t.timeout.connect(self._detect)
        t.start(3000)

    def _build_ui(self):
        root = QWidget()
        self.setCentralWidget(root)
        lay = QVBoxLayout(root)
        lay.setSpacing(14)
        lay.setContentsMargins(18, 18, 18, 18)

        # Lunii
        g1 = QGroupBox("Boîte à histoires Lunii")
        r1 = QHBoxLayout(g1)
        self._lunii_lbl = QLabel("Non détectée — branchez la Lunii en USB")
        self._lunii_lbl.setWordWrap(True)
        btn_r = QPushButton("↻")
        btn_r.setFixedWidth(36)
        btn_r.setToolTip("Détecter à nouveau")
        btn_r.clicked.connect(self._detect)
        r1.addWidget(self._lunii_lbl, 1)
        r1.addWidget(btn_r)
        lay.addWidget(g1)

        # Dossier audio
        g2 = QGroupBox("Dossier de vos histoires (MP3 / M4A…)")
        r2 = QHBoxLayout(g2)
        self._folder_lbl = QLabel("Aucun dossier sélectionné")
        self._folder_lbl.setWordWrap(True)
        self._folder_btn = QPushButton("Choisir…")
        self._folder_btn.setFixedWidth(90)
        self._folder_btn.clicked.connect(self._pick_folder)
        r2.addWidget(self._folder_lbl, 1)
        r2.addWidget(self._folder_btn)
        lay.addWidget(g2)

        sep = QFrame()
        sep.setFrameShape(QFrame.Shape.HLine)
        sep.setFrameShadow(QFrame.Shadow.Sunken)
        lay.addWidget(sep)

        self._sync_btn = QPushButton("⟳  Synchroniser")
        self._sync_btn.setEnabled(False)
        self._sync_btn.setFixedHeight(52)
        f = QFont()
        f.setPointSize(15)
        f.setBold(True)
        self._sync_btn.setFont(f)
        self._sync_btn.clicked.connect(self._start_sync)
        lay.addWidget(self._sync_btn)

        self._bar = QProgressBar()
        self._bar.setVisible(False)
        lay.addWidget(self._bar)

        self._log = QTextEdit()
        self._log.setReadOnly(True)
        self._log.setMinimumHeight(150)
        self._log.setFont(QFont("Menlo", 11))
        lay.addWidget(self._log)

        note = QLabel("Après la synchro : éjectez la Lunii depuis le Finder, puis redémarrez-la.")
        note.setWordWrap(True)
        note.setStyleSheet("color: gray; font-size: 11px;")
        lay.addWidget(note)

    def _detect(self):
        path = find_lunii()
        if path:
            self._lunii_path = path
            self._lunii_lbl.setText(f"✅  {path}")
        else:
            self._lunii_path = None
            self._lunii_lbl.setText("⏳  Non détectée — branchez la Lunii en USB")
        self._sync_btn.setEnabled(bool(self._lunii_path and self._audio_dir))

    def _pick_folder(self):
        d = QFileDialog.getExistingDirectory(self, "Choisir le dossier audio")
        if d:
            self._audio_dir = Path(d)
            self._folder_lbl.setText(str(self._audio_dir))
        self._sync_btn.setEnabled(bool(self._lunii_path and self._audio_dir))

    def _append(self, msg: str):
        self._log.append(msg)

    def _start_sync(self):
        if not self._lunii_path or not self._audio_dir:
            return
        self._sync_btn.setEnabled(False)
        self._folder_btn.setEnabled(False)
        self._log.clear()
        self._bar.setValue(0)
        self._bar.setVisible(True)
        self._append(f"Lunii : {self._lunii_path}\nDossier : {self._audio_dir}\n")

        self._worker = SyncWorker(self._lunii_path, self._audio_dir)
        self._thread = QThread(self)
        self._worker.moveToThread(self._thread)
        Q = Qt.ConnectionType.QueuedConnection
        self._thread.started.connect(self._worker.run)
        self._worker.log.connect(self._append, Q)
        self._worker.progress.connect(lambda d, t: (
            self._bar.__setattr__("maximum", max(t, 1)) or self._bar.setValue(d)), Q)
        self._worker.finished.connect(self._on_done, Q)
        self._worker.finished.connect(self._thread.quit, Q)
        self._thread.start()

    def _on_done(self, ok: bool, summary: str):
        self._bar.setVisible(False)
        self._sync_btn.setEnabled(True)
        self._folder_btn.setEnabled(True)
        self._append(f"\n{'✅' if ok else '⚠️'}  {summary}")
        def _show_result():
            box = QMessageBox(self)
            box.setWindowTitle("Synchronisation terminée")
            icon = QMessageBox.Icon.Information if ok else QMessageBox.Icon.Warning
            box.setIcon(icon)
            label = "✅  Synchronisation réussie !" if ok else "⚠️  Synchronisation avec erreurs"
            box.setText(f"{label}\n\n{summary}")
            box.open()
        QTimer.singleShot(0, _show_result)


# ── 7. Point d'entrée ─────────────────────────────────────────────────────────

def main():
    app = QApplication.instance() or QApplication(sys.argv)
    app.setApplicationName("Lunii Sync")

    if not _FROZEN and (not LUNII_QT_PATH.exists() or not SPG_BINARY.exists()):
        dlg = SetupDialog()
        if dlg.exec() != QDialog.DialogCode.Accepted:
            sys.exit(1)

    win = MainWindow()
    win.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
