# migration_oxlint_extends_workspace_symlinks

## `vpt rm packages/a/.oxlintrc.json`


## `vpt rm packages/b/.oxlintrc.json`


## `vpt json-edit .oxlintrc.json rules.no-console '"off"'`


## `vpt write-file shared/lint.json '{"rules":{"no-console":"error"},"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt write-file packages/a/index.js 'console.log("hello");'`


## `vpt write-file packages/b/index.js 'console.log("hello");'`


## `node create-config-links.mjs`


## `vpt stat-file packages/a/.oxlintrc.json --assert symlink`

```
packages/a/.oxlintrc.json: symlink
```

## `vpt stat-file packages/b/.oxlintrc.json --assert symlink`

```
packages/b/.oxlintrc.json: symlink
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

The first symlinked consumer reports no-console before migration

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

The second symlinked consumer reports the same error before migration

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

Each consumer receives its own lint block

```
// vp migrate: merged packages%2Fa%2F.oxlintrc.json (c8b042efd6403025ed6a8239c29b3114).
export default {
  lint: {
    rules: {
      "no-console": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    options: {
      typeAware: false,
      typeCheck: false,
    },
    jsPlugins: [
      {
        name: "vite-plus",
        specifier: "vite-plus/oxlint-plugin",
      },
    ],
  },
};
```

## `vpt print-file packages/b/vite.config.mjs`

```
// vp migrate: merged packages%2Fb%2F.oxlintrc.json (b9baaacbd5e6b450f2ea1acce04edac9).
export default {
  lint: {
    rules: {
      "no-console": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    options: {
      typeAware: false,
      typeCheck: false,
    },
    jsPlugins: [
      {
        name: "vite-plus",
        specifier: "vite-plus/oxlint-plugin",
      },
    ],
  },
};
```

## `vpt stat-file packages/a/.oxlintrc.json --assert missing`

```
packages/a/.oxlintrc.json: missing
```

## `vpt stat-file packages/b/.oxlintrc.json --assert missing`

```
packages/b/.oxlintrc.json: missing
```

## `vpt print-file shared/lint.json`

Removing workspace symlinks preserves their shared JSON target

```
{ "rules": { "no-console": "error" }, "options": { "typeAware": false, "typeCheck": false } }
```

## `cd packages/a && vp lint --threads=1 index.js`

The first migrated consumer keeps the same diagnostic and exit code

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

The second migrated consumer keeps the same diagnostic and exit code

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
