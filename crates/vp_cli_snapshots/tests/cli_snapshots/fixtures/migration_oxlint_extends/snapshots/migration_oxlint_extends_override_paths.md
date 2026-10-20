# migration_oxlint_extends_override_paths

## `vpt write-file .oxlintrc.json '{"extends":["./configs/base.json"]}'`


## `vpt write-file configs/base.json '{"options":{"typeAware":false,"typeCheck":false},"rules":{"no-debugger":"error"},"overrides":[{"files":["src/**"],"excludeFiles":["src/skip.js"],"rules":{"no-debugger":"off"}}]}'`


## `vpt write-file src/allowed.js 'debugger;
'`


## `vpt write-file src/skip.js 'debugger;
'`


## `vpt write-file configs/src/allowed.js 'debugger;
'`


## `vpt write-file configs/src/skip.js 'debugger;
'`


## `vp lint --threads=1 -c .oxlintrc.json src/allowed.js`

Original inherited override matches the consumer-relative path

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 -c .oxlintrc.json src/skip.js`

Original excludeFiles prevents the override

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[src/skip.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 -c .oxlintrc.json configs/src/allowed.js`

The declaring directory does not rebase the inherited glob

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[configs/src/allowed.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 -c .oxlintrc.json configs/src/skip.js`

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[configs/src/skip.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 2 config updates applied
```

## `vp lint --threads=1 src/allowed.js`

The imported override matches the same path

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 src/skip.js`

The imported exclusion has the same effect

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[src/skip.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 configs/src/allowed.js`

The declaring-directory path retains its original diagnostic

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[configs/src/allowed.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 configs/src/skip.js`

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[configs/src/skip.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
