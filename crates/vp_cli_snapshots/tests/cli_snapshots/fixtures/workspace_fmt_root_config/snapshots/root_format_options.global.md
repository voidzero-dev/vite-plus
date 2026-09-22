# root_format_options

## `vp fmt --check index.js`

The package format settings cannot replace the root settings.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Checking formatting...

index.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
```

## `vp check --no-lint index.js`

The formatting phase of check uses the same root config.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Formatting issues found
index.js (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
```

## `vp fmt index.js`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file index.js`

The root single-quote and semicolon settings apply without changing the package working directory.

```
export const message = 'hello'
```

## `vp check --no-lint index.js`

```
VITE+ - The Unified Toolchain for the Web

pass: All 1 file are correctly formatted (<duration>, <n> threads)
```
