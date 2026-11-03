# discovered_format_options

## `vp fmt --check index.js`

Oxfmt discovers the package format settings.

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp check --no-lint index.js`

The check command keeps the root format settings.

**Exit code:** 1

```
error: Formatting issues found
index.js (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
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

Direct formatting does not change the root settings used by check.

**Exit code:** 1

```
error: Formatting issues found
index.js (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
```
