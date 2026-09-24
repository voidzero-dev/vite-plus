# conflicting_package_options

## `vpt write-file vite.config.ts 'export default { lint: { options: { typeAware: false, typeCheck: false }, rules: { '\''no-console'\'': '\''off'\'' } } };
'`


## `vp lint index.ts`

Oxlint uses the package rules and type-check options.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp check --no-fmt index.ts`

```
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```

## `vp lint -c vite.config.ts index.ts`

An explicit package config matches native discovery.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --config vite.config.ts index.ts`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --config=vite.config.ts index.ts`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
