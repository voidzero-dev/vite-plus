# migration_oxlint_extends_workspace_symlink_contexts

## `vpt rm packages/a/.oxlintrc.json`


## `vpt rm packages/b/.oxlintrc.json`


## `vpt write-file shared/lint.json '{"extends":["./base.json"],"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt write-file shared/base.json '{"rules":{"no-console":"off"}}'`


## `vpt write-file packages/a/base.json '{"rules":{"no-console":"error"}}'`


## `vpt write-file packages/b/base.json '{"rules":{"no-console":"warn"}}'`


## `vpt write-file packages/a/index.js 'console.log("hello");'`


## `vpt write-file packages/b/index.js 'console.log("hello");'`


## `node create-config-links.mjs`


## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

The first alias resolves its local base to an error

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/b && vp lint --threads=1 -c .oxlintrc.json index.js`

The second alias resolves the same extends string to a warning

```

  ⚠ eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
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

## `vpt print-file packages/a/vite.config.mjs`

```
// vp migrate: merged packages%2Fa%2F.oxlintrc.json (5aa76017b601250136404623f0a0a3dd).
import oxlintConfig1 from "./base.js";
export default {
  lint: {
    extends: [oxlintConfig1],
    options: {
      typeAware: false,
      typeCheck: false,
    },
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
};
```

## `vpt print-file packages/b/vite.config.mjs`

```
// vp migrate: merged packages%2Fb%2F.oxlintrc.json (56118339eefbe7bfe3d4f1a40696a3e9).
import oxlintConfig1 from "./base.js";
export default {
  lint: {
    extends: [oxlintConfig1],
    options: {
      typeAware: false,
      typeCheck: false,
    },
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
};
```

## `cd packages/a && vp lint --threads=1 index.js`

The migrated first consumer retains its error

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/b && vp lint --threads=1 index.js`

The migrated second consumer retains its warning

```

  ⚠ eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
