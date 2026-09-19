set -eu
printf '$ command -v node\n'
command -v node
printf '$ node --version\n'
node --version
printf '$ %s\n' "$ACTIVATION_COMMAND"
eval "$ACTIVATION_COMMAND"
printf '$ command -v node\n'
command -v node
test "$(command -v node)" = "$ACTIVATION_BIN/node"
printf '$ node --version\n'
node --version
printf '$ vp env list node\n'
vp env list node
