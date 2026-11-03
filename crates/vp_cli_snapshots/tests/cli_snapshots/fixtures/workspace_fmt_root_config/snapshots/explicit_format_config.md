# explicit_format_config

## `vp fmt -c vite.config.ts --check index.js`

An explicit package config matches native discovery.

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt --config vite.config.ts --check index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt --config=vite.config.ts --check index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt -c ../../vite.config.ts --check index.js`

Selecting the root config explicitly detects the conflicting format.

**Exit code:** 1

```
Checking formatting...

index.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
```
