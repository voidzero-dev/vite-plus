# discovered_format_options

## `vp fmt --check index.js`

Oxfmt discovers the package format settings.

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp check --no-lint index.js`

The formatting phase of check also discovers the package config.

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```

## `vp fmt index.js`

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file index.js`

The package double-quote and semicolon settings apply with package-relative file paths.

```
export const message = "hello";
```

## `vp check --no-lint index.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```
