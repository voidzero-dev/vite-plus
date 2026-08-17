# Test Config

`vp test` reads Vitest settings from the `test` block in `vite.config.ts`. See [Vitest's configuration](https://vitest.dev/config/) for details.

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    coverage: {
      reporter: ['text', 'html'],
    },
  },
});
```
