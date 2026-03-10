# Pack Config

`vp pack` reads tsdown settings from the `pack` block in `vite.config.ts`. See [tsdown's configuration](https://tsdown.dev/options/config-file) for details.

## Example

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    dts: true,
    format: ['esm', 'cjs'],
    sourcemap: true,
  },
});
```
