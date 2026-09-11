set -eu
. "$VP_HOME/env"

export VP_PACKAGE_MANAGER=pnpm@10.19.0
vp env use pnpm@10.20.0 --no-install
vp env use yarn@1.22.22 --no-install
test "$VP_PNPM_VERSION" = 10.20.0
test "$VP_YARN_VERSION" = 1.22.22
test "$VP_PACKAGE_MANAGER" = pnpm@10.19.0

vp env use pnpm --unset
test "${VP_PNPM_VERSION-unset}" = unset
test "$VP_YARN_VERSION" = 1.22.22
test "$VP_PACKAGE_MANAGER" = pnpm@10.19.0

vp env use pm --unset
test "${VP_YARN_VERSION-unset}" = unset
test "$VP_PACKAGE_MANAGER" = pnpm@10.19.0
echo 'env use keeps shim versions independent of the selected manager'
