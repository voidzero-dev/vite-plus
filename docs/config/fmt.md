# Format Config

`vp fmt` and the formatting phase of `vp check` use Oxfmt's [native config discovery](/guide/fmt#configuration) from the working directory. Use `vp fmt -c <path>` or `vp fmt --config <path>` to select another config. See [Oxfmt's configuration](https://oxc.rs/docs/guide/usage/formatter/config.html) for details.

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
