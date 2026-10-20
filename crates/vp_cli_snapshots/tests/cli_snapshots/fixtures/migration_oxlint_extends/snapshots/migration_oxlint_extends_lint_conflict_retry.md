# migration_oxlint_extends_lint_conflict_retry

## `vpt write-file vite.config.mjs 'export default { lint: {} };
'`


## `vp lint --threads=1 -c .oxlintrc.json index.js`

The original JSON inheritance is selected explicitly

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

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
! Warnings:
  - Preserved .oxlintrc.json because vite.config.mjs already has a lint config. Merge its inherited rules manually.
```

## `vpt print-file vite.config.mjs`

The lint conflict records a pending marker

```
export default {
  fmt: {},
  lint: {},
};
// vp migrate: pending .oxlintrc.json.
```

## `vpt replace-file-content vite.config.mjs 'lint: {},' `

Remove the conflicting lint block

```
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

## `vpt stat-file .oxlintrc.json --assert missing`

```
.oxlintrc.json: missing
```

## `vpt print-file vite.config.mjs`

```
// vp migrate: merged .oxlintrc.json (6e95c926c620f96efc4952a2de4580b0).
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";
export default {
  lint: {
    extends: [oxlintConfig1, oxlintConfig2],
    rules: {
      "no-alert": "off",
      "local/no-foo": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    ignorePatterns: ["shared/**", "vite.config.ts"],
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
  },
  fmt: {},
};
```

## `vp lint --threads=1 index.js`

The retried migration retains the original diagnostics and exit code

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
