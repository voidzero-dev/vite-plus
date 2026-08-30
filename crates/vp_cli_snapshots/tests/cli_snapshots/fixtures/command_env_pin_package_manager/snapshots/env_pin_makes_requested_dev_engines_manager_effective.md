# env_pin_makes_requested_dev_engines_manager_effective

## `vp env pin pnpm@10.18.0 --no-install --force`

pinning an existing later option makes it the effective first supported entry

```
VITE+ - The Unified Toolchain for the Web

warn: Current environment resolves to npm from devEngines.packageManager, but pnpm was requested.
✓ Pinned package manager to pnpm@10.18.0
note: Package manager will be downloaded on first use.
```

## `vp env current pm --json`

current resolves the newly pinned manager

```
{
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

## `vpt print-file package.json`

the pin preserves sibling options and their policy

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
        "name": "pnpm",
        "version": "<version>",
        "onFail": "download"
      },
      {
        "name": "npm",
        "version": "<version>",
        "onFail": "error"
      }
    ]
  }
}
```
