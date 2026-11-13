# test_completion_follows_local_toolchain

## `vpt mkdir -p node_modules`


## `vpt cp -r package-alpha node_modules/vite-plus`


## `vpt cp -r package-alpha/artifacts node_modules/vite-plus/dist`


## `vpt mkdir -p node_modules/vite-plus/node_modules/vitest`


## `vpt cp package-alpha/vitest-package.json node_modules/vite-plus/node_modules/vitest/package.json`


## `vpt mkdir -p other/node_modules`


## `vpt cp -r package-beta other/node_modules/vite-plus`


## `vpt cp -r package-beta/artifacts other/node_modules/vite-plus/dist`


## `vpt mkdir -p other/node_modules/vite-plus/node_modules/vitest`


## `vpt cp package-beta/vitest-package.json other/node_modules/vite-plus/node_modules/vitest/package.json`


## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist vp -- vp test --al`

Uses the runnable local CLI metadata without starting Node

```
--alpha
```

## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist _CLAP_COMPLETE_INDEX=4 vp -- vp -C other test --be`

Reads metadata from the directory selected inside the completion payload

```
--beta
```

## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist _CLAP_COMPLETE_INDEX=3 vp -- vp -Cother test --be`

```
--beta
```

## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist _CLAP_COMPLETE_INDEX=3 vp -- vp -C=other test --be`

```
--beta
```
