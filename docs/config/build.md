# Build Config

`vp dev`, `vp build`, and `vp preview` use the standard [Vite configuration](https://vite.dev/config/), including [plugins](https://vite.dev/guide/using-plugins), [aliases](https://vite.dev/config/shared-options#resolve-alias), [`server`](https://vite.dev/config/server-options), [`build`](https://vite.dev/config/build-options) and [`preview`](https://vite.dev/config/preview-options) fields.

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  server: {
    port: 3000,
  },
  build: {
    sourcemap: true,
  },
  preview: {
    port: 4173,
  },
});
```
