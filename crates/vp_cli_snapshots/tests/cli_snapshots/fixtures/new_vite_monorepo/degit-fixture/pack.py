"""Pack the checked-in template without fetching external data."""

import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import tarfile

source = Path(__file__).resolve().parent
buffer = io.BytesIO()
with tarfile.open(fileobj=buffer, mode="w") as archive:
    files = [source / name for name in ["package.json", "cli.mjs", "LICENSE"]]
    files += sorted(path for path in (source / "template").rglob("*") if path.is_file())
    for path in files:
        data = path.read_bytes()
        info = tarfile.TarInfo("package/" + path.relative_to(source).as_posix())
        info.size = len(data)
        info.mode = 0o755 if path.name == "cli.mjs" else 0o644
        archive.addfile(info, io.BytesIO(data))

tarball = gzip.compress(buffer.getvalue(), mtime=0)
integrity = "sha512-" + base64.b64encode(hashlib.sha512(tarball).digest()).decode()
package = json.loads((source / "package.json").read_text())
package["dist"] = {
    "tarball": "{REGISTRY}/degit/-/degit-0.0.0.tgz",
    "integrity": integrity,
}
manifest = {
    "degit": {
        "name": "degit",
        "dist-tags": {"latest": "0.0.0"},
        "versions": {"0.0.0": package},
    }
}
for name in ["new_vite_monorepo", "new_vite_monorepo_bun"]:
    case = source.parent.parent / name
    (case / "tarballs").mkdir(exist_ok=True)
    (case / "tarballs" / "degit-0.0.0.tgz").write_bytes(tarball)
    (case / "mock-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
