#!/usr/bin/env bash
set -euo pipefail

vp_bin="$1"

# Read the same pin that the built CLI uses, including after dependency upgrades.
default_node_version=$(sed -nE 's/^pub const DEFAULT_NODE_VERSION: &str = "([0-9]+\.[0-9]+\.[0-9]+)";$/\1/p' crates/vp_js_runtime/src/lib.rs)
if [[ ! "$default_node_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'Could not read the pinned default Node.js version\n' >&2
  exit 1
fi

prewarm_node() {
  local version="$1"
  local TIMEFORMAT="Node.js ${version}: %3R seconds elapsed"
  printf '\nPreparing Node.js %s\n' "$version"
  time VP_NODE_VERSION="$version" "$vp_bin" node --version || return "$?"
}

prewarm_node "$(< .node-version)"
prewarm_node "$default_node_version"
prewarm_node lts
while IFS= read -r node_version; do
  prewarm_node "$node_version"
done < crates/vp_cli_snapshots/tests/cli_snapshots/node-versions.txt
