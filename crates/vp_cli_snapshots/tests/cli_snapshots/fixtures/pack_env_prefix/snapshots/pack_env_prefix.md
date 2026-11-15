# pack_env_prefix

The env prefix comes from the --env-prefix flag first, then pack.envPrefix in
the config file, then the VITE_PACK_ and TSDOWN_ defaults.

## `vp pack --logLevel silent`

pack.envPrefix from the config file replaces the defaults

```
ℹ Build start
```

## `vpt print-file dist/index.mjs`

```
// src/index.ts
console.log("from-app", import.meta.env.VITE_PACK_BAR);
export {};
```

## `vp pack --logLevel silent --env-prefix VITE_PACK_`

the flag overrides the config file

```
ℹ Build start
ℹ Cleaning <n> files
```

## `vpt print-file dist/index.mjs`

```
// src/index.ts
console.log(import.meta.env.APP_FOO, "from-vite-pack");
export {};
```

## `vp pack --logLevel silent --no-config`

without a config file the defaults apply

```
ℹ Build start
ℹ Cleaning <n> files
```

## `vpt print-file dist/index.mjs`

```
// src/index.ts
console.log(import.meta.env.APP_FOO, "from-vite-pack");
export {};
```
