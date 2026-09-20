set -eu
. "$XDG_CONFIG_HOME/vite-plus/env"
. "$XDG_CONFIG_HOME/vite-plus/env"
test "${PATH%%:*}" = "$VP_BIN_DIR"
test "${PATH##*:}" = "$VP_DATA_DIR/fallback-bin"
echo 'Split shell PATH starts with shared bin and ends with data-root fallback'
