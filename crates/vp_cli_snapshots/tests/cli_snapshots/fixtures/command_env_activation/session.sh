set -eu
show() {
    if [ -n "${ACTIVATION_SHOW_COMMANDS-}" ]; then
        printf '$ %s\n' "$1"
    fi
}
show 'command -v node'
command -v node
show node
node
show 'vp env list node'
"$ACTIVATION_VP" env list node
show "$ACTIVATION_COMMAND"
eval "$ACTIVATION_COMMAND"
test "$(command -v node)" = "$ACTIVATION_BIN/node"
show 'node --version'
node --version
show 'vp env list node'
vp env list node
# Existing command lookup caches must be refreshed by sourcing the env file.
show 'export PATH="$ACTIVATION_SYSTEM:$ACTIVATION_BIN"'
PATH="$ACTIVATION_SYSTEM:$ACTIVATION_BIN"
export PATH
show node
node
show 'vp env list node'
vp env list node
show "$ACTIVATION_COMMAND"
eval "$ACTIVATION_COMMAND"
test "$(command -v node)" = "$ACTIVATION_BIN/node"
show 'node --version'
node --version
show 'vp env list node'
vp env list node
