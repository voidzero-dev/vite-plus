# migration_oxlint_extends_check_js

## `vpt write-file vite.config.js 'export default {};
'`


## `vpt json-edit tsconfig.json compilerOptions.allowJs true`


## `vpt json-edit tsconfig.json compilerOptions.checkJs true`


## `vpt json-edit tsconfig.json include '["vite.config.js"]'`


## `vp lint --threads=1 -c .oxlintrc.json index.js`

Original inherited diagnostics

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
• 1 config update applied
```

## `vpt print-file vite.config.js`

```
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";
export default {
  fmt: {},
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
};
```

## `vp exec tsc --noEmit`

JavaScript config imports type-check without allowImportingTsExtensions

```
```

## `vp lint --threads=1 index.js`

The generated imports preserve the diagnostics and exit code

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
