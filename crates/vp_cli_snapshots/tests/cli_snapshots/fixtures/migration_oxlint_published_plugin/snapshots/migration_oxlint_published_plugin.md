# migration_oxlint_published_plugin

## `vp migrate --no-interactive`

this package declares `oxlint` as a peer dependency, which marks it a published Oxlint plugin

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file lint/index.js`

the authoring import stays on 'oxlint'. Consumers of a published plugin may run plain Oxlint, so a rewrite to vite-plus would break them. This also covers the ordering trap: rewritePackageJson strips `oxlint` before the import rewriter reads the manifest, so the skip signal is captured up front

```
import { defineRule } from 'oxlint';

export const noFoo = defineRule({
  meta: { messages: { noFoo: 'Do not name things "foo".' } },
  create(context) {
    return {
      Identifier(node) {
        if (node.name === 'foo') {
          context.report({ node, messageId: 'noFoo' });
        }
      },
    };
  },
});
```

## `vpt print-file package.json`

the `oxlint` peer entry survives. It is a consumer contract, not a tool this package runs, and stripping it would leave the source importing a package the manifest no longer declares

```
{
  "name": "oxlint-plugin-example",
  "version": "1.0.0",
  "scripts": {
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "peerDependencies": {
    "oxlint": "^1.0.0"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```
