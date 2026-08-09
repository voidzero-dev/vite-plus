# Lint

`vp lint` lints code with Oxlint.

## Overview

`vp lint` is built on [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), the Oxc linter. Oxlint is designed as a fast replacement for ESLint for most frontend projects and ships with built-in support for core ESLint rules and many popular community rules.

Use `vp lint` to lint your project, and `vp check` to format, lint and type-check all at once.

## Usage

```bash
vp lint
vp lint --fix
vp lint --type-aware
```

## Configuration

Put lint configuration directly in the `lint` block in `vite.config.ts` so all your configuration stays in one place. We do not recommend using `oxlint.config.ts` or `.oxlintrc.json` with Vite+.

For the upstream rule set, options, and compatibility details, see the [Oxlint docs](https://oxc.rs/docs/guide/usage/linter.html).

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    ignorePatterns: ['dist/**'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
});
```

## Type-Aware Linting

We recommend enabling both `typeAware` and `typeCheck` in the `lint` block:

- `typeAware: true` enables rules that require TypeScript type information
- `typeCheck: true` enables full type checking during linting

This path is powered by [tsgolint](https://github.com/oxc-project/tsgolint) on top of the TypeScript 7 (aka TypeScript Go) toolchain. It gives Oxlint access to type information and allows type checking directly via `vp lint` and `vp check`.

## JS Plugins

If you are migrating from ESLint and still depend on a few critical JavaScript-based ESLint plugins, Oxlint has [JS plugin support](https://oxc.rs/docs/guide/usage/linter/js-plugins) that can help you keep those plugins running while you complete the migration.

JS Plugins also enable [writing your own custom rules](https://oxc.rs/docs/guide/usage/linter/writing-js-plugins.html) for Oxlint.

### Writing Your Own Rules

Import the plugin authoring API from `vite-plus/lint/plugins`:

```js [lint/my-plugin.js]
import { definePlugin, defineRule } from 'vite-plus/lint/plugins';

const noFoo = defineRule({
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

export default definePlugin({
  meta: { name: 'my' },
  rules: { 'no-foo': noFoo },
});
```

Register it under `lint.jsPlugins` and enable its rules:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    jsPlugins: ['./lint/my-plugin.js'],
    rules: {
      'my/no-foo': 'error',
    },
  },
});
```

For rule tests, `RuleTester` is available from `vite-plus/lint/plugins-dev`.

Both entrypoints re-export the copy that ships with Vite+. The API therefore
always matches the bundled Oxlint.

Use them instead of adding `@oxlint/plugins` or `oxlint` as a direct
dependency. A separately pinned copy can drift from the linter that loads your
plugin. It also does not resolve from a plugin file under pnpm's strict layout,
unless every package that holds a plugin declares it.

`vp migrate` rewrites existing `oxlint` and `@oxlint/plugins` imports for you.
See [Oxlint JS Plugin Imports](/guide/migrate-rules#oxlint-js-plugin-imports).
The `vite-plus/prefer-vite-plus-imports` rule reports any that come back.
