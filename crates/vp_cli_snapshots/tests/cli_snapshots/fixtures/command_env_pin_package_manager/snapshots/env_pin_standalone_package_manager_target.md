# env_pin_standalone_package_manager_target

## `vpt write-file package.json '{"name": "standalone"}
'`


## `vp env pin yarn@4.12.0 --no-install`

first write uses this project manifest without a confirmation

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned package manager to yarn@4.12.0
  File: package.json
  Field: devEngines.packageManager
note: Package manager will be downloaded on first use.
```

## `vpt print-file package.json`

```
{
  "name": "standalone",
  "devEngines": {
    "packageManager": {
      "name": "yarn",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp env unpin pm`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed package-manager pin
  File: package.json
  Field: devEngines.packageManager
```

## `vpt print-file package.json`

```
{
  "name": "standalone",
  "devEngines": {}
}
```
