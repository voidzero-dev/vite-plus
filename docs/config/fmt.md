# Format Config

`vp fmt` uses Oxfmt's [native config discovery](/guide/fmt#configuration) from the working directory. Use `vp fmt -c <path>` or `vp fmt --config <path>` to select another config. See [Oxfmt's configuration](https://oxc.rs/docs/guide/usage/formatter/config.html) for details.

`vp check` uses the workspace-root `fmt` block when it exists, including from a package directory. Package configs do not replace these format settings in `vp check`.

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
