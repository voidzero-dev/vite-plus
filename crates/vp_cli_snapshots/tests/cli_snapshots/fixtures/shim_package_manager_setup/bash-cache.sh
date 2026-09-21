set -eu
set -h

# Populate and inspect Bash's real command cache before the upgrade handoff.
pnpm --version
pnpx --version
hash -t pnpm
hash -t pnpx

# The old binary runs the new binary's setup with piped output. Do not source
# env, assign PATH, run the vp shell wrapper, or clear the cache between calls.
"$SETUP_NODE" "$SETUP_HELPER" refresh

hash -t pnpm
hash -t pnpx
pnpm --version
pnpm --version
pnpx --version
