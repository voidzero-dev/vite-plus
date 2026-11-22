# command_remove_global_dry_run

## `vp install -g vp-test-echo`


## `vp remove -g --dry-run vp-test-echo`

previews the shims and package files it would remove

```
Would uninstall vp-test-echo:
  - shim: <home>/.vite-plus/bin/vp-test-echo
  - shim: <home>/.vite-plus/bin/vp-test-echo-alias
  - package dir: <home>/.vite-plus/packages/vp-test-echo/<uuid>
  - metadata: <home>/.vite-plus/packages/vp-test-echo.json
```

## `vp list -g vp-test-echo`

the dry run leaves the package installed

```
Package              Node version   Binaries
---                  ---            ---
vp-test-echo@1.0.0   <version>        vp-test-echo, vp-test-echo-alias
```

## `vp-test-echo still callable`

and its shim callable

```
still callable
```
