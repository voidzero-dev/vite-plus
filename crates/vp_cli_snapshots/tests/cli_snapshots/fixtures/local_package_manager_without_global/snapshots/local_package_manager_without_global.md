# local_package_manager_without_global

Without a global install's config, the local CLI must use the package manager on PATH and never download one into Vite+ directories.

## `vpt rm ../home/.vite-plus/config.json`

the global install's config is the local CLI's only sign of a global install


## `vpt chmod +x system-bin/pnpm`


## `vp install`

no pnpm on PATH: fail instead of downloading

**Exit code:** 1

```
error: No usable pnpm was found on PATH. Install it, or install the global `vp` CLI so Vite+ can manage it.
```

## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp install`

the system pnpm runs the install

```
system pnpm install
```

## `vpt stat-file ../home/.vite-plus/package_manager`

no package manager was downloaded

```
../home/.vite-plus/package_manager: missing
```
