# env_unpin_package_manager_from_nested_workspace

## `vp env unpin pm`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed package-manager pin
  File: ../../package.json
  Field: devEngines.packageManager
```

## `vpt print-file ../../package.json package.json`

remove the root manager and preserve the member manifest and sibling manager

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
      }
    ]
  }
}
{
  "name": "app",
  "private": true
}
```
