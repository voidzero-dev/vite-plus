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
PATH="$system_bin:$PATH"
. "$VP_HOME/env"
. "$VP_HOME/env"
test "${PATH%%:*}" = "$VP_HOME/bin"
test "${PATH##*:}" = "$VP_HOME/fallback-bin"
test "$(command -v node)" = "$system_bin/node"
test "$(node --version)" = v99.0.0
test "$(vp env which node)" = "$system_bin/node"

PATH="$VP_HOME/fallback-bin:$system_bin:$original_path"
resolved=$(vp env which node)
test "${resolved%%$'\n'*}" = "$VP_HOME/js_runtime/node/20.18.0/bin/node"
test "$(node --version)" = v20.18.0
PATH="$system_bin:$original_path"
. "$VP_HOME/env"
node --version >/dev/null
vp env on node >/dev/null
test "$(command -v node)" = "$VP_HOME/bin/node"
test "$(node --version)" = v20.18.0
echo 'Repeated shell setup and mode changes preserve PATH precedence and clear Bash cache'
