"""R002 — vecteurs de référence INDÉPENDANTS (doubleur), via le code StoryBox.QT au tip.

Autres SNU, autres données, autres tailles que le script d'auteur M0005.
Sorties : <OUT>/in (entrées partagées avec la sonde Rust), <OUT>/py (sorties de la référence).
"""
import hashlib
import io
import json
import os
import shutil
import sys
import zipfile

REPO_QT = sys.argv[1]           # <worktree>/StoryBox.QT
OUT = sys.argv[2]
sys.path.insert(0, REPO_QT)

from pkg.api import device_storybox as ds  # noqa: E402

# Neutralisations hors crypto (l'app Rust ne transcode pas, ne convertit pas les images)
ds.transcoding_required = lambda f, d: False
ds.tags_removal_required = lambda d: None
ds.image_to_bitmap_rle4 = lambda d: d


def prng(tag: str, n: int) -> bytes:
    out = b""
    i = 0
    while len(out) < n:
        out += hashlib.sha256(f"R002|{tag}|{i}".encode()).digest()
        i += 1
    return out[:n]


def make_md(version: int, size: int, snu_ascii: bytes, tag: str) -> bytes:
    md = bytearray(prng("md-" + tag, size))
    md[0] = version
    md[1] = 0
    md[2:7] = b"3.2.9"
    md[0x1A:0x1A + 14] = snu_ascii
    return bytes(md)


CASES = {
    "v6a": (6, 112, b"7F00A1B2C3D4E5"),
    "v6b": (6, 128, b"fedcba98765432"),
    "v7a": (7, 128, b"DEADBEEF123456"),
    "v7b": (7, 112, b"00000000000001"),
}
SIZES = [5, 15, 16, 511, 512, 513, 4096]


def sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def fresh_mount(path: str, md: bytes):
    if os.path.isdir(path):
        shutil.rmtree(path)
    os.makedirs(os.path.join(path, ".content"))
    with open(os.path.join(path, ".md"), "wb") as f:
        f.write(md)


STORY_UUID = "0f1e2d3c-4b5a-4968-8776-a5b4c3d2e1f0"
STORY2_UUID = "aaaabbbb-cccc-4ddd-8eee-ffff00001111"
STORY_JSON = {
    "format": "v1", "version": 3, "title": "R002 doublage", "description": "d",
    "nightModeAvailable": False, "factoryPack": False,
    "stageNodes": [
        {"uuid": STORY_UUID, "squareOne": True, "audio": "0123456789abcdef.mp3",
         "image": "cafebabe00112233.bmp",
         "controlSettings": {"wheel": True, "ok": True, "home": False, "pause": False, "autoplay": False},
         "okTransition": {"actionNode": "act-1", "optionIndex": 0}, "homeTransition": None},
        {"uuid": STORY2_UUID, "audio": "fin00042.mp3", "image": None,
         "controlSettings": {"wheel": False, "ok": False, "home": True, "pause": True, "autoplay": True},
         "okTransition": None, "homeTransition": None},
    ],
    "actionNodes": [{"id": "act-1", "options": [STORY2_UUID]}],
    "listNodes": [],
}
ASSETS = {
    "assets/0123456789abcdef.mp3": prng("audio1", 4096),
    "assets/fin00042.mp3": prng("audio2", 511),
    "assets/cafebabe00112233.bmp": prng("image1", 513),
}


def main():
    inp = os.path.join(OUT, "in")
    pyo = os.path.join(OUT, "py")
    os.makedirs(inp, exist_ok=True)
    os.makedirs(pyo, exist_ok=True)
    report = {"keys": {}, "cipher": {}, "e2e": {}}

    for n in SIZES:
        with open(os.path.join(inp, f"data_{n}.bin"), "wb") as f:
            f.write(prng(f"data-{n}", n))

    zbuf = io.BytesIO()
    with zipfile.ZipFile(zbuf, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("story.json", json.dumps(STORY_JSON))
        for name, data in ASSETS.items():
            z.writestr(name, data)
    zip_path = os.path.join(inp, "story.zip")
    with open(zip_path, "wb") as f:
        f.write(zbuf.getvalue())

    for case, (ver, size, snu) in CASES.items():
        md = make_md(ver, size, snu, case)
        with open(os.path.join(inp, f"{case}.md"), "wb") as f:
            f.write(md)
        mount = os.path.join(OUT, "mnt_py", case)
        fresh_mount(mount, md)
        dev = ds.StoryBoxDevice(mount)
        assert dev.device_version == 3, dev.device_version
        report["keys"][case] = {
            "md_sha256": sha(md),
            "story_key": dev.story_key.hex(), "story_iv": dev.story_iv.hex(), "bt": dev.bt.hex(),
        }
        os.makedirs(os.path.join(pyo, case), exist_ok=True)
        for n in SIZES:
            data = prng(f"data-{n}", n)
            enc = dev.cipher(data, dev.story_key, dev.story_iv)
            with open(os.path.join(pyo, case, f"enc_{n}.bin"), "wb") as f:
                f.write(enc)
            report["cipher"][f"{case}/{n}"] = {"len": len(enc), "sha256": sha(enc)}
        ok = dev.import_story(zip_path)
        assert ok, f"import référence {case} a échoué"
        tree = {}
        croot = os.path.join(mount, ".content")
        for root, _, files in os.walk(croot):
            for fn in files:
                p = os.path.join(root, fn)
                with open(p, "rb") as f:
                    b = f.read()
                tree[os.path.relpath(p, croot)] = [len(b), sha(b)]
        with open(os.path.join(mount, ".pi"), "rb") as f:
            pi = f.read()
        report["e2e"][case] = {"tree": dict(sorted(tree.items())), "pi": pi.hex()}

    with open(os.path.join(OUT, "py_report.json"), "w") as f:
        json.dump(report, f, indent=1, sort_keys=True)
    print(json.dumps(report["keys"], indent=1))


if __name__ == "__main__":
    main()
