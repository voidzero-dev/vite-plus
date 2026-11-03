# fmt_check_with_vite_config

src/valid.js follows the `singleQuote` / `semi: false` options from vite.config.ts, so it passes only when the config is read.

## `vp fmt --check`

Passes with the vite.config.ts fmt options

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 3 files using <n> threads.
```

## `vpt rm vite.config.ts`


## `vp fmt --check`

Fails with the default options once the config is removed

**Exit code:** 1

```
Checking formatting...

src/valid.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 2 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```
