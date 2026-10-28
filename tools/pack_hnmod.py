"""Pack Biome Compass's registry file (.hnmod): horizon-native.json + mixins/biomecompass/*.{rs,toml}.

Deterministic: sorted names, no directory entries, deflate, fixed 2026-01-01
timestamps, Unix 0644. It checks what the registry and the launcher check
(`horizon_mods::index::native::Bundle::read`): exact 40-hex revisions, the
same id, version and Horizon revision in both manifests, only .rs and .toml
under mixins/<id>/.

Usage: python tools/pack_hnmod.py [OUT]   (default: dist/<id>-<version>.hnmod)
"""

import json
import pathlib
import re
import sys
import tomllib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
REVISION = re.compile(r"^[0-9a-f]{40}$")


def fail(message: str) -> None:
    sys.exit(f"pack_hnmod: {message}")


def main() -> None:
    native = json.loads((ROOT / "horizon-native.json").read_text(encoding="utf-8"))
    if set(native) != {"schema", "id", "version", "game", "horizon", "mixins"}:
        fail("horizon-native.json has other keys than the registry's schema 1")
    if native["schema"] != 1:
        fail("schema must be 1")
    for key in ("horizon", "mixins"):
        if not REVISION.match(native[key]):
            fail(f"{key} must be a 40-hex commit")
    bundle = ROOT / "mixins" / native["id"]
    mixin = tomllib.loads((bundle / "horizon-mixin.toml").read_text(encoding="utf-8"))
    pairs = [
        (mixin["mod"]["id"], native["id"], "id"),
        (mixin["mod"]["version"], native["version"], "version"),
        (mixin["compatibility"]["horizon_revision"], native["horizon"], "Horizon revision"),
    ]
    for theirs, ours, what in pairs:
        if theirs != ours:
            fail(f"the {what} differs: horizon-mixin.toml {theirs}, horizon-native.json {ours}")
    names = ["horizon-native.json"] + [
        f"mixins/{native['id']}/{p.name}"
        for p in bundle.iterdir()
        if p.is_file() and p.suffix in (".rs", ".toml")
    ]
    out = (
        pathlib.Path(sys.argv[1])
        if len(sys.argv) > 1
        else ROOT / "dist" / f"{native['id']}-{native['version']}.hnmod"
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w") as z:
        for name in sorted(names):
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            z.writestr(info, (ROOT / name).read_bytes())
    print(f"{out} ({len(names)} files)")


if __name__ == "__main__":
    main()
