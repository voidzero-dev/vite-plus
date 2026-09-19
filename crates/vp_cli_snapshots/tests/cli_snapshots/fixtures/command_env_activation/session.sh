set -eu
command -v node
node
"$ACTIVATION_VP" env list node
eval "$ACTIVATION_COMMAND"
test "$(command -v node)" = "$ACTIVATION_BIN/node"
node --version
vp env list node
# Existing command lookup caches must be refreshed by sourcing the env file.
PATH="$ACTIVATION_SYSTEM:$ACTIVATION_BIN"
export PATH
node
vp env list node
eval "$ACTIVATION_COMMAND"
test "$(command -v node)" = "$ACTIVATION_BIN/node"
node --version
vp env list node
