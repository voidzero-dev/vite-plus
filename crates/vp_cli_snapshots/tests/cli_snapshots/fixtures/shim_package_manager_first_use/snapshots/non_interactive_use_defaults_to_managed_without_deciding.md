# non_interactive_use_defaults_to_managed_without_deciding

Upgraded installations may have no recorded family mode; non-interactive use must stay deterministic without persisting consent on the user's behalf.

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

an undecided non-interactive shim uses managed pnpm without prompting

```
11.24.0
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env current pnpm --json`

environment inspection uses the same stable managed default

```
{
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": "registry fallback",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```

## `vpt stat-file $VP_HOME/config.json --assert missing`

non-interactive use does not record a choice

```
<home>/.vite-plus/config.json: missing
```
