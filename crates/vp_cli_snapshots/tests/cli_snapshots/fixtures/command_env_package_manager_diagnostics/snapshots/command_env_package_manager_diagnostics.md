# command_env_package_manager_diagnostics

## `node prepare-npm.cjs`


## `vp env current pm --json`

current reports the npm packageManager pin

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": "packageManager",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_paths": {
      "npm": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm",
      "npx": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```

## `vp env current pm`

current lists every binary exposed by the selected package-manager family

```
VITE+ - The Unified Toolchain for the Web

Package Manager:
  Name       npm
  Version    10.9.4
  Source     packageManager
  Bin Paths
    npm      <home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm
    npx      <home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx
  Installed  true
  Mode       managed
```

## `vp env which npm`

which reports the npm packageManager pin

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm
  Package:    npm@10.9.4
  Source:     <workspace>/package.json
```

## `vp env which npx`

the npx alias reports the same npm packageManager pin

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx
  Package:    npm@10.9.4
  Source:     <workspace>/package.json
```
