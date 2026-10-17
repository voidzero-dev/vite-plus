# migration_oxlint_extends

## `vp lint --threads=1 -c .oxlintrc.json`

Original inherited errors, warnings, and overrides

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

Found 2 warnings and 3 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
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

## `vpt print-file vite.config.ts`

Path extends become imports of shared TypeScript configs

```
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";
import { defineConfig } from "vite-plus";

export default defineConfig({
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
});
```

## `vpt print-file shared/nested/first.ts`

Nested imports preserve overrides and their plugin declarations

```
import { fileURLToPath } from "node:url";
import type { OxlintConfig } from "vite-plus/lint";
import oxlintConfig1 from "../base.js";

export default {
  extends: [oxlintConfig1],
  overrides: [
    {
      files: ["**/*.test.js"],
      rules: {
        "no-debugger": "warn",
        "override/no-bar": "error",
      },
      jsPlugins: [
        {
          name: "override",
          specifier: fileURLToPath(new URL("../override-plugin.mjs", import.meta.url)),
        },
      ],
    },
  ],
} satisfies OxlintConfig;
```

## `vpt print-file shared/base.ts`

```
import type { OxlintConfig } from "vite-plus/lint";

export default {
  options: {
    typeAware: false,
    typeCheck: false,
  },
  rules: {
    "no-debugger": "error",
    "no-console": "error",
    "no-alert": "error",
  },
} satisfies OxlintConfig;
```

## `vpt print-file shared/second.ts`

Relative plugin paths resolve from the shared module

```
import { fileURLToPath } from "node:url";
import type { OxlintConfig } from "vite-plus/lint";

export default {
  rules: {
    "no-console": "warn",
  },
  jsPlugins: [fileURLToPath(new URL("./plugin.mjs", import.meta.url))],
} satisfies OxlintConfig;
```

## `vpt stat-file .oxlintrc.json --assert-not file`

```
.oxlintrc.json: missing
```

## `vpt print-file shared/second.jsonc`

Shared JSON configs are retained

```
{
  // Later entries take priority.
  "rules": { "no-console": "warn" },
  "jsPlugins": ["./plugin.mjs"],
}
```

## `vp exec tsc --noEmit`

Generated config types accept the resolved inheritance

```
```

## `vp lint --threads=1`

The diagnostics and exit code match the original config without a config flag

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

Found 2 warnings and 3 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

A second migration leaves the config unchanged

```
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";
import { defineConfig } from "vite-plus";

export default defineConfig({
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
});
```

## `vp lint --threads=1`

The diagnostics and exit code remain unchanged after the second migration

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

Found 2 warnings and 3 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt replace-file-content shared/base.ts '"no-debugger": "error"' '"no-debugger": "warn"'`

Edits to the shared TS config propagate through its imports

```
```

## `vp lint --threads=1`

The edited base rule now warns in index.js; the test override remains a warning

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

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
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

Found 3 warnings and 2 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
