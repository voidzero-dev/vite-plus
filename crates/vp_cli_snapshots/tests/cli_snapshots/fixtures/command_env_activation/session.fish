printf '$ command -v node\n'
command -v node
printf '$ node --version\n'
node --version
printf '$ %s\n' "$ACTIVATION_COMMAND"
eval $ACTIVATION_COMMAND; or exit 1
printf '$ command -v node\n'
command -v node
test (command -v node) = "$ACTIVATION_BIN/node"; or exit 1
printf '$ node --version\n'
node --version; or exit 1
printf '$ vp env list node\n'
vp env list node; or exit 1
