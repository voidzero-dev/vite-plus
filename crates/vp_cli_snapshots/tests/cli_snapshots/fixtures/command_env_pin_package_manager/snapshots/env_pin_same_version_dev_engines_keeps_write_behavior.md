# env_pin_same_version_dev_engines_keeps_write_behavior

## `vpt write-file package.json '{"devEngines": {"packageManager": [{"name": "pnpm", "version": "10.18.0", "onFail": "error"}, {"name": "yarn", "version": "4.12.0"}]}}
'`


## `vp env pin pnpm@10.18.0 --no-install`

equal versions retain the existing write path rather than reporting a no-op

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned package manager to pnpm@10.18.0
  File: package.json
  Field: devEngines.packageManager
note: Package manager will be downloaded on first use.
```

## `vpt print-file package.json`

```
{
  "devEngines": {
    "packageManager": [
      {
        "name": "pnpm",
        "version": "<version>",
        "onFail": "error"
      },
      {
        "name": "yarn",
        "version": "<version>"
      }
    ]
  }
}
```
