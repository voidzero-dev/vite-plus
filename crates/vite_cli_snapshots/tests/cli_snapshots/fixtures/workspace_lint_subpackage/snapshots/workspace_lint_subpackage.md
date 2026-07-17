# workspace_lint_subpackage

## `cd packages/app-a && vp lint`

sub-workspace has no-console:off but root has no-console:warn

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
