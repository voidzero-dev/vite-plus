# lint_explicit_config

## `vp lint index.js`

use inline lint rules when no config flag is supplied

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --config custom-lint.json index.js`

an explicit config overrides the inline lint rules

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint -c custom-lint.json index.js`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --config=custom-lint.json index.js`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint -c=custom-lint.json index.js`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint -ccustom-lint.json index.js`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp run lint:custom`

```
$ vp lint --config custom-lint.json index.js ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt write-file vite.config.ts 'throw new Error('\''Vite config must not be loaded'\'');
'`


## `vp lint --config custom-lint.json index.js`

explicit lint configs do not require a working Vite config

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
