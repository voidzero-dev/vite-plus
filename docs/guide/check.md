# Check

`vp check` runs format, lint, and type checks together.

## Overview

`vp check` is the default command for fast static checks in Vite+. It brings together formatting through [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), linting through [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), and TypeScript type checks through [tsgolint](https://github.com/oxc-project/tsgolint). By merging all of these tasks into a single command, `vp check` is faster than running formatting, linting, and type checking as separate tools in separate commands.

When `typeCheck` is enabled in the `lint.options` block in `vite.config.ts`, `vp check` also runs TypeScript type checks through the Oxlint type-aware path powered by the TypeScript Go toolchain and [tsgolint](https://github.com/oxc-project/tsgolint). `vp create` and `vp migrate` enable both `typeAware` and `typeCheck` by default.

We recommend turning `typeCheck` on so `vp check` becomes the single command for static checks during development.

## Usage

```bash
vp check
vp check --fix # Format and run autofixers.
```

## Phase skip flags

`vp check` runs three phases — format, lint rules, and type check — all enabled by default. Each phase can be skipped independently:

| Flag | Skips |
| ---- | ----- |
| `--no-fmt` | Format check |
| `--no-lint` | Lint rules (type check still runs if enabled in config) |
| `--no-type-check` | Type check |

## Configuration

`vp check` uses the same configuration you already define for linting and formatting:

- [`lint`](/guide/lint#configuration) block in `vite.config.ts`
- [`fmt`](/guide/fmt#configuration) block in `vite.config.ts`
- TypeScript project structure and tsconfig files for type-aware linting

Recommended base `lint` config:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
});
```
