# vpx_script_cache_invalidation

Persistent transform and compile caches keep ESM, CommonJS, JSX settings, and side effects current. Cache corruption is repaired.

## `node mutate.mjs stamp`


## `vpx ./probe.tsx one h cold`

```
cache cold: esm=one cjs=one jsx=h run=1
```

## `vpx ./probe.tsx one h warm`

```
cache warm: esm=one cjs=one jsx=h run=2
```

## `node mutate.mjs edit`


## `vpx ./probe.tsx two h edited`

Source bytes change while file size and mtime stay identical

```
cache edited: esm=two cjs=two jsx=h run=3
```

## `vpt json-edit tsconfig.json compilerOptions.jsxFactory '"j"'`


## `vpx ./probe.tsx two j configured`

The effective JSX configuration changes the generated code

```
cache configured: esm=two cjs=two jsx=j run=4
```

## `node mutate.mjs corrupt`


## `vpx ./probe.tsx two j repaired`

```
cache repaired: esm=two cjs=two jsx=j run=5
```

## `vpx ./probe.tsx two j warm`

```
cache warm: esm=two cjs=two jsx=j run=6
```

## `VPX_DISABLE_CACHE=1 vpx ./probe.tsx two j disabled`

Disabling the managed caches still executes the same script

```
cache disabled: esm=two cjs=two jsx=j run=7
```
