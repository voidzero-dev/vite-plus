# migration_oxc_reference_retry

## `vpt json-edit .oxlintrc.json rules.no-debugger '"error"'`


## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

Package b cannot merge while extra.ts conflicts with its config

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◆ ✔ Merged .oxlintrc.json into vite.config.mjs
● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
! Warnings:
  - Could not migrate packages/b/.oxlintrc.json: Error: Cannot create shared Oxlint config because <workspace>/packages/b/extra.ts or <workspace>/packages/b/extra.js already exists. The original config was preserved; use vp lint -c packages/b/.oxlintrc.json until it is migrated manually.
```

## `vpt json-edit package.json scripts.lint:a '"vp lint -c packages/a/.oxlintrc.json packages/a/index.js"'`


## `vpt json-edit package.json scripts.lint:b '"vp lint -c packages/b/.oxlintrc.json packages/b/index.js"'`


## `vpt rm packages/b/extra.ts`


## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

New script references protect both completed and pending JSON configs on retry

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt stat-file .oxlintrc.json --assert file`

```
.oxlintrc.json: file
```

## `vpt stat-file packages/a/.oxlintrc.json --assert file`

```
packages/a/.oxlintrc.json: file
```

## `vpt stat-file packages/b/.oxlintrc.json --assert file`

```
packages/b/.oxlintrc.json: file
```

## `vpt print-file packages/b/vite.config.mjs`

```
export default {};
// vp migrate: pending .oxlintrc.json.
```

## `vp run lint:a`

The completed config remains usable as JSON

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ vp lint -c packages/a/.oxlintrc.json packages/a/index.js ⊘ cache disabled

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[packages/a/index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp run lint:b`

The pending config retains its complete inheritance chain

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ vp lint -c packages/b/.oxlintrc.json packages/b/index.js ⊘ cache disabled

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[packages/b/index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt json-edit package.json scripts {}`

```
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

Removing the references allows migration and deferred cleanup to finish

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

## `vpt stat-file .oxlintrc.json --assert missing`

```
.oxlintrc.json: missing
```

## `vpt stat-file packages/a/.oxlintrc.json --assert missing`

```
packages/a/.oxlintrc.json: missing
```

## `vpt stat-file packages/b/.oxlintrc.json --assert missing`

```
packages/b/.oxlintrc.json: missing
```

## `cd packages/b && vp lint --threads=1 index.js`

Plain lint enforces the inherited error after recovery

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
