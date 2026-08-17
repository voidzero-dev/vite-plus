# Lint Config

`vp lint` and `vp check` read Oxlint settings from the `lint` block in `vite.config.ts`. See [Oxlint's configuration](https://oxc.rs/docs/guide/usage/linter/config.html) for details.

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    ignorePatterns: ['dist/**'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
    rules: {
      'no-console': ['error', { allow: ['error'] }],
    },
  },
});
```

We recommend enabling both `options.typeAware` and `options.typeCheck` so `vp lint` and `vp check` can use the full type-aware path.

For package-specific lint rules in a workspace, use [`lint.overrides`](/guide/monorepo#root-config-with-overrides) from the root `vite.config.ts`.
