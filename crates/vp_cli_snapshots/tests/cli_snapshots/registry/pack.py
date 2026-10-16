"""Pack the local CLI fixtures without downloading packages or running scripts."""

import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import tarfile

root = Path(__file__).resolve().parent
destination = root / "local"
(destination / "tarballs").mkdir(parents=True, exist_ok=True)
manifest = {}
for source in sorted((root / "packages").iterdir()):
    package = json.loads((source / "package.json").read_text())
    name, version = package["name"], package["version"]
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w") as archive:
        for path in sorted(source.iterdir()):
            data = path.read_bytes()
            info = tarfile.TarInfo("package/" + path.name)
            info.size = len(data)
            info.mode = 0o755 if path.suffix == ".cjs" else 0o644
            archive.addfile(info, io.BytesIO(data))

    tarball = gzip.compress(buffer.getvalue(), mtime=0)
    filename = f"{name}-{version}.tgz"
    (destination / "tarballs" / filename).write_bytes(tarball)
    package["dist"] = {
        "tarball": f"{{REGISTRY}}/{name}/-/{filename}",
        "integrity": "sha512-" + base64.b64encode(hashlib.sha512(tarball).digest()).decode(),
    }
    manifest[name] = {
        "name": name,
        "dist-tags": {"latest": version},
        "versions": {version: package},
    }

(destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
