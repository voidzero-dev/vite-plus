set -eu
vp_binary="$VP_HOME/bin/vp"
unset VP_HOME
HOME="$PWD/user"
XDG_CONFIG_HOME="$HOME/config"
VP_BIN_DIR="$PWD/shared-bin"
VP_DATA_DIR="$PWD/data"
VP_CACHE_DIR="$PWD/cache"
export HOME XDG_CONFIG_HOME VP_BIN_DIR VP_DATA_DIR VP_CACHE_DIR
mkdir -p "$VP_BIN_DIR"
printf '#!/bin/sh\necho foreign-node\n' > "$VP_BIN_DIR/node"
chmod +x "$VP_BIN_DIR/node"
"$vp_binary" env setup --refresh >/dev/null
"$vp_binary" env off node >/dev/null
test -L "$VP_DATA_DIR/fallback-bin/node"
test "$("$VP_BIN_DIR/node")" = foreign-node
"$vp_binary" env setup --refresh >/dev/null
test -L "$VP_DATA_DIR/fallback-bin/node"
test "$("$VP_BIN_DIR/node")" = foreign-node
. "$XDG_CONFIG_HOME/vite-plus/env"
test "${PATH%%:*}" = "$VP_BIN_DIR"
test "${PATH##*:}" = "$VP_DATA_DIR/fallback-bin"
"$vp_binary" env on node >/dev/null
test "$("$VP_BIN_DIR/node")" = foreign-node
echo 'Split layout and foreign tool preservation passed'
