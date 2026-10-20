# env_unpin_reports_empty_and_missing_manifest

## `vpt write-file package.json '{"name": "empty"}
'`


## `vp env unpin pm`

```
VITE+ - The Unified Toolchain for the Web

No matching package-manager pin found. No changes made.
  File: package.json
  Checked fields: devEngines.packageManager
```

## `vp env unpin yarn --target dev-engines`

```
VITE+ - The Unified Toolchain for the Web

No matching package-manager pin found. No changes made.
  File: package.json
  Checked fields: devEngines.packageManager
  Scope: yarn
```

## `vp env unpin pm --target package-manager`

```
VITE+ - The Unified Toolchain for the Web

No matching package-manager pin found. No changes made.
  File: package.json
  Checked fields: packageManager
```

## `vpt rm package.json`

```
```

## `vp env unpin pm`

a missing file is distinct from a readable manifest with no matching pin

```
VITE+ - The Unified Toolchain for the Web

No package.json found at package.json. No changes made.
```
