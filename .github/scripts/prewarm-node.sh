#!/usr/bin/env bash
set -euo pipefail

vp_bin="$1"

prewarm_node() {
  local version="$1"
  local TIMEFORMAT="Node.js ${version}: %3R seconds elapsed"
  printf '\nPreparing Node.js %s\n' "$version"
  time VP_NODE_VERSION="$version" "$vp_bin" node --version || return "$?"
}

# Development, CI, and the built-in runtime all use this pin.
prewarm_node "$(< .node-version)"
prewarm_node lts
prewarm_node latest
while IFS= read -r node_version; do
  prewarm_node "$node_version"
done < crates/vp_cli_snapshots/tests/cli_snapshots/node-versions.txt
