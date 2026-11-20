# migration_duplicate_imports

## `vp migrate --no-interactive --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  1.5.0 → <version>
    vite             → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file vite.config.ts`

Merge the rewritten type import into the existing value import (issue #3430).

```
import { defineConfig, type Plugin } from 'vite-plus';

const noop = (): Plugin => ({ name: 'noop' });

export default defineConfig({
  plugins: [noop()],
});
```
