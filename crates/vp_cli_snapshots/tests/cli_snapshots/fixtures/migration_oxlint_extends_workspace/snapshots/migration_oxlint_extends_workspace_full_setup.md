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

Prettier migration merges the root configs without migrating package configs

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
• 2 config updates applied
• Prettier migrated to Oxfmt
• Package manager settings configured
```

## `vpt print-file vite.config.mjs`

The root uses its shared module

```
// vp migrate: merged .oxlintrc.json (cca48660331feaa20cf74e4f147c8e10).
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
  fmt: {
    singleQuote: true,
    printWidth: 80,
    sortPackageJson: false,
    ignorePatterns: [],
  },
};
```

## `vpt stat-file .oxlintrc.json --assert file`

The root JSON remains available to untouched consumers

```
.oxlintrc.json: file
```

## `vpt stat-file .oxlintrc.ts --assert file`

```
.oxlintrc.ts: file
```

## `vpt print-file packages/a/vite.config.mjs`

The package Vite config remains unchanged

```
export default {};
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

The preserved package JSON still reports the same diagnostic

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
