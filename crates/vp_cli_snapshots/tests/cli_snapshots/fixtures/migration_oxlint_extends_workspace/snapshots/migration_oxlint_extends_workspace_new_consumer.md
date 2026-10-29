# migration_oxlint_extends_workspace_new_consumer

## `vpt rm packages/b/extra.ts`


## `vpt write-file shared/base.json '{"rules":{"no-console":"error"},"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt json-edit .oxlintrc.json extends '["./shared/base.json"]'`


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

## `vpt stat-file shared/base.ts --assert file`

```
shared/base.ts: file
```

## `vpt write-file packages/c/package.json '{"name":"new-consumer","type":"module"}'`


## `vpt write-file packages/c/vite.config.mjs 'export default {};
'`


## `vpt write-file packages/c/.oxlintrc.json '{"extends":["../../shared/base.json"]}'`


## `vpt write-file packages/c/index.js 'console.log("hello");'`


## `cd packages/c && vp lint --threads=1 -c .oxlintrc.json index.js`

A new workspace uses the retained shared JSON

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

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

Recover the unreferenced workspace config even without a pending marker

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

## `vpt print-file packages/c/vite.config.mjs`

The workspace imports the existing shared module

```
// vp migrate: merged packages%2Fc%2F.oxlintrc.json (f042a7ee2bd1c045ffb01c004fc0106b).
import oxlintConfig1 from "../../shared/base.js";
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

## `vpt stat-file packages/c/.oxlintrc.json --assert missing`

The recovered config no longer needs its standalone JSON

```
packages/c/.oxlintrc.json: missing
```

## `cd packages/c && vp lint --threads=1 index.js`

Plain lint retains the same diagnostic and exit status

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
