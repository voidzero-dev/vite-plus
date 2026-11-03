# toolchain_unknown

## `vp toolchain rollup`

readable output points to the package graph

**Exit code:** 1

```
error: `rollup` is not in the Vite+ toolchain
hint: run `vp why rollup` to show project dependencies
```

## `vp toolchain rollup --json`

JSON output does not include the readable hint

**Exit code:** 1

```
error: `rollup` is not in the Vite+ toolchain
```
