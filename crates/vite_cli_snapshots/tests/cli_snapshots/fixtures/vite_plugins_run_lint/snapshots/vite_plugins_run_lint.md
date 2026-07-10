# vite_plugins_run_lint

## `vp run lint-task`

vp run lint should not load plugins (heavy-plugin.ts throws if imported)

```
$ vp lint src/ ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
