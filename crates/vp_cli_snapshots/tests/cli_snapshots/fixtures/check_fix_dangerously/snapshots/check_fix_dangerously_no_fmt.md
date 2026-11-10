# check_fix_dangerously_no_fmt

## `vp check --fix-dangerously --no-fmt src/index.js`

```
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vpt print-file src/index.js`

```

export function greet(enabled) {
  if (enabled) {return "hello";}
  return "world";
}
```

## `vp fmt --check src/index.js`

formatting remains skipped after lint fixes

**Exit code:** 1

```
Checking formatting...

src/index.js (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```
