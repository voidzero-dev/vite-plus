# env_unpin_scoped_manager_falls_back_to_dev_engines

## `vpt write-file package.json '{"packageManager": "pnpm@10.18.0", "devEngines": {"packageManager": [{"name": "npm", "version": "11.0.0"}, {"name": "yarn", "version": "4.12.0"}]}}
'`


## `vp env unpin yarn`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed package-manager pin
  File: package.json
  Field: devEngines.packageManager
```

## `vpt print-file package.json`

only the matching devEngines entry was removed

```
{
  "packageManager": "pnpm@10.18.0",
  "devEngines": {
    "packageManager": [
      {
        "name": "npm",
        "version": "<version>"
      }
    ]
  }
}
```

## `vp env unpin yarn`

a miss reports both searched fields and the explicit scope

```
VITE+ - The Unified Toolchain for the Web

No matching package-manager pin found. No changes made.
  File: package.json
  Checked fields: packageManager, devEngines.packageManager
  Scope: yarn
```

## `vp env unpin yarn --target package-manager`

an explicit top-level target checks only that field

```
VITE+ - The Unified Toolchain for the Web

No matching package-manager pin found. No changes made.
  File: package.json
  Checked fields: packageManager
  Scope: yarn
```
