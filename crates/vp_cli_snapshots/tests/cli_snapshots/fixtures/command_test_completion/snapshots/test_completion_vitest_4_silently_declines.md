# test_completion_vitest_4_silently_declines

## `vpt mkdir -p node_modules`


## `vpt cp -r package-alpha node_modules/vite-plus`


## `vpt cp -r package-alpha/artifacts node_modules/vite-plus/dist`


## `vpt mkdir -p node_modules/vite-plus/node_modules/vitest`


## `vpt write-file node_modules/vite-plus/node_modules/vitest/package.json '{"name":"vitest","version":"4.1.10"}'`


## `vpt cp -r package-global global`


## `vpt cp -r package-global/artifacts global/dist`


## `vpt mkdir -p global/node_modules/vitest`


## `vpt cp package-global/vitest-package.json global/node_modules/vitest/package.json`


## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist vp -- vp test --al`

Stale Vitest 5 metadata with legacy Vitest: empty stdout/stderr and success

```
```

## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist vp -- vp test --glob`

Does not borrow the global toolchain's Vitest 5 metadata

```
```

## `vpt rm node_modules/vite-plus/dist/completion/test.json`


## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global/dist vp -- vp test --al`

Missing metadata also completes silently

```
```

## `vpt cp package-alpha/artifacts/completion/test.json node_modules/vite-plus/dist/completion/test.json`


## `vpt cp package-alpha/vitest-package.json node_modules/vite-plus/node_modules/vitest/package.json`


## `vp -- vp test --al`

Positive control: matched Vitest 5 metadata completes again, without executing tests

```
--alpha
```
