#!/usr/bin/env bash
# Install every Node.js runtime the CLI snapshot fixtures resolve into the
# managed runtime that seeds each case home (VP_SNAP_JS_RUNTIME_DIR):
#
#   bash .github/scripts/prewarm-snapshot-runtimes.sh "$HOME/.vite-plus/bin/vp"
#
# Seeded cases share that directory, so a version missing from it is
# downloaded from nodejs.org inside a step's 50s budget (60s on Windows).
# When nodejs.org is slow, every case racing the same download times out,
# nothing lands in the seed, and the next case starts over. Downloading here
# keeps those transfers out of the step budget.
#
# Best-effort: the scan only needs to cover the common pin forms, and a
# version that is missed or fails to install here is still downloaded on
# demand by the case that needs it.
set -uo pipefail

vp="$1"
fixtures="$(dirname "$0")/../../crates/vp_cli_snapshots/tests/cli_snapshots/fixtures"

requests() {
  # Unpinned cases resolve the latest LTS; `node`/`stable` pins resolve latest.
  printf '%s\n' lts latest
  # Pin files: the first non-comment token of each `.node-version`/`.nvmrc`.
  find "$fixtures" \( -name .node-version -o -name .nvmrc \) -type f \
    -exec awk '{ sub(/#.*/, ""); if (NF) { print $1; exit } }' {} \;
  # Step requests: `--node <v>`, `vp env install|use <v>`, VP_NODE_VERSION,
  # and pin files written by steps.
  grep -rhoE --include=snapshots.toml \
    -e '"--node", *"[^"]+"' \
    -e '"env", *"(install|use)", *"[^"]+"' \
    -e 'VP_NODE_VERSION"?,? *=? *"[^"]+"' \
    -e '"write-file", *"[^"]*(\.node-version|\.nvmrc)", *"[^"]+"' \
    "$fixtures" | sed -E 's/.*"([^"]+)"$/\1/; s/\\n$//'
}

# Keep only Node.js versions and aliases; package-manager selectors and
# intentionally invalid pins are not runtimes to provision.
requests | tr -d '\r' | sed -E 's/^v([0-9])/\1/' |
  grep -E '^([0-9]+(\.[0-9]+){0,2}|lts(/[a-z*-]+|/-[0-9]+)?|latest)$' |
  sort -u |
  xargs -P 4 -I{} sh -c '"$1" env install "$2" || echo "::warning::prewarm could not install Node.js $2; cases will download it on demand"' sh "$vp" {}
