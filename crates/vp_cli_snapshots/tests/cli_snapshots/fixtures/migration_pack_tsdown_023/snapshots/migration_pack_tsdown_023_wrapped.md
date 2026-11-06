# migration_pack_tsdown_023_wrapped

Upgrade a pack config whose config object is wrapped in a type assertion.

## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\'';
import type { UserConfig } from '\''vite-plus'\'';

export default defineConfig({
  pack: {
    entry: '\''src/index.ts'\'',
    bundle: false,
  },
} as UserConfig);
'`


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
import type { UserConfig } from 'vite-plus';

export default defineConfig({
  pack: { deps: {
      // tsdown <0.23 compatibility: resolve external dependency subpaths.
      // Remove to preserve subpath imports as written (the new default).
      // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
      resolveDepSubpath: true },
    entry: 'src/index.ts',
    unbundle: true,
  },
} as UserConfig);
```
