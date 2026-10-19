# migration_oxlint_extends_workspace_symlink_plugins

## `vpt rm packages/a/.oxlintrc.json`


## `vpt rm packages/b/.oxlintrc.json`


## `vpt write-file shared/lint.json '{"jsPlugins":[{"name":"fixture","specifier":"./plugin.mjs"}],"rules":{"fixture/require-setting":"error"},"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt cp settings-plugin.mjs packages/a/plugin.mjs`


## `vpt cp settings-plugin.mjs packages/b/plugin.mjs`


## `vpt write-file packages/a/index.js 'export const value = 1;
'`


## `vpt write-file packages/b/index.js 'export const value = 1;
'`


## `node create-config-links.mjs`


## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

Oxlint resolves the relative root plugin from the alias directory

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[index.js:1:1]
 1 │ export const value = 1;
   · ────────────────────────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/b && vp lint --threads=1 -c .oxlintrc.json index.js`

Each workspace uses its own relative plugin

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[index.js:1:1]
 1 │ export const value = 1;
   · ────────────────────────
   ╰────

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

The root plugin path stays relative to the workspace

```
// vp migrate: merged packages%2Fa%2F.oxlintrc.json (2e3968680d769859f24911af4896427f).
export default {
  lint: {
    jsPlugins: [
      {
        name: "fixture",
        specifier: "./plugin.mjs",
      },
      {
        name: "vite-plus",
        specifier: "vite-plus/oxlint-plugin",
      },
    ],
    rules: {
      "fixture/require-setting": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    options: {
      typeAware: false,
      typeCheck: false,
    },
  },
};
```

## `cd packages/a && vp lint --threads=1 index.js`

The first consumer keeps the original plugin diagnostic

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[index.js:1:1]
 1 │ export const value = 1;
   · ────────────────────────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/b && vp lint --threads=1 index.js`

The second consumer keeps the original plugin diagnostic

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[index.js:1:1]
 1 │ export const value = 1;
   · ────────────────────────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
