# Test

`vp test` runs tests with [Vitest](https://vitest.dev).

## Overview

`vp test` is built on [Vitest](https://vitest.dev/), so you get a Vite-native test runner that reuses your Vite config and plugins, supports Jest-style expectations, snapshots, and coverage, and handles modern ESM, TypeScript, and JSX projects cleanly.

::: info
`vp test` always runs the built-in Vitest command. If your project also has a `test` script in `package.json`, run `vp run test` when you want to run that script instead. See [Built-in Commands vs Scripts](/guide/run#built-in-commands-vs-scripts).
:::

## Usage

```bash
vp test
vp test watch
vp test run --coverage
```

::: info
Unlike Vitest on its own, `vp test` does not stay in watch mode by default. Use `vp test` when you want a normal test run, and use `vp test watch` when you want to jump into watch mode.
:::

## Configuration

Put test configuration directly in the `test` block in `vite.config.ts` so all your configuration stays in one place. We do not recommend using `vitest.config.ts` with Vite+.

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
  },
});
```

For the full Vitest configuration reference, see the [Vitest config docs](https://vitest.dev/config/).
