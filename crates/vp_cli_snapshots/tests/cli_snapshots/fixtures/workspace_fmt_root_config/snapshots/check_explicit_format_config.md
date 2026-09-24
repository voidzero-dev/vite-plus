# check_explicit_format_config

## `vp check --no-lint -- -c vite.config.ts index.js`

An explicit config passed through check still takes precedence over the root config.

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```

## `vp check --no-lint -- --config vite.config.ts index.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```

## `vp check --no-lint -- --config=vite.config.ts index.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```
