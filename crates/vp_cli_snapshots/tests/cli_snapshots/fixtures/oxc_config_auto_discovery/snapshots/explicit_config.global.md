# explicit_config

## `vp lint -c custom-lint.json src`

An explicit lint config overrides the root config.

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt --config custom-fmt.json src`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.js`

The explicit fmt config keeps double quotes and semicolons.

```
console.log("hello");
```

## `vp lint --config=custom-lint.json src`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt -c custom-fmt.json --check src`

```
VITE+ - The Unified Toolchain for the Web

Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```
