# check_fix_dangerously_continue_on_error

## `vpt write-file src/error.js 'eval('\''value'\'');
'`

```
```

## `vpt write-file vite.config.ts 'export default { lint: { categories: { correctness: '\''off'\'' }, rules: { '\''no-unused-vars'\'': '\''error'\'', curly: '\''error'\'', '\''no-eval'\'': '\''error'\'' } } };
'`

```
```

## `vp check --fix-dangerously --continue-on-error`

remaining lint errors preserve a nonzero exit while the final format pass runs

**Exit code:** 1

```
error: Lint issues found
× eslint(no-eval): eval can be harmful.
   ╭─[src/error.js:1:1]
 1 │ eval("value");
   · ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

Found 1 error and 0 warnings in 3 files (<duration>, <n> threads)
pass: Formatting completed for checked files (<duration>)
```

## `vpt print-file src/index.js`

```
export function greet(enabled) {
  if (enabled) {
    return "hello";
  }
  return "world";
}
```

## `vp fmt --check src/index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```
