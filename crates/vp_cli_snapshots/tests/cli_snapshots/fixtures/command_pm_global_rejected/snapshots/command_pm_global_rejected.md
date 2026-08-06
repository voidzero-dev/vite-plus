# command_pm_global_rejected

## `vp install -g testnpm2`

rejected: managed install

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp install -g testnpm2 --node 20`

rejected: --node implicitly covered

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp add -g testnpm2`

rejected: managed add

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp add -g testnpm2 --node 20`

rejected: --node implicitly covered

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp remove -g testnpm2`

rejected: managed uninstall

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp remove -g --dry-run testnpm2`

rejected: --dry-run implicitly covered

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp update -g`

rejected: managed update

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp update -g testnpm2`

rejected: managed update with package

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```

## `vp pm ls -g`

rejected: managed packages listing

**Exit code:** 1

```
error: Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.
```
