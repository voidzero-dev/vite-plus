# env_clean_preserves_concrete_package_manager_fallback

## `vp env print pnpm`


## `vp env off pnpm`


## `vpt write-file $VP_HOME/package_manager/pnpm/0.0.1/pnpm/bin/pnpm '#'\!'/bin/sh
'`


## `vp env clean pnpm`

cleanup removes stale installs but preserves the concrete family's managed fallback even in system-first mode

```
VITE+ - The Unified Toolchain for the Web

✓ Removed 1 package manager install
```

## `node assert-one-pnpm-version.cjs`

the cached registry fallback remains available

```
kept one concrete pnpm fallback
```
