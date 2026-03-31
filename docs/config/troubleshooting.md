# Configuration Troubleshooting

Use this page when your Vite+ configuration is not behaving the way you expect.

## Slow config loading caused by heavy plugins

When `vite.config.ts` imports heavy plugins at the top level, every `import` is evaluated eagerly, even for commands like `vp lint` or `vp fmt` that don't need those plugins. This can make config loading noticeably slow.

Vite supports promises in the `plugins` array, so you can use dynamic `import()` to defer heavy plugin loading:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [
    import('vite-plugin-heavy').then((m) => m.default()),
  ],
});
```

This way the plugin module is only loaded when Vite actually resolves plugins, keeping config loading fast for commands that don't need them.

You can mix regular plugins with deferred ones. Lightweight plugins stay inline while expensive ones use dynamic `import()`:

```ts
import { defineConfig } from 'vite-plus';
import lightPlugin from 'vite-plugin-light';

export default defineConfig({
  plugins: [
    lightPlugin(),
    import('vite-plugin-heavy').then((m) => m.default()),
  ],
});
```
