# migration_oxlint_extends_workspace_repaired_shared_owner

## `vpt rm packages/b/.oxlintrc.json`


## `vpt write-file packages/a/.oxlintrc.json '{"extends":["./missing.json"]}'`


## `vp lint --threads=1 -c .oxlintrc.json index.js`

The original owner reports a warning

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◆ ✔ Merged .oxlintrc.json into vite.config.mjs
● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
! Warnings:
  - Could not migrate packages/a/.oxlintrc.json: Error: ENOENT: no such file or directory, lstat '<workspace>/packages/a/missing.json'. The original config was preserved; use vp lint -c packages/a/.oxlintrc.json until it is migrated manually.
```

## `vpt write-file packages/a/missing.json '{"extends":["../../.oxlintrc.json"]}'`

The repaired graph reveals that the root is a shared owner

```
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

The repaired JSON inherits the original warning

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 1 config update applied
```

## `vp lint --threads=1 index.js`

The migrated owner retains its warning

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 index.js`

The repaired consumer retains the same warning

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt replace-file-content .oxlintrc.ts '"no-debugger": "warn"' '"no-debugger": "error"'`

```
```

## `vp lint --threads=1 index.js`

The owner follows the shared module edit

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 index.js`

The repaired consumer follows the same edit

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
