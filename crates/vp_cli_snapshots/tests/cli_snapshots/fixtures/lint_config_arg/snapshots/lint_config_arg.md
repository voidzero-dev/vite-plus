# lint_config_arg

## `vp lint -c custom.json`

Should apply lint from custom.json

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/invalid.js:2:3]
 1 │ export function example() {
 2 │   console.log('hello');
   ·   ───────────
 3 │   return 'hello';
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 3 files with <n> rules using <n> threads.
```

## `vp lint -c=custom.json`

Should apply lint from custom.json

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/invalid.js:2:3]
 1 │ export function example() {
 2 │   console.log('hello');
   ·   ───────────
 3 │   return 'hello';
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 3 files with <n> rules using <n> threads.
```

## `vp lint --config custom.json`

Should apply lint from custom.json

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/invalid.js:2:3]
 1 │ export function example() {
 2 │   console.log('hello');
   ·   ───────────
 3 │   return 'hello';
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 3 files with <n> rules using <n> threads.
```

## `vp lint --config=custom.json`

Should apply lint from custom.json

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/invalid.js:2:3]
 1 │ export function example() {
 2 │   console.log('hello');
   ·   ───────────
 3 │   return 'hello';
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 3 files with <n> rules using <n> threads.
```
