# migration_oxlint_extends_isolated_retry

## `vpt write-file .oxlintrc.json '{"extends":["./missing.json"]}'`


## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
! Warnings:
  - Could not migrate .oxlintrc.json: Error: ENOENT: no such file or directory, lstat '<workspace>/missing.json'. The original config was preserved; use vp lint -c .oxlintrc.json until it is migrated manually.
```

## `vpt print-file vite.config.ts`

The failed migration records a pending marker without a lint block

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
});
// vp migrate: pending .oxlintrc.json.
```

## `vpt write-file missing.json '{"rules":{"no-debugger":"error"},"options":{"typeAware":false,"typeCheck":false}}'`

Repair the only unreadable config

```
```

## `vp lint --threads=1 -c .oxlintrc.json index.js`

The repaired JSON loads before retrying

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
 2 │ console.log("test");
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

## `vpt stat-file .oxlintrc.json --assert missing`

```
.oxlintrc.json: missing
```

## `vpt print-file vite.config.ts`

The successful retry replaces the pending marker with the lint config

```
// vp migrate: merged .oxlintrc.json (ddb1d7ee7999d49e4e56c1776d652081).
import oxlintConfig1 from "./missing.js";
import { defineConfig } from "vite-plus";

export default defineConfig({
  lint: {
    extends: [oxlintConfig1],
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
  fmt: {},
});
```

## `vp lint --threads=1 index.js`

The migrated config retains the same diagnostic and exit code

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
 2 │ console.log("test");
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
