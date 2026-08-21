# env_pin_warns_for_lockfile_selection_offline

## `vpt write-file package.json '{"name":"command-env-package-manager-mismatch","private":true}
'`


## `vpt touch-file pnpm-lock.yaml`


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env pin yarn@4.12.0 --no-install`

lockfile mismatch warning does not depend on registry resolution

```
VITE+ - The Unified Toolchain for the Web

warn: Current environment resolves to pnpm from lockfile or config, but yarn was requested.
✓ Pinned package manager to yarn@4.12.0
note: Package manager will be downloaded on first use.
```
