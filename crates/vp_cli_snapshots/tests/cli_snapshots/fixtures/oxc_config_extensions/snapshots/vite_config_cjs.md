# vite_config_cjs

Known upstream gap: auto-discovery ignores vite.config.cjs, but explicit loading works. An upstream fix must change this snapshot.

## `vpt cp config.cjs vite.config.cjs`


## `vp lint src`

Automatic discovery should apply the lint block in vite.config.cjs.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt --check src`

Automatic discovery should apply the fmt block in vite.config.cjs.

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```

## `vp check src`

The composite command should use the same config.

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vp lint -c vite.config.cjs src`

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

## `vp fmt -c vite.config.cjs --check src`

Explicit loading confirms that the fmt config is valid.

**Exit code:** 1

```
Checking formatting...

src/index.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
```
