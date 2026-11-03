# vite_config_mts

Auto-discovery and explicit loading both support vite.config.mts.

## `vpt cp config.mjs vite.config.mts`


## `vp lint src`

Automatic discovery should apply the lint block in vite.config.mts.

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt --check src`

Automatic discovery should apply the fmt block in vite.config.mts.

**Exit code:** 1

```
Checking formatting...

src/index.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
```

## `vp check src`

The composite command should use the same config.

**Exit code:** 1

```
error: Formatting issues found
src/index.js (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
```

## `vp lint -c vite.config.mts src`

Explicit loading confirms that the lint config is valid.

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt -c vite.config.mts --check src`

Explicit loading confirms that the fmt config is valid.

**Exit code:** 1

```
Checking formatting...

src/index.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
```
