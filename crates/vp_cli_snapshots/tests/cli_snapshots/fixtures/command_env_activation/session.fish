command -v node
node
$ACTIVATION_VP env list node; or exit 1
eval $ACTIVATION_COMMAND; or exit 1
test (command -v node) = "$ACTIVATION_BIN/node"; or exit 1
node --version; or exit 1
vp env list node; or exit 1
set -gx PATH $ACTIVATION_SYSTEM $ACTIVATION_BIN
node
vp env list node; or exit 1
eval $ACTIVATION_COMMAND; or exit 1
test (command -v node) = "$ACTIVATION_BIN/node"; or exit 1
node --version; or exit 1
vp env list node; or exit 1
