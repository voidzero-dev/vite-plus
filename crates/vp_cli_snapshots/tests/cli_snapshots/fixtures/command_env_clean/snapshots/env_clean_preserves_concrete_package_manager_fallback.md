# env_clean_preserves_concrete_package_manager_fallback

System-first changes dispatch preference, not cache ownership; clean must retain the managed fallback used when no system manager is available.

## `node prepare-pnpm-versions.cjs`


## `vp env off pnpm`


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
