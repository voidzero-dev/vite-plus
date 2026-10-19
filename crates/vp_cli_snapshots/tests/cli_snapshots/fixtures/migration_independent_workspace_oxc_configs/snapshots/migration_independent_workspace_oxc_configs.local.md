# migration_independent_workspace_oxc_configs

A reference to the root config must not prevent merging an independent package config with the same filename.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
• 1 config update applied
```

## `vpt print-file .oxlintrc.json`

```
{
  "rules": { "no-debugger": "error" },
  "options": { "typeAware": false, "typeCheck": false }
}
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "private": true,
  "type": "module",
  "scripts": {
    "lint": "vp lint index.js"
  },
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  }
}
```

## `vpt print-file packages/app/vite.config.mjs`

```
export default {
  lint: {
    "rules": {
      "no-console": "error",
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "options": {
      "typeAware": false,
      "typeCheck": false
    },
    "jsPlugins": [
      {
        "name": "vite-plus",
        "specifier": "vite-plus/oxlint-plugin"
      }
    ]
  },

}
```

## `vpt stat-file packages/app/.oxlintrc.json --assert missing`

```
packages/app/.oxlintrc.json: missing
```

## `vp lint -c .oxlintrc.json root.js`

```
note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/app && vp lint index.js`

plain lint must report no-console as an error

**Exit code:** 1

```
note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("test");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file .oxlintrc.json`

```
{
  "rules": { "no-debugger": "error" },
  "options": { "typeAware": false, "typeCheck": false }
}
```

## `vpt print-file packages/app/vite.config.mjs`

```
export default {
  lint: {
    "rules": {
      "no-console": "error",
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "options": {
      "typeAware": false,
      "typeCheck": false
    },
    "jsPlugins": [
      {
        "name": "vite-plus",
        "specifier": "vite-plus/oxlint-plugin"
      }
    ]
  },

}
```

## `cd packages/app && vp lint index.js`

a second migration must preserve the error and nonzero exit status

**Exit code:** 1

```
note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("test");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
