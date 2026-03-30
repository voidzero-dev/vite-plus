# Configuring Vite+

Vite+ keeps project configuration in one place: `vite.config.ts`, allowing you to consolidate many top-level configuration files in a single file. You can keep using your Vite configuration such as `server` or `build`, and add Vite+ blocks for the rest of your workflow:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  server: {},
  build: {},
  preview: {},

  test: {},
  lint: {},
  fmt: {},
  run: {},
  pack: {},
  staged: {},
});
```

## Vite+ Specific Configuration

Vite+ extends the basic Vite configuration with these additions:

- [`lint`](/config/lint) for Oxlint
- [`fmt`](/config/fmt) for Oxfmt
- [`test`](/config/test) for Vitest
- [`run`](/config/run) for Vite Task
- [`pack`](/config/pack) for tsdown
- [`staged`](/config/staged) for staged-file checks

## Lazy Loading Plugins

When `vite.config.ts` imports heavy plugins at the top level, every `import` is evaluated eagerly, even for commands like `vp lint` or `vp fmt` that don't need those plugins. This can make config loading noticeably slow.

The `lazy` field solves this by letting you defer plugin loading into an async function. Plugins provided through `lazy` are only resolved when actually needed:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lazy: async () => {
    const { default: myHeavyPlugins } = await import('./my-heavy-plugins');
    return { plugins: myHeavyPlugins };
  },
});
```

### Type Signature

```ts
lazy?: () => Promise<{
  plugins?: Plugin[];
}>;
```

### Merging with Existing Plugins

Plugins returned from `lazy` are appended after any plugins already in the `plugins` array. This lets you keep lightweight plugins inline and defer only the expensive ones:

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

### Function Config

`lazy` also works with function-style and async function-style configs:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig(async () => ({
  lazy: async () => {
    const { default: heavyPlugin } = await import('vite-plugin-heavy');
    return { plugins: [heavyPlugin()] };
  },
}));
```

::: info
The `lazy` field is a temporary Vite+ extension. We plan to support this in upstream Vite in the future.
:::
