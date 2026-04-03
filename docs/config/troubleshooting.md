# Configuration Troubleshooting

Use this page when your Vite+ configuration is not behaving the way you expect.

## Slow config loading caused by heavy plugins

When `vite.config.ts` imports heavy plugins at the top level, every `import` is evaluated eagerly, even for commands like `vp lint` or `vp fmt` that don't need those plugins. This can make config loading noticeably slow.

Use the `vitePlugins()` helper to conditionally load plugins. It checks which `vp` command is running and skips plugin loading for commands that don't need them (like `lint`, `fmt`, `check`):

```ts
import { defineConfig, vitePlugins } from 'vite-plus';

export default defineConfig({
  plugins: vitePlugins(() => [myPlugin()]),
});
```

For heavy plugins that should be lazily imported, combine with dynamic `import()`:

```ts
import { defineConfig, vitePlugins } from 'vite-plus';

export default defineConfig({
  plugins: vitePlugins(async () => {
    const { default: heavyPlugin } = await import('vite-plugin-heavy');
    return [heavyPlugin()];
  }),
});
```

Plugins load for `dev`, `build`, `test`, and `preview`. They are skipped for `lint`, `fmt`, `check`, and other commands that don't need them.

::: info
`vitePlugins()` works by checking the `VP_COMMAND` environment variable, which is automatically set by `vp` for every command.
:::
