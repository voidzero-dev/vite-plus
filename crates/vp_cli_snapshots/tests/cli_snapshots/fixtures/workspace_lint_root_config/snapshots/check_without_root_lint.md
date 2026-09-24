# check_without_root_lint

## `vpt write-file ../../vite.config.ts 'export default { fmt: {} };
'`


## `vpt write-file vite.config.ts 'export default { lint: { rules: { '\''no-console'\'': '\''error'\'' } } };
'`


## `vp check --no-fmt index.ts`

Without a root lint block, check lets Oxlint discover the package lint rules.

**Exit code:** 1

```
error: Lint issues found
× eslint(no-console): Unexpected console statement.
   ╭─[index.ts:2:1]
 1 │ export const value: number = "not a number";
 2 │ console.log(value);
   · ───────────
   ╰────
  help: Delete this console statement.

Found 1 error and 0 warnings in 1 file (<duration>, <n> threads)
```
