# migration_oxlint_extends_workspace_preserved_consumer

## `vpt replace-file-content .oxlintrc.json '"no-debugger": "warn"' '"no-debugger": "error"'`


## `cd packages/b && vp lint -c .oxlintrc.json index.js`

The original JSON consumer inherits an error through both workspaces

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

Root and package a migrate; package b keeps its JSON because extra.ts exists

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

## `vpt print-file vite.config.mjs`

Retaining JSON does not undo the owner's successful migration

```
// vp migrate: merged .oxlintrc.json (bf57dd1b2fb51cd1cb30e45e36742e3b).
import oxlintConfig1 from "./.oxlintrc.js";
export default {
  fmt: {},
  lint: {
    extends: [oxlintConfig1],
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
};
```

## `vpt print-file packages/a/vite.config.mjs`

```
// vp migrate: merged packages%2Fa%2F.oxlintrc.json (c59f567503f7d6b118c01736f9206ec4).
import oxlintConfig1 from "./.oxlintrc.js";
export default {
  lint: {
    extends: [oxlintConfig1],
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
};
```

## `vpt print-file packages/b/vite.config.mjs`

The preserved consumer's Vite config has no partial lint config

```
export default {};
// vp migrate: pending .oxlintrc.json.
```

## `vpt print-file packages/b/extra.ts`

The conflicting file is unchanged

```
export default {};
```

## `cd packages/b && vp lint -c .oxlintrc.json index.js`

The JSON fallback still reports the same diagnostic and exit code

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

## `vp lint --threads=1 -c packages/b/.oxlintrc.json packages/b/index.js`

The fallback command shown at the workspace root selects the preserved package config

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[packages/b/index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint index.js`

The migrated owner uses its shared module

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

## `cd packages/a && vp lint index.js`

The migrated intermediate owner uses its shared module

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
