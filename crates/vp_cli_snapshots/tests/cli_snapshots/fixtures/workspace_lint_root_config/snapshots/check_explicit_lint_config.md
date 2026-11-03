# check_explicit_lint_config

## `vp check --no-fmt -- -c vite.config.ts index.ts`

An explicit config passed through check still takes precedence over the root config.

```
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```

## `vp check --no-fmt -- --config vite.config.ts index.ts`

```
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```

## `vp check --no-fmt -- --config=vite.config.ts index.ts`

```
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```
