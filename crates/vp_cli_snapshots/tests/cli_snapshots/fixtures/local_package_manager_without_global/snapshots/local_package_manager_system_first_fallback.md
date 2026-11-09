# local_package_manager_system_first_fallback

With a global install in system-first mode, the local CLI downloads the package manager when the one on PATH fails to run.

## `vpt write-file ../home/.vite-plus/config.json '{"packageManagerShimModes":{"pnpm":"system_first"}}
'`


## `vpt chmod +x broken-bin/pnpm`


## `PATH=${workspace}/broken-bin${PATH_SEPARATOR}${PATH} npm_config_update_notifier=false vp install`

the broken system pnpm falls back to a download; the real pnpm's update banner would race its output

```
Already up to date

Done in <duration> using pnpm <version>
```

## `vpt stat-file ../home/.vite-plus/package_manager/pnpm/10.18.0`

the pinned pnpm was downloaded

```
../home/.vite-plus/package_manager/pnpm/<version>: dir
```
