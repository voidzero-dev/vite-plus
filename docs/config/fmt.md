# Format Config

`vp fmt` and `vp check` read Oxfmt settings from the `fmt` block in `vite.config.ts`. See [Oxfmt's configuration](https://oxc.rs/docs/guide/usage/formatter/config.html) for details.

## Example

```ts
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
