# Format Config

`vp fmt` and `vp check` read Oxfmt settings from the `fmt` block in your Vite config. Oxfmt [discovers the config](/guide/fmt#configuration) from the working directory and its parents. Keep shared settings in the workspace-root config. See [Oxfmt's configuration](https://oxc.rs/docs/guide/usage/formatter/config.html) for details.

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

Vite+ passes `--disable-nested-config` by default, so nested format configs do not override settings for individual files. This does not prevent discovery of a package-level config when running from that package. See [troubleshooting](/guide/troubleshooting#nested-lint-or-format-config-is-not-applied) for details.
