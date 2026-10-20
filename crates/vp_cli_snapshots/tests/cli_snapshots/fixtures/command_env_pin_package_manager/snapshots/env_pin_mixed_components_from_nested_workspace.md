# env_pin_mixed_components_from_nested_workspace

## `vp env pin 22.13.0 yarn@4.12.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated devEngines.runtime in <workspace>/packages/app/package.json
note: Version will be downloaded on first use.
warn: Current environment resolves to npm from devEngines.packageManager, but yarn was requested.
✓ Pinned package manager to yarn@4.12.0
  File: ../../package.json
  Field: devEngines.packageManager
note: Package manager will be downloaded on first use.
```

## `vpt print-file ../../package.json package.json`

Node is local while the package manager belongs to the workspace

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

## `vp env pin --unpin`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed devEngines.runtime node entry from <workspace>/packages/app/package.json
✓ Removed package-manager pin
  File: ../../package.json
  Field: devEngines.packageManager
```

## `vpt print-file ../../package.json package.json`

mixed removal uses the same separate targets

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
