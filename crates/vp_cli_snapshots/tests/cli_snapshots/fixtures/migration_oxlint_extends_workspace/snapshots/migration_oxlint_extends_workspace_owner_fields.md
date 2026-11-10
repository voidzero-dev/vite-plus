# migration_oxlint_extends_workspace_owner_fields

## `vpt rm packages/b/extra.ts`


## `vpt json-edit .oxlintrc.json env '{"browser":true}'`


## `vpt json-edit .oxlintrc.json globals '{"fixtureGlobal":"readonly"}'`


## `vpt json-edit .oxlintrc.json settings '{"fixture":{"enabled":true}}'`


## `vpt json-edit .oxlintrc.json jsPlugins '["./settings-plugin.mjs"]'`


## `vpt json-edit .oxlintrc.json rules '{"no-undef":"error"}'`


## `vpt json-edit .oxlintrc.json overrides '[{"files":["settings.js"],"rules":{"fixture/require-setting":"error"}}]'`


## `vpt write-file env.js 'export const value = window;
'`


## `vpt write-file globals.js 'export const value = fixtureGlobal;
'`


## `vpt write-file settings.js 'export const value = 1;
'`


## `vpt cp env.js packages/a/env.js`


## `vpt cp globals.js packages/a/globals.js`


## `vpt cp settings.js packages/a/settings.js`


## `vp lint --threads=1 -c .oxlintrc.json env.js`

The original owner uses its environment, globals, and plugin settings

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 -c .oxlintrc.json globals.js`

The original owner uses its environment, globals, and plugin settings

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 -c .oxlintrc.json settings.js`

The original owner uses its environment, globals, and plugin settings

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json env.js`

A dependent does not inherit the owner's local fields

**Exit code:** 1

```

  × eslint(no-undef): 'window' is not defined.
   ╭─[env.js:1:22]
 1 │ export const value = window;
   ·                      ──────
   ╰────
  help: Either define 'window' or remove the reference to it. If 'window' is a global variable, add it to the 'globals' configuration.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json globals.js`

A dependent does not inherit the owner's local fields

**Exit code:** 1

```

  × eslint(no-undef): 'fixtureGlobal' is not defined.
   ╭─[globals.js:1:22]
 1 │ export const value = fixtureGlobal;
   ·                      ─────────────
   ╰────
  help: Either define 'fixtureGlobal' or remove the reference to it. If 'fixtureGlobal' is a global variable, add it to the 'globals' configuration.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json settings.js`

A dependent does not inherit the owner's local fields

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[settings.js:1:1]
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

## `vpt print-file vite.config.mjs`

The owner references its fields at the top level

```
// vp migrate: merged .oxlintrc.json (776c7037d8d471e35fd2209aa88a3a33).
import oxlintConfig1 from "./.oxlintrc.js";
export default {
  fmt: {},
  lint: {
    extends: [oxlintConfig1],
    env: oxlintConfig1.env,
    globals: oxlintConfig1.globals,
    settings: oxlintConfig1.settings,
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
};
```

## `vp lint --threads=1 env.js`

The owner retains its exclusions from no-undef and the plugin diagnostic

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 globals.js`

The owner retains its exclusions from no-undef and the plugin diagnostic

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 settings.js`

The owner retains its exclusions from no-undef and the plugin diagnostic

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 env.js`

The dependent retains the same diagnostics and exit code

**Exit code:** 1

```

  × eslint(no-undef): 'window' is not defined.
   ╭─[env.js:1:22]
 1 │ export const value = window;
   ·                      ──────
   ╰────
  help: Either define 'window' or remove the reference to it. If 'window' is a global variable, add it to the 'globals' configuration.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 globals.js`

The dependent retains the same diagnostics and exit code

**Exit code:** 1

```

  × eslint(no-undef): 'fixtureGlobal' is not defined.
   ╭─[globals.js:1:22]
 1 │ export const value = fixtureGlobal;
   ·                      ─────────────
   ╰────
  help: Either define 'fixtureGlobal' or remove the reference to it. If 'fixtureGlobal' is a global variable, add it to the 'globals' configuration.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 settings.js`

The dependent retains the same diagnostics and exit code

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[settings.js:1:1]
 1 │ export const value = 1;
   · ────────────────────────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt replace-file-content .oxlintrc.ts 'browser: true' 'browser: false'`

```
```

## `vpt replace-file-content .oxlintrc.ts 'fixtureGlobal: "readonly"' 'fixtureGlobal: "off"'`

```
```

## `vpt replace-file-content .oxlintrc.ts 'enabled: true' 'enabled: false'`

```
```

## `vp lint --threads=1 env.js`

Shared-module edits update all three owner fields

**Exit code:** 1

```

  × eslint(no-undef): 'window' is not defined.
   ╭─[env.js:1:22]
 1 │ export const value = window;
   ·                      ──────
   ╰────
  help: Either define 'window' or remove the reference to it. If 'window' is a global variable, add it to the 'globals' configuration.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 globals.js`

Shared-module edits update all three owner fields

**Exit code:** 1

```

  × eslint(no-undef): 'fixtureGlobal' is not defined.
   ╭─[globals.js:1:22]
 1 │ export const value = fixtureGlobal;
   ·                      ─────────────
   ╰────
  help: Either define 'fixtureGlobal' or remove the reference to it. If 'fixtureGlobal' is a global variable, add it to the 'globals' configuration.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 settings.js`

Shared-module edits update all three owner fields

**Exit code:** 1

```

  × fixture(require-setting): The fixture setting must be enabled.
   ╭─[settings.js:1:1]
 1 │ export const value = 1;
   · ────────────────────────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
