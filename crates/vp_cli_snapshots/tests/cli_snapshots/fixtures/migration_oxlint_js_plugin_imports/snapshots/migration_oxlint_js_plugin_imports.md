# migration_oxlint_js_plugin_imports

## `vp migrate --no-interactive`

the standalone oxlint dependency goes away, so the JS plugin's authoring imports must move to vite-plus

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied, 3 files had imports rewritten
```

## `vpt print-file package.json`

oxlint is removed and nothing replaces it. The API now comes from vite-plus. @oxlint/plugins stays on purpose: it is inert once the imports are rewritten, and removing it would also remove the peer dependency of a published Oxlint plugin

```
{
  "name": "migration-oxlint-js-plugin-imports",
  "scripts": {
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@oxlint/plugins": "^1.0.0",
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

## `vpt print-file lint/no-foo.js`

legacy `defineRule` from 'oxlint' -> 'vite-plus/lint/plugins'

```
import { defineRule } from 'vite-plus/lint/plugins';

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

## `vpt print-file lint/plugin.js`

'@oxlint/plugins' -> 'vite-plus/lint/plugins'

```
import { definePlugin } from 'vite-plus/lint/plugins';

import { noFoo } from './no-foo.js';

export default definePlugin({
  meta: { name: 'local' },
  rules: { 'no-foo': noFoo },
});
```

## `vpt print-file lint/no-foo.test.ts`

RuleTester lives in 'oxlint/plugins-dev' upstream and breaks the same way, so it maps to 'vite-plus/lint/plugins-dev'. The plugin type import follows the runtime API

```
import type { Context } from 'vite-plus/lint/plugins';
import { RuleTester } from 'vite-plus/lint/plugins-dev';

import { noFoo } from './no-foo.js';

export type RuleContext = Context;

new RuleTester().run('no-foo', noFoo, {
  valid: ['const bar = 1;'],
  invalid: [{ code: 'const foo = 1;', errors: 1 }],
});
```

## `vpt print-file lint/shared-config.ts`

the config surface is NOT redirected. vite-plus/lint/plugins has no defineConfig or OxlintOverride

```
import { defineConfig } from 'oxlint';
import type { OxlintOverride } from 'oxlint';

export const testOverride: OxlintOverride = {
  files: ['**/*.test.ts'],
  rules: { 'local/no-foo': 'off' },
};

export default defineConfig({ overrides: [testOverride] });
```

## `vpt print-file vite.config.ts`

the jsPlugins entry survives the .oxlintrc.json merge. It still points at the plugin file, which is now rewritten. KNOWN PRE-EXISTING GAP, unrelated to the import rewrite: the merge drops `local/no-foo`. sanitizeMigratedOxlintConfig derives a plugin's rule namespace from its package name, and a relative-path plugin has no package name. Its namespace comes from `meta.name` at load time instead. Recorded here so a fix shows up as a snapshot diff

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {
    "jsPlugins": [
      "./lint/plugin.js",
      {
        "name": "vite-plus",
        "specifier": "vite-plus/oxlint-plugin"
      }
    ],
    "rules": {
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "options": {
      "typeAware": true,
      "typeCheck": true
    }
  },
});
```
