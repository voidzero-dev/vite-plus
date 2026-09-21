set -eu
. "$VP_HOME/env"
set -h

if [ "${1-}" = all ]; then
  hash -p "$VP_HOME/bin/pnpx" pnpx
  hash -p "$VP_HOME/bin/yarn" yarn
fi

# Keep both direct calls in this shell. A subshell, PATH assignment, or vp
# shell function between them can hide removal of the cached shim.
pnpm --version
hash -t pnpm
pnpm --version
echo 'Both pnpm calls succeeded with the same Bash command cache'

if [ "${1-}" = all ]; then
  pnpx --version
  yarn --version
  echo 'Cached aliases and other families still work after choosing all system managers'
fi
