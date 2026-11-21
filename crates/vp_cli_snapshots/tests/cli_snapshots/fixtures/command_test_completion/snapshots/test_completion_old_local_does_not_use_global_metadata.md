# test_completion_old_local_does_not_use_global_metadata

## `vpt mkdir -p node_modules`


## `vpt cp -r package-old node_modules/vite-plus`


## `vpt cp -r package-old/artifacts node_modules/vite-plus/dist`


## `vpt cp -r package-global global`


## `vpt cp -r package-global/artifacts global/dist`


## `vpt mkdir -p global/node_modules/vitest`


## `vpt cp package-global/vitest-package.json global/node_modules/vitest/package.json`


## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist vp -- vp test --glob`

An old runnable local package must not use a newer global completion schema

```
```

## `vpt rm node_modules/vite-plus/dist/bin.js`


## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist vp -- vp test --glob`

Falls back globally only when the local CLI is not runnable

```
--global-option
```
