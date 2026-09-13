# migration_oxlint_optional_plugin

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vp lint --fix plugin.js`

Lint autofix must preserve the optional runtime API import.

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt print-file package.json`

```
{
  "name": "oxlint-plugin-optional-example",
  "version": "1.0.0",
  "files": [
    "plugin.js"
  ],
  "type": "module",
  "exports": "./plugin.js",
  "scripts": {
    "lint": "vp lint ."
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "optionalDependencies": {
    "@oxlint/plugins": "1.79.0"
  },
  "packageManager": "pnpm@11.24.0"
}
```

## `vpt print-file plugin.js`

```
import { definePlugin, defineRule } from "@oxlint/plugins";

export default definePlugin({
  meta: { name: "optional" },
  rules: {
    "no-foo": defineRule({
      meta: { messages: { noFoo: 'Do not name things "foo".' } },
      create(context) {
        return {
          Identifier(node) {
            if (node.name === "foo") {
              context.report({ node, messageId: "noFoo" });
            }
          },
        };
      },
    }),
  },
});
```

## `vp pm pack --pack-destination artifacts`


## `cd consumer && pnpm install --ignore-workspace --ignore-scripts`


## `cd consumer && node check.mjs`

Install the packed plugin as a consumer dependency, with no vite-plus dependency.

```
The published plugin loads with its optional API and without vite-plus.
```
