# migration_oxlint_extends_workspace_owner_ignores

## `vpt rm packages/b/extra.ts`


## `vpt json-edit .oxlintrc.json ignorePatterns '["generated/**"]'`


## `vpt json-edit .oxlintrc.json rules.no-debugger '"error"'`


## `vpt write-file index.js 'export const clean = true;
'`


## `vpt write-file packages/a/index.js 'export const clean = true;
'`


## `vpt write-file generated/built.js 'debugger;
'`


## `vpt write-file packages/a/generated/built.js 'debugger;
'`


## `vp lint --threads=1 -c .oxlintrc.json index.js generated/built.js`

The original owner excludes its generated file

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js generated/built.js`

The dependent does not inherit the owner's ignore patterns

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[generated/built.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
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
• 2 config updates applied
```

## `vpt print-file vite.config.mjs`

The owner's top-level ignore patterns reference its shared module

```
// vp migrate: merged .oxlintrc.json (4d2ecd8da8b74c19aab2ebbba9d98fe6).
import oxlintConfig1 from "./.oxlintrc.js";
export default {
  fmt: {},
  lint: {
    extends: [oxlintConfig1],
    ignorePatterns: oxlintConfig1.ignorePatterns,
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
};
```

## `vpt print-file packages/a/vite.config.mjs`

The dependent does not receive top-level ignore patterns

```
// vp migrate: merged packages%2Fa%2F.oxlintrc.json (b829c0806f4ea2e6444bd91df0061b20).
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

## `vp lint --threads=1 index.js generated/built.js`

The owner still excludes the generated file and exits successfully

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 index.js generated/built.js`

The dependent still reports the same diagnostic and exit code

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[generated/built.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt replace-file-content .oxlintrc.ts '"generated/**"' '"other/**"'`

Edit the shared ignore patterns once

```
```

## `vp lint --threads=1 index.js generated/built.js`

The owner follows the edit and now reports the generated file

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[generated/built.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 index.js generated/built.js`

The dependent's diagnostics remain unchanged

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[generated/built.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
