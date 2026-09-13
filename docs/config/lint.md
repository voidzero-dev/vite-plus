# Lint Config

`vp lint` and `vp check` read Oxlint settings from the `lint` block in the root `vite.config.ts`. See [Oxlint's configuration](https://oxc.rs/docs/guide/usage/linter/config.html) for details.

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

For file- or package-specific lint rules, use [`lint.overrides`](/guide/monorepo#root-config-with-overrides) from the root `vite.config.ts`.

Vite+ does not currently support nested lint configuration. See [troubleshooting](/guide/troubleshooting#nested-lint-or-format-config-is-not-applied) for details and how to give feedback on future support.
