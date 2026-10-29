# migration_oxlint_extends_workspace_full_setup

## `vp add -D vite-plus prettier@3.6.2`


## `vpt write-file .prettierrc.json '{"singleQuote":true}'`


## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

The existing workspace consumer inherits the root JSON rule

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --full --no-interactive --no-agent --no-editor --no-hooks`

Recover workspace configs and merge the migrated Prettier config

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● Prettier configuration detected. Auto-migrating to Oxfmt...
● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
✓ Dependencies installed in <duration>
• 3 config updates applied
• Prettier migrated to Oxfmt
• Package manager settings configured
! Warnings:
  - Could not migrate packages/b/.oxlintrc.json: Error: Cannot create shared Oxlint config because <workspace>/packages/b/extra.ts or <workspace>/packages/b/extra.js already exists. The original config was preserved; use vp lint -c packages/b/.oxlintrc.json until it is migrated manually.
```

## `vpt print-file vite.config.mjs`

The root uses its shared module

```
// vp migrate: merged .oxlintrc.json (cca48660331feaa20cf74e4f147c8e10).
import oxlintConfig1 from './.oxlintrc.js';
export default {
  fmt: {
    singleQuote: true,
    printWidth: 80,
    sortPackageJson: false,
    ignorePatterns: [],
  },
  lint: {
    extends: [oxlintConfig1],
    options: {},
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: {
      'vite-plus/prefer-vite-plus-imports': 'error',
    },
  },
};
```

## `vpt stat-file .oxlintrc.json --assert file`

The root JSON remains available to package b while its output conflict remains

```
.oxlintrc.json: file
```

## `vpt stat-file .oxlintrc.ts --assert file`

```
.oxlintrc.ts: file
```

## `vpt print-file packages/a/vite.config.mjs`

The recovered package uses the shared module

```
// vp migrate: merged packages%2Fa%2F.oxlintrc.json (e93b0fadd405db856ff1b8847eb5f2c3).
import oxlintConfig1 from './.oxlintrc.js';
export default {
  lint: {
    extends: [oxlintConfig1],
    options: {},
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: {
      'vite-plus/prefer-vite-plus-imports': 'error',
    },
  },
};
```

## `vpt stat-file packages/a/.oxlintrc.json --assert file`

```
packages/a/.oxlintrc.json: file
```

## `cd packages/a && vp lint --threads=1 index.js`

The recovered package reports the same diagnostic

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 index.js`

The migrated root enforces the same rule

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
