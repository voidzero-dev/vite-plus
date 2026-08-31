# env_pin_package_manager_from_nested_workspace

## `vp env pin yarn@4.12.0 --no-install --force`

a nested workspace pin updates the resolver-owned root manifest

```
VITE+ - The Unified Toolchain for the Web

warn: Current environment resolves to npm from devEngines.packageManager, but yarn was requested.
✓ Pinned package manager to yarn@4.12.0
note: Package manager will be downloaded on first use.
```

## `vp env current pm --json`

the nested project resolves the new root pin

```
{
  "package_manager": {
    "name": "yarn",
    "version": "<version>",
    "source": "devEngines.packageManager",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_paths": {
      "yarn": "<home>/.vite-plus/package_manager/yarn/<version>/yarn/bin/yarn",
      "yarnpkg": "<home>/.vite-plus/package_manager/yarn/<version>/yarn/bin/yarnpkg"
    },
    "installed": false,
    "mode": "managed"
  }
}
```

## `vpt print-file ../../package.json package.json`

only the workspace manifest owns the package-manager pin

```
{
  "name": "command-env-pin-package-manager",
  "private": true,
  "workspaces": [
    "packages/*"
  ],
  "devEngines": {
    "packageManager": [
      {
        "name": "yarn",
        "version": "<version>",
        "onFail": "download"
      },
      {
        "name": "npm",
        "version": "<version>",
        "onFail": "error"
      },
      {
        "name": "pnpm",
        "version": "<version>",
        "onFail": "download"
      }
    ]
  }
}
{
  "name": "app",
  "private": true
}
```
