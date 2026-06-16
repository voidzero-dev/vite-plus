#!/usr/bin/env bash
# Regenerate the vendored Node.js release signing keyring from the canonical
# nodejs/release-keys repository.
#
# The keyring is the trust anchor used by `vite_js_runtime` to verify the PGP
# signature on Node.js `SHASUMS256.txt` (see rfcs/verify-node-shasums-signature.md).
# Run from the repository root:
#
#   .github/scripts/update-node-release-keys.sh
#
# When KEYRING_SUMMARY_FILE is set, a Markdown pull-request body describing the
# before/after key changes (added, removed, modified, by fingerprint) is written
# to that path. The accompanying workflow
# (.github/workflows/update-node-release-keys.yml) runs this weekly and opens a
# PR when the keyring changes. The change must be reviewed by a human before
# merging, since it alters which keys are trusted.
set -euo pipefail

DEST="crates/vite_js_runtime/src/assets/node-release-keys.asc"
SUMMARY_FILE="${KEYRING_SUMMARY_FILE:-}"

if [[ ! -f "$DEST" ]]; then
  echo "error: run from the repository root (missing $DEST)" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Snapshot the current keyring so we can diff it after regenerating.
cp "$DEST" "$TMP/before.asc"

git clone --depth 1 https://github.com/nodejs/release-keys.git "$TMP/release-keys"

# Concatenate every armored key block, sorted by filename and separated by a
# single blank-free newline, so a key whose file lacks a trailing newline does
# not get its END line merged with the next BEGIN line.
python3 - "$TMP/release-keys/keys" "$DEST" <<'PY'
import glob
import os
import sys

keys_dir, dest = sys.argv[1], sys.argv[2]
files = sorted(glob.glob(os.path.join(keys_dir, "*.asc")))
if not files:
    sys.exit("error: no key files found in nodejs/release-keys")

blocks = [open(path, encoding="utf-8").read().strip("\n") for path in files]
with open(dest, "w", encoding="utf-8") as out:
    out.write("\n".join(blocks) + "\n")
print(f"wrote {len(files)} release keys to {dest}")
PY

# Sanity check: balanced, non-trivial keyring.
begin=$(grep -c '^-----BEGIN PGP PUBLIC KEY BLOCK-----$' "$DEST" || true)
end=$(grep -c '^-----END PGP PUBLIC KEY BLOCK-----$' "$DEST" || true)
if [[ "$begin" != "$end" || "$begin" -lt 8 ]]; then
  echo "error: unexpected keyring (begin=$begin end=$end)" >&2
  exit 1
fi
echo "keyring has $begin key blocks"

# Write the PR body with a before/after comparison of the changed keys. Best
# effort: if the diff can't be produced (e.g. gpg missing), fall back to a plain
# body so the PR is still created.
if [[ -n "$SUMMARY_FILE" ]]; then
  if ! python3 - "$TMP/before.asc" "$DEST" > "$SUMMARY_FILE" <<'PY'
import hashlib
import subprocess
import sys
import tempfile

PREAMBLE = (
    "Automated refresh of the vendored Node.js release signing keys from "
    "[nodejs/release-keys](https://github.com/nodejs/release-keys).\n\n"
    "This keyring is the trust anchor used to verify the PGP signature on "
    "Node.js `SHASUMS256.txt` (see `rfcs/verify-node-shasums-signature.md`).\n\n"
    "**Review the key changes below before merging.** PR CI runs the "
    "`vite_js_runtime` tests, which confirm every vendored key still parses; a "
    "new key that fails to parse will surface as a failing test.\n"
)


def load_keys(asc_path):
    """Map primary fingerprint -> {uid, fingerprint, digest of its colon record}."""
    home = tempfile.mkdtemp()
    out = subprocess.run(
        ["gpg", "--homedir", home, "--quiet", "--batch",
         "--show-keys", "--with-colons", asc_path],
        capture_output=True, text=True, check=True,
    ).stdout

    keys, fpr, pending = {}, None, []
    for line in out.splitlines():
        field = line.split(":")
        record = field[0]
        if record == "pub":
            fpr, pending = None, [line]
        elif record == "fpr" and fpr is None:
            fpr = field[9]
            keys[fpr] = {"uid": "", "lines": pending + [line]}
            pending = []
        elif fpr is not None:
            keys[fpr]["lines"].append(line)
            if record == "uid" and not keys[fpr]["uid"]:
                keys[fpr]["uid"] = field[9]
        else:
            pending.append(line)

    for entry in keys.values():
        entry["digest"] = hashlib.sha256("\n".join(entry["lines"]).encode()).hexdigest()
    return keys


before = load_keys(sys.argv[1])
after = load_keys(sys.argv[2])

added = sorted(set(after) - set(before))
removed = sorted(set(before) - set(after))
modified = sorted(f for f in set(after) & set(before)
                  if after[f]["digest"] != before[f]["digest"])


def bullets(fingerprints, table):
    if not fingerprints:
        return ["- _none_"]
    out = []
    for fpr in fingerprints:
        uid = table[fpr]["uid"] or "(no user id)"
        out.append(f"- {uid} `{fpr}`")
    return out


lines = [PREAMBLE, "## Key changes", ""]
lines.append(f"Release keyring: **{len(before)} → {len(after)} keys**.")
lines += ["", f"### Added ({len(added)})", *bullets(added, after)]
lines += ["", f"### Removed ({len(removed)})", *bullets(removed, before)]
lines += ["", f"### Modified ({len(modified)})", *bullets(modified, after)]
print("\n".join(lines))
PY
  then
    echo "warning: could not build key diff; writing a plain PR body" >&2
    cat > "$SUMMARY_FILE" <<'EOF'
Automated refresh of the vendored Node.js release signing keys from
[nodejs/release-keys](https://github.com/nodejs/release-keys).

This keyring is the trust anchor used to verify the PGP signature on Node.js
`SHASUMS256.txt` (see `rfcs/verify-node-shasums-signature.md`). Review which keys
changed (`git diff`) before merging.
EOF
  fi
  echo "wrote PR body to $SUMMARY_FILE"
fi
