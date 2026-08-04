# toolchain_unknown

## `vp toolchain rollup`

human output points back to the package graph

**Exit code:** 1

```
error: `rollup` is not part of the Vite+ toolchain manifest
hint: run `vp why rollup` to inspect project dependencies
```

## `vp toolchain rollup --json`

JSON mode omits the human hint

**Exit code:** 1

```
error: `rollup` is not part of the Vite+ toolchain manifest
```
