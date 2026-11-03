# workspace_lint_subpackage

## `vp lint packages/app-a`

Running from the root uses the root no-console:warn rule.

```

  ⚠ eslint(no-console): Unexpected console statement.
   ╭─[packages/app-a/src/index.js:2:3]
 1 │ function hello() {
 2 │   console.log('hello from app-a');
   ·   ───────────
 3 │   return 'hello';
   ╰────
  help: Delete this console statement.

Found 1 warning and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `cd packages/app-a && vp lint`

Running from the package discovers its no-console:off rule.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `cd packages/app-a && vp lint -c vite.config.ts`

Explicitly selecting the package config uses its no-console:off rule.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt write-file packages/app-a/vite.config.ts 'export default {};
'`

```
```

## `cd packages/app-a && vp lint`

A package config without lint settings also uses the root config.

```

  ⚠ eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:2:3]
 1 │ function hello() {
 2 │   console.log('hello from app-a');
   ·   ───────────
 3 │   return 'hello';
   ╰────
  help: Delete this console statement.

Found 1 warning and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
