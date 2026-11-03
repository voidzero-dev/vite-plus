# conflicting_package_options

## `vpt write-file vite.config.ts 'export default { lint: { options: { typeAware: false, typeCheck: false }, rules: { '\''no-console'\'': '\''off'\'' } } };
'`


## `vp lint index.ts`

Oxlint uses the package rules and type-check options.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp check --no-fmt index.ts`

Package options cannot disable the root rules or type checking in check.

**Exit code:** 1

```
error: Lint or type issues found
× eslint(no-console): Unexpected console statement.
   ╭─[index.ts:2:1]
 1 │ export const value: number = "not a number";
 2 │ console.log(value);
   · ───────────
   ╰────
  help: Delete this console statement.

  × typescript(TS2322): Type 'string' is not assignable to type 'number'.
   ╭─[index.ts:1:14]
 1 │ export const value: number = "not a number";
   ·              ─────
 2 │ console.log(value);
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
```

## `vp lint -c vite.config.ts index.ts`

An explicit package config matches native discovery.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --config vite.config.ts index.ts`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --config=vite.config.ts index.ts`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
