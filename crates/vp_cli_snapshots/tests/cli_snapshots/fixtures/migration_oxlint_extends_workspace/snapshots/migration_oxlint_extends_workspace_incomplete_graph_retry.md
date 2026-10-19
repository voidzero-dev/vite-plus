# migration_oxlint_extends_workspace_incomplete_graph_retry

## `vpt rm .oxlintrc.json`


## `vpt write-file packages/a/.oxlintrc.json '{"rules":{"no-debugger":"error"},"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt write-file packages/b/.oxlintrc.json '{"extends":["./missing.json"]}'`


## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

Original rules without inheritance

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

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
! Warnings:
  - Could not migrate packages/b/.oxlintrc.json: Error: ENOENT: no such file or directory, lstat '<workspace>/packages/b/missing.json'. The original config was preserved; use vp lint -c packages/b/.oxlintrc.json until it is migrated manually.
```

## `vpt stat-file packages/a/.oxlintrc.json --assert file`

An unreadable graph keeps completed JSON configs on disk

```
packages/a/.oxlintrc.json: file
```

## `vpt stat-file packages/a/.oxlintrc.ts --assert file`

```
packages/a/.oxlintrc.ts: file
```

## `vpt write-file packages/b/missing.json '{"rules":{"no-debugger":"error"},"options":{"typeAware":false,"typeCheck":false}}'`

Repair the unreadable inheritance graph

```
```

## `cd packages/b && vp lint --threads=1 -c .oxlintrc.json index.js`

The repaired JSON loads before retrying

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

## `vpt stat-file packages/a/.oxlintrc.json --assert missing`

```
packages/a/.oxlintrc.json: missing
```

## `vpt stat-file packages/b/.oxlintrc.json --assert missing`

```
packages/b/.oxlintrc.json: missing
```

## `cd packages/a && vp lint --threads=1 index.js`

The completed consumer retains its diagnostic

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

## `cd packages/b && vp lint --threads=1 index.js`

The repaired consumer retains its diagnostic after retrying

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
