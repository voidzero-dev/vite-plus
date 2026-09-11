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

Vite+ does not currently support nested format configuration. For now, use [`fmt.overrides`](/guide/monorepo#format-overrides) in the root `vite.config.ts` for file- or package-specific options. The long-term behavior is open for discussion; [share your use case and expectations](/guide/troubleshooting#nested-lint-or-format-config-is-not-applied) to help shape it.

For editors, disable nested formatter configs so format-on-save uses the root Vite+ `fmt` block:

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
