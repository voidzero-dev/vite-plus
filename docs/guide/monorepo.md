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

The root `vite.config.ts` is most valuable for shared linting, formatting, staged checks, and task definitions. Development, build, preview, and packaging still act on a single app, so Vite+ makes the built-in commands monorepo-aware instead of forcing you to `cd` between packages.

### Running at the workspace root

`vp dev`, `vp build`, `vp preview`, and `vp pack` never silently act on the workspace root, which usually has no app of its own. Run them at the top of the monorepo and Vite+ works out which app you mean.

When exactly one package looks like an app, vp runs it and shows you the direct command for next time:

```
$ vp dev
Selected package: web (apps/web)
Tip: run this directly with `vp -C apps/web dev`

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/
  ➜  Network: use --host to expose
```

When several packages could be the target, vp lists them with ready-to-copy commands instead of guessing:

```
$ vp build
error: `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    admin     apps/admin
    web       apps/web
    @shop/ui  packages/ui

  Pass a directory:  vp -C apps/admin build
  Or run every package's build script:  vp run -r build
```

Packages that look runnable for the command (an `index.html` or `vite.config.*` for `dev` / `build` / `preview`, a library entry for `pack`) are listed first.

### Targeting a package with `-C`

The global `-C` flag runs any command as if you had `cd`'d into a package. It works with every vp command and is identical to `cd <dir> && vp <command>`:

```bash
vp -C apps/web dev
vp -C apps/web build
vp -C packages/ui pack
```

Passing a folder as a positional (`vp dev apps/web`) still works, but keeps upstream Vite semantics: it sets Vite's `root` option without changing the working directory, so `process.cwd()` reads in configs and plugins resolve from where you ran vp. Prefer `-C` when the package should behave as if you had `cd`'d into it.

### A fixed default with `defaultPackage`

To always target one directory and skip the resolution above, set [`defaultPackage`](/config/#defaultpackage) in the root config:

```ts [vite.config.ts]
export default {
  defaultPackage: './apps/web',
};
```

```
$ vp dev
note: vp dev: using ./apps/web (defaultPackage)

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/
```

This is the right choice for framework monorepos that are not JavaScript workspaces, such as a Laravel or Rails app with a `frontend/` directory: there is no package list to resolve, so `defaultPackage` points vp straight at the app. Because vp reads it without executing the config, it works even when `vite-plus` is installed only inside that subdirectory.

### Package scripts and workspace-wide tasks

Keep package-specific scripts in each package when the command differs per app:

```json [apps/api/package.json]
{
  "scripts": {
    "dev": "tsx watch src/index.ts",
    "build": "tsc -p tsconfig.json"
  }
}
```

Run scripts across the whole workspace with `vp run`:

```bash
vp run -r build
vp run -r --parallel dev
vp run --filter ./apps/web build
```

See the [Run guide](/guide/run) for recursive, parallel, filtered, and cached workspace tasks.
