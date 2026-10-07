# Staged Config

`vp staged` reads staged-file rules from the `staged` block in `vite.config.ts`. See the [Commit hooks guide](/guide/commit-hooks).

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*': 'vp check --fix',
  },
});
```
