# migration_oxlint_extends_missing_plugin

## `vp lint --threads=1 -c .oxlintrc.json index.js`

Original inherited diagnostics before a missing generated plugin is added

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

## `vpt json-edit .oxlintrc.json jsPlugins '["eslint-plugin-vp-missing"]'`


## `vpt json-edit .oxlintrc.json rules.vp-missing/no-foo '"error"'`


## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

Sanitize missing generated package references without discarding inheritance

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 2 config updates applied
! Warnings:
  - Stripped JS plugin reference(s) from the generated lint config: eslint-plugin-vp-missing. No matching package is present in this workspace, so loading them at lint time would fail. If you want their Oxlint coverage back, install each package (e.g. `vp install <name>`) and add its name back to `lint.jsPlugins` in vite.config.ts.
```

## `vpt print-file vite.config.ts`

The missing plugin and its rule are removed while inherited plugin rules remain

```
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
  lint: {
    extends: [oxlintConfig1, oxlintConfig2],
    ignorePatterns: ["shared/**", "vite.config.ts"],
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "local/no-foo": "error",
      "no-alert": "off",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    options: {},
  },
});
```

## `vp lint --threads=1 index.js`

Lint loads successfully and reports the inherited diagnostics

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

## `vp lint --threads=1 example.test.js`

Inherited override plugins still run

**Exit code:** 1

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[example.test.js:1:1]
 1 │ debugger;
   · ─────────
 2 │ export const bar = 1;
   ╰────
  help: Remove the debugger statement

  × override(no-bar): Do not use bar.
   ╭─[example.test.js:2:14]
 1 │ debugger;
 2 │ export const bar = 1;
   ·              ───
   ╰────

Found 1 warning and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
