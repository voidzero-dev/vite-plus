# env_use_does_not_warn_for_different_default

## `vp env default pnpm@10.18.0`


## `vpt write-file package.json '{"name":"command-env-package-manager-mismatch","private":true}
'`


## `vp env use yarn@4.12.0 --no-install`

a different fallback manager does not warn

```
export VP_PACKAGE_MANAGER=yarn@4.12.0
Using yarn <version> (resolved from 4.12.0)
```
