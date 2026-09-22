# Format Config

`vp fmt` and `vp check` use the workspace-root `fmt` block, including when run from a package directory. Package configs do not replace these format settings. Use `vp fmt -c <path>` or `vp fmt --config <path>` to select another config. If the root config has no `fmt` block, Oxfmt uses [native discovery](/guide/fmt#configuration). See [Oxfmt's configuration](https://oxc.rs/docs/guide/usage/formatter/config.html) for details.

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

Oxfmt disables nested configs in Vite+ mode, so nested format configs do not override settings for individual files. See [troubleshooting](/guide/troubleshooting#nested-lint-or-format-config-is-not-applied) for details.
