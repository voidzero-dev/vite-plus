# discover_root_config

## `vp lint src`

The root lint config enables no-console.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt src`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.js`

The root fmt config selects single quotes and no semicolons.

```
console.log('hello')
```

## `vp check src`

The composite command uses the same discovered settings.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

pass: All 1 file are correctly formatted (<duration>, <n> threads)
error: Lint issues found
× eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 1 error and 0 warnings in 1 file (<duration>, <n> threads)
```
