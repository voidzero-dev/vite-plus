# vpx_script_cache_async_hooks

Off-thread ESM hooks inherit the cache policy; CommonJS hooks retain their own cache counters.

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

```
cache edited: esm=two cjs=two jsx=h run=3
```

## `vpt json-edit tsconfig.json compilerOptions.jsxFactory '"j"'`


## `vpx ./probe.tsx two j configured`

```
cache configured: esm=two cjs=two jsx=j run=4
```

## `vpx ./probe.tsx two j warm`

```
cache warm: esm=two cjs=two jsx=j run=5
```
