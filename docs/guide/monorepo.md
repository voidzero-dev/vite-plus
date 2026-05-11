# Monorepo

Vite+ supports monorepos with `vite.config.ts` at the root. You can define the defaults for `lint`, `fmt`, etc. at the root, and use `overrides` to apply package-specific lint and format settings.

Because `vite.config.ts` is just JavaScript, you can choose to put your entire config into this file or compose it using regular JavaScript imports. You can still have separate `vite.config.ts` files in each package for the Vite, Vitest, framework or runtime configuration.

## Root Config With Overrides

Use `lint.overrides` for Oxlint rules that only apply to some packages:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    plugins: ['typescript'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
    rules: {
      'no-console': ['error', { allow: ['warn', 'error'] }],
    },
    overrides: [
      {
        files: ['apps/web/**', 'packages/ui/**'],
        plugins: ['typescript', 'react'],
        rules: {
          'react/self-closing-comp': 'error',
        },
      },
      {
        files: ['apps/api/**'],
        env: {
          node: true,
        },
        rules: {
          'no-console': 'off',
        },
      },
      {
        files: ['**/*.test.ts', '**/*.spec.ts'],
        plugins: ['typescript', 'vitest'],
        rules: {
          '@typescript-eslint/no-explicit-any': 'off',
          'vitest/no-disabled-tests': 'error',
        },
      },
    ],
  },
});
```

Globs are resolved from the root `vite.config.ts`, so use workspace paths such as `apps/web/**`, `apps/api/**`, and `packages/ui/**`.

::: tip
When a `lint.overrides` entry sets `plugins`, that list replaces the base `lint.plugins` list for matched files. Include every plugin needed by that file group, such as `['typescript', 'react']`. Omit `plugins` only when the override should inherit the base list unchanged.
:::

## Format Overrides

Use `fmt.overrides` for file or package-specific Oxfmt options. Formatter overrides put their settings under `options`:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    singleQuote: true,
    semi: true,
    overrides: [
      {
        files: ['apps/api/**'],
        options: {
          printWidth: 120,
        },
      },
      {
        files: ['**/*.md'],
        options: {
          proseWrap: 'always',
        },
      },
    ],
  },
});
```

## Composing Configuration Files

You can split configuration across your repository and compose them using JavaScript imports. Export JavaScript objects from nearby files or packages, import them in the root config, and merge them into the matching override.

```ts [tooling/lint/react.ts]
import type { OxlintOverride } from 'vite-plus/lint';

export const reactLint = {
  plugins: ['typescript', 'react'],
  rules: {
    'react/self-closing-comp': 'error',
  },
} satisfies Omit<OxlintOverride, 'files'>;
```

```ts [tooling/lint/node.ts]
import type { OxlintOverride } from 'vite-plus/lint';

export const nodeLint = {
  env: {
    node: true,
  },
  rules: {
    'no-console': 'off',
  },
} satisfies Omit<OxlintOverride, 'files'>;
```

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

import { nodeLint } from './tooling/lint/node';
import { reactLint } from './tooling/lint/react';

export default defineConfig({
  lint: {
    plugins: ['typescript'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
    overrides: [
      {
        files: ['apps/web/**', 'packages/ui/**'],
        ...reactLint,
      },
      {
        files: ['apps/api/**'],
        ...nodeLint,
      },
    ],
  },
});
```

This keeps the behavior centralized while letting each team or package own the pieces of config it needs.

## App Commands

The root `vite.config.ts` is most valuable for shared linting, formatting, staged checks, and task definitions. For project-specific development, build, and test behavior, use the setup that best matches each app:

- Pass a folder to built-in Vite commands when you want to target one app:

```bash
vp dev apps/web
vp build apps/web
```

- Keep package-specific scripts in each package when the command differs per app:

```json [apps/api/package.json]
{
  "scripts": {
    "dev": "tsx watch src/index.ts",
    "build": "tsc -p tsconfig.json"
  }
}
```

- Run scripts across the workspace with `vp run`:

```bash
vp run -r build
vp run -r --parallel dev
vp run --filter ./apps/web build
```

See the [Run guide](/guide/run) for recursive, parallel, filtered, and cached workspace tasks.
