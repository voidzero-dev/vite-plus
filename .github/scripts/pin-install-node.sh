#!/bin/sh
set -eu

: "${VP_HOME:?Set VP_HOME to the isolated test installation}"
: "${VP_NODE_VERSION:?Set VP_NODE_VERSION to an exact Node.js version}"

# The published installer resolves the latest LTS independently of VP_NODE_VERSION.
# Pin its release index as well as the runtime used by subsequent commands.
# Archives and checksums still come from the real Node.js distribution server.
mkdir -p "$VP_HOME/js_runtime/node"
printf '{"expires_at":%s,"versions":[{"version":"v%s","lts":"Krypton"}]}\n' \
  "$(( $(date +%s) + 86400 ))" "$VP_NODE_VERSION" \
  > "$VP_HOME/js_runtime/node/index_cache.json"
