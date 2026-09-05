# migration_pack_tsdown_023

Upgrade an existing Vite+ pack config without --full and preserve it on a second migration.

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite             → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig(() => ({
  // This is Vite's public directory, which must stay unchanged.
  publicDir: 'vite-public',
  pack: [
    {
      entry: 'src/index.ts',
      unbundle: true,
      outExtensions: () => ({ js: '.mjs' }),
      copy: 'public',
      nodeProtocol: 'strip',
      css: { inject: true },
      deps: { neverBundle: true, onlyBundle: [/^allowed/], resolveDepSubpath: true },

      dts: { generator: 'oxc',  },
      attw: { profile: 'strict' },
    },
    {
      entry: 'src/index.ts',
      deps: { onlyBundle: false, neverBundle: true, resolveDepSubpath: false },
      dts: { generator: 'tsgo', tsgo: { path: './tsgo' },  },
      attw: { profile: 'node16' },
    },
  ],
}));
```

## `vpt print-file package.json`

```
{
  "name": "migration-pack-tsdown-023",
  "type": "module",
  "private": true,
  "scripts": {
    "build": "vp pack --copy public"
  },
  "devDependencies": {
    "vite-plus": "<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  }
}
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig(() => ({
  // This is Vite's public directory, which must stay unchanged.
  publicDir: 'vite-public',
  pack: [
    {
      entry: 'src/index.ts',
      unbundle: true,
      outExtensions: () => ({ js: '.mjs' }),
      copy: 'public',
      nodeProtocol: 'strip',
      css: { inject: true },
      deps: { neverBundle: true, onlyBundle: [/^allowed/], resolveDepSubpath: true },

      dts: { generator: 'oxc',  },
      attw: { profile: 'strict' },
    },
    {
      entry: 'src/index.ts',
      deps: { onlyBundle: false, neverBundle: true, resolveDepSubpath: false },
      dts: { generator: 'tsgo', tsgo: { path: './tsgo' },  },
      attw: { profile: 'node16' },
    },
  ],
}));
```
