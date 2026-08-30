# env_unpin_targets_shadowed_dev_engines_manager

## `vpt write-file package.json '{"name":"command-env-pin-package-manager","private":true,"packageManager":"pnpm@10.18.0","devEngines":{"packageManager":{"name":"yarn","version":"4.12.0","onFail":"download"}}}
'`


## `vp env unpin pm --target dev-engines`

an explicit target removes the devEngines manager even when packageManager shadows it

```
VITE+ - The Unified Toolchain for the Web

✓ Removed package-manager pin
```

## `vpt print-file package.json`

the effective top-level packageManager remains intact

```
{
  "name": "command-env-pin-package-manager",
  "private": true,
  "packageManager": "pnpm@10.18.0",
  "devEngines": {}
}
```
