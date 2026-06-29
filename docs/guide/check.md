# Check

`vp check` runs format, lint, and type checks together.

## Overview

`vp check` is the default command for fast static checks in Vite+. It brings together formatting through [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), linting through [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), and TypeScript type checks through [tsgolint](https://github.com/oxc-project/tsgolint). By merging all of these tasks into a single command, `vp check` is faster than running formatting, linting, and type checking as separate tools in separate commands.

When `typeCheck` is enabled in the `lint.options` block in `vite.config.ts`, `vp check` also runs TypeScript type checks through the Oxlint type-aware path powered by the TypeScript Go toolchain and [tsgolint](https://github.com/oxc-project/tsgolint). `vp create` and `vp migrate` enable both `typeAware` and `typeCheck` by default.

We recommend turning `typeCheck` on so `vp check` becomes the single command for static checks during development.

## Usage

```bash
vp check
vp check --fix             # Format and run autofixers.
vp check --no-fmt          # Skip format; run lint (and type-check if enabled).
vp check --no-lint         # Skip lint rules; keep type-check when enabled.
vp check --no-fmt --no-lint # Type-check only (requires `typeCheck` enabled).
```

## Configuration

`vp check` uses the same configuration you already define for linting and formatting:

- [`lint`](/guide/lint#configuration) block in `vite.config.ts`
- [`fmt`](/guide/fmt#configuration) block in `vite.config.ts`
- TypeScript project structure and tsconfig files for type-aware linting

Recommended base `lint` config:

```ts [vite.config.ts]
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

### Disabling a step by default

To make `vp check` skip formatting or linting without passing a flag every time, set the [`check`](/config/check) block in `vite.config.ts`. This is handy when a project wants the rest of the toolchain but not, say, formatting:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  check: {
    fmt: false, // `vp check` lints (and type-checks) but does not format
  },
});
```

These options only affect `vp check`; standalone `vp fmt` and `vp lint` still run normally. A step is skipped if it is disabled in config or the matching `--no-fmt` / `--no-lint` flag is passed.
