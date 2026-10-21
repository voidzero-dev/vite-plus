# migration_oxlint_extends_backslash_path

## `vpt cp shared/base.json shared/base\name.json`


## `vpt json-edit shared/nested/first.json extends '["../base\\name.json"]'`


## `vp lint --threads=1 -c .oxlintrc.json index.js`

Original diagnostics with a literal backslash in the shared config filename

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
 2 │ console.log("test");
   ╰────
  help: Remove the debugger statement

  ⚠ eslint(no-console): Unexpected console statement.
   ╭─[index.js:2:1]
 1 │ debugger;
 2 │ console.log("test");
   · ───────────
 3 │ alert("allowed");
   ╰────
  help: Delete this console statement.

  × local(no-foo): Do not use foo.
   ╭─[index.js:4:14]
 3 │ alert("allowed");
 4 │ export const foo = 1;
   ·              ───
 5 │ export const bar = 1;
   ╰────

Found 1 warning and 2 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

Warn and preserve the JSON because TypeScript treats the backslash as a separator

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
! Warnings:
  - Could not migrate .oxlintrc.json: Error: Cannot import a shared Oxlint config whose relative path contains "\\": <workspace>/shared/base\name.ts. The original config was preserved; use vp lint -c .oxlintrc.json until it is migrated manually.
```

## `vpt stat-file .oxlintrc.json --assert file`

```
.oxlintrc.json: file
```

## `vpt stat-file shared/nested/first.ts --assert missing`

```
shared/nested/first.ts: missing
```

## `vpt stat-file shared/base\name.ts --assert missing`

```
shared/base\name.ts: missing
```

## `vp lint --threads=1 -c .oxlintrc.json index.js`

The retained JSON still produces the same diagnostics and exit code

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
 2 │ console.log("test");
   ╰────
  help: Remove the debugger statement

  ⚠ eslint(no-console): Unexpected console statement.
   ╭─[index.js:2:1]
 1 │ debugger;
 2 │ console.log("test");
   · ───────────
 3 │ alert("allowed");
   ╰────
  help: Delete this console statement.

  × local(no-foo): Do not use foo.
   ╭─[index.js:4:14]
 3 │ alert("allowed");
 4 │ export const foo = 1;
   ·              ───
 5 │ export const bar = 1;
   ╰────

Found 1 warning and 2 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
