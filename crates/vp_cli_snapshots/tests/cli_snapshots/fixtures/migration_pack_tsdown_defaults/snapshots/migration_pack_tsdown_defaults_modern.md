# migration_pack_tsdown_defaults_modern

Read tsdown 0.23 from the installed Vite+ 1.0.0 toolchain manifest and preserve omitted and explicit pack defaults across workspace packages and repeated migrations.

## `vpt write-file node_modules/vite/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"1.0.0","bundledVersions":{"vite":"8.3.1"}}'`


## `vpt write-file node_modules/vite-plus/dist/toolchain.json '{"schemaVersion":1,"nodes":[{"id":"tsdown","name":"tsdown","version":"0.23.0","kind":"tool","delivery":["bundled"]}],"edges":[]}'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  1.0.0 → <version>
    vite       8.3.1 → <version>
• Package manager settings configured
```

## `vpt print-file apps/server-v2/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/main.ts'],
    format: ['esm'],
  },
});
```

## `vpt print-file packages/library/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: [
    {
      entry: ['src/index.ts'],
      attw: true,
    },
    {
      entry: ['src/index.ts'],
      deps: {},
      attw: { enabled: true },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: false },
      attw: { profile: 'esm-only' },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: true },
      attw: { profile: 'strict' },
    },
  ],
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file apps/server-v2/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/main.ts'],
    format: ['esm'],
  },
});
```

## `vpt print-file packages/library/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: [
    {
      entry: ['src/index.ts'],
      attw: true,
    },
    {
      entry: ['src/index.ts'],
      deps: {},
      attw: { enabled: true },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: false },
      attw: { profile: 'esm-only' },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: true },
      attw: { profile: 'strict' },
    },
  ],
});
```
