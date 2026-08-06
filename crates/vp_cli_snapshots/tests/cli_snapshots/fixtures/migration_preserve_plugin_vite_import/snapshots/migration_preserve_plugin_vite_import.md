# migration_preserve_plugin_vite_import

## `vp migrate --no-interactive --no-hooks`

`vite` is rewritten to vite-plus only in config entry files; every other file keeps its `vite` imports (issue #2004)

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file vite.config.ts`

REWRITTEN: the config entry's `defineConfig` import becomes vite-plus

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt print-file packages/app/src/main.ts`

PRESERVED: a normal app's non-config `vite` import (createServer, typeof import) stays on vite

```
import { createServer } from 'vite';

// A Vite-core programmatic API. `vite-plus` does not re-expose it on its public
// surface, so this import (and the type position below) must stay on `vite`
// rather than be rewritten to `vite-plus` (issue #2004).
export type ViteApi = Pick<typeof import('vite'), 'createBuilder' | 'loadConfigFromFile'>;

export async function start() {
  const server = await createServer();
  await server.listen();
}
```

## `vpt print-file packages/vite-plugin-demo/index.ts`

PRESERVED: a vite-plugin-* package keeps `from 'vite'` so it stays usable by plain-vite projects

```
import type { Plugin } from 'vite';

export default function demo(): Plugin {
  return { name: 'vite-plugin-demo' };
}
```

## `vpt print-file packages/unplugin-demo/src/index.ts`

PRESERVED: an unplugin-* package keeps `from 'vite'` too

```
import type { Plugin } from 'vite';

export function vitePlugin(): Plugin {
  return { name: 'unplugin-demo' };
}
```
