# env_unpin_package_manager_target_preserves_dev_engines

## `vpt write-file package.json '{"name":"command-env-pin-package-manager","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"10.18.0","onFail":"download"}}}
'`


## `vp env unpin pm --target package-manager`

an explicit top-level target does not remove a devEngines-only package-manager pin

```
VITE+ - The Unified Toolchain for the Web

No package manager pin found in current directory.
```

## `vpt print-file package.json`

the devEngines package-manager pin remains unchanged

```
{"name":"command-env-pin-package-manager","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"<version>","onFail":"download"}}}
```
