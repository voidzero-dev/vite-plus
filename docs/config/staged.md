# Staged Config

`vp staged` and `vp config` read staged-file rules from the `staged` block in `vite.config.ts`. See the [Commit hooks guide](/guide/commit-hooks).

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.{js,ts,tsx,vue,svelte}': 'vp check --fix',
  },
});
```
