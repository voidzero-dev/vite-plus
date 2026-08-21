# command_env_package_manager_session_provenance

## `vp env use npm@10.9.4 --no-install`


## `vpt write-file $VP_HOME/package_manager/npm/10.9.4/npm/bin/npm '#'\!'/bin/sh
'`


## `vpt chmod +x $VP_HOME/package_manager/npm/10.9.4/npm/bin/npm`


## `vp env current pm --json`

current reports the package-manager session file path

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": ".session-package-manager",
    "source_path": "<home>/.vite-plus/.session-package-manager",
    "bin_paths": {
      "npm": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm",
      "npx": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```

## `vp env which npm`

which reports the package-manager session file as its source

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm
  Package:    npm@10.9.4
  Source:     <home>/.vite-plus/.session-package-manager
```
