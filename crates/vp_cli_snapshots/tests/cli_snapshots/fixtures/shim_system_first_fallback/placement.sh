set -eu
. "$VP_HOME/env"
original_path=$PATH
mkdir -p system-bin
printf '#!/bin/sh\necho v99.0.0\n' > system-bin/node
chmod +x system-bin/node
system_bin="$PWD/system-bin"

vp env on >/dev/null
node --version >/dev/null # Populate Bash's command cache before moving the shim.
vp env off node >/dev/null
test ! -e "$VP_HOME/bin/node"
test -L "$VP_HOME/fallback-bin/node"
test -L "$VP_HOME/bin/pnpm"
PATH="$system_bin:$PATH"
. "$VP_HOME/env"
test "$(command -v node)" = "$system_bin/node"
test "$(node --version)" = v99.0.0
test "$(vp env which node)" = "$system_bin/node"

vp env off pnpm >/dev/null
for tool in pnpm pnpx; do
    test ! -e "$VP_HOME/bin/$tool"
    test -L "$VP_HOME/fallback-bin/$tool"
done
for tool in npm npx yarn yarnpkg bun bunx vpr vpx; do
    test -L "$VP_HOME/bin/$tool"
done
vp env setup --refresh >/dev/null
test -L "$VP_HOME/fallback-bin/node"
test -L "$VP_HOME/fallback-bin/pnpm"

# A Vite+ shim before another executable ends internal lookup at managed resolution.
PATH="$VP_HOME/fallback-bin:$system_bin:$original_path"
resolved=$(vp env which node)
test "${resolved%%$'\n'*}" = "$VP_HOME/js_runtime/node/20.18.0/bin/node"
test "$(node --version)" = v20.18.0
PATH="$system_bin:$original_path"
. "$VP_HOME/env"
. "$VP_HOME/env"
test "${PATH%%:*}" = "$VP_HOME/bin"
test "${PATH##*:}" = "$VP_HOME/fallback-bin"
test "$(node --version)" = v99.0.0

vp env on node >/dev/null
test "$(command -v node)" = "$VP_HOME/bin/node"
test "$(node --version)" = v20.18.0
test ! -e "$VP_HOME/fallback-bin/node"
test -L "$VP_HOME/fallback-bin/pnpm"
vp env on pnpm >/dev/null
test -L "$VP_HOME/bin/pnpm"
test ! -e "$VP_HOME/fallback-bin/pnpm"
echo 'Scoped placement, refresh, shell cache, and PATH precedence passed'
