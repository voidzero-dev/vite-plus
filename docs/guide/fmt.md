# Format

`vp fmt` formats code with Oxfmt.

## Overview

`vp fmt` is built on [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), the Oxc formatter. Oxfmt has full Prettier compatibility and is designed as a fast drop-in replacement for Prettier.

Use `vp fmt` to format your project, and `vp check` to format, lint and type-check all at once.

## Usage

```bash
vp fmt
vp fmt --check
vp fmt . --write
```

## Configuration

Put formatting configuration directly in the `fmt` block in the root `vite.config.ts` so all your configuration stays in one place. We do not recommend using `.oxfmtrc.json` with Vite+.

`vp fmt` lets Oxfmt discover configuration from the working directory, including when run from a workspace package. Relative file arguments retain their meaning. Use [`fmt.overrides`](/guide/monorepo#format-overrides) for file- or package-specific options.

`vp check` uses the workspace-root `fmt` block when it exists, including from a package directory. Package configs cannot replace those format settings in `vp check`.

An explicit `vp fmt -c <path>` or `vp fmt --config <path>` selects another config. Otherwise, Oxfmt discovers the nearest `vite.config.*` file with a `fmt` block. Supported extensions are `.js`, `.mjs`, `.ts`, `.cjs`, `.mts`, and `.cts`. Nested configs do not override settings for individual files.

For editors, disable nested formatter configs to prevent per-file overrides:

```json [.vscode/settings.json]
{
  "oxc.fmt.disableNestedConfig": true
}
```

For the upstream formatter behavior and configuration reference, see the [Oxfmt docs](https://oxc.rs/docs/guide/usage/formatter.html).

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    singleQuote: true,
  },
});
```
