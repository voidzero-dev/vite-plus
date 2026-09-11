# Format Config

`vp fmt` and `vp check` read Oxfmt settings from the `fmt` block in the root `vite.config.ts`. See [Oxfmt's configuration](https://oxc.rs/docs/guide/usage/formatter/config.html) for details.

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    ignorePatterns: ['dist/**'],
    singleQuote: true,
    semi: true,
    sortPackageJson: true,
  },
});
```

For file- or package-specific formatting settings, use [`fmt.overrides`](/guide/monorepo#format-overrides) from the root `vite.config.ts`.

Vite+ does not currently support nested format configuration. See [troubleshooting](/guide/troubleshooting#nested-lint-or-format-config-is-not-applied) for details and how to give feedback on future support.
