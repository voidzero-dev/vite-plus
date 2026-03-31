# Configuration Troubleshooting

Use this page when your Vite+ configuration is not behaving the way you expect.

## Slow config loading caused by heavy plugins

When `vite.config.ts` imports heavy plugins at the top level, every `import` is evaluated eagerly, even for commands like `vp lint` or `vp fmt` that don't need those plugins. This can make config loading noticeably slow.

Use the `lazy` field in `defineConfig` to defer heavy plugin loading. Plugins provided through `lazy` are only resolved when Vite actually needs them:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lazy: async () => {
    const { default: heavyPlugin } = await import('vite-plugin-heavy');
    return { plugins: [heavyPlugin()] };
  },
});
```

You can keep lightweight plugins inline and defer only the expensive ones. Plugins from `lazy` are appended after existing plugins:

```ts
import { defineConfig } from 'vite-plus';
import lightPlugin from 'vite-plugin-light';

export default defineConfig({
  plugins: [lightPlugin()],
  lazy: async () => {
    const { default: heavyPlugin } = await import('vite-plugin-heavy');
    return { plugins: [heavyPlugin()] };
  },
});
```

The resulting plugin order is: `[lightPlugin(), heavyPlugin()]`.

::: info
The `lazy` field is a Vite+ extension. We plan to support this in upstream Vite in the future.
:::
