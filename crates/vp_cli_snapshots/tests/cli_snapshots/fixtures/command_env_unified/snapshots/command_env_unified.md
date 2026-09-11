# command_env_unified

## `vp env pin 22.0.0 --no-install --force`

legacy unqualified versions still pin only Node.js

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.0.0
  Updated devEngines.runtime in <workspace>/package.json
note: Version will be downloaded on first use.
```

## `vpt print-file package.json`

the Node.js pin preserves the package manifest structure

```
{
  "name": "command-env-unified",
  "private": true,
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp env pin pnpm@10.18.0 --no-install`

a qualified spec pins only the package manager

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned package manager to pnpm@10.18.0
note: Package manager will be downloaded on first use.
```

## `vpt print-file package.json`

the PM pin is written beside the runtime declaration

```
{
  "name": "command-env-unified",
  "private": true,
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    },
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp env current --json`

current JSON exposes peer Node.js and package-manager objects

```
{
  "node": {
    "version": "22.0.0",
    "source": "devEngines.runtime",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node",
    "installed": false,
    "mode": "managed"
  },
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": "devEngines.packageManager",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```

## `vp env list --json`

bare list JSON includes Node.js and every PM family

```
{
  "node": [],
  "package_managers": {
    "bun": [],
    "npm": [],
    "pnpm": [],
    "yarn": []
  }
}
```

## `vp env list node --json`

the node selector omits package managers

```
{
  "node": []
}
```

## `vp env list pm --json`

the pm selector omits Node.js

```
{
  "package_managers": {
    "bun": [],
    "npm": [],
    "pnpm": [],
    "yarn": []
  }
}
```

## `vp env unpin`

bare unpin removes both effective project pins

```
VITE+ - The Unified Toolchain for the Web

✓ Removed devEngines.runtime node entry from <workspace>/package.json
✓ Removed package-manager pin
```

## `vpt print-file package.json`

both devEngines declarations were removed

```
{
  "name": "command-env-unified",
  "private": true,
  "devEngines": {}
}
```
