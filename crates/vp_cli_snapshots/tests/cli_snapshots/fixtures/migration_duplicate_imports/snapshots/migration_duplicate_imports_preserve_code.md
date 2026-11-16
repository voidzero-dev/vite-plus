# migration_duplicate_imports_preserve_code

## `vpt replace-file-content vite.config.ts 'import { defineConfig }' 'const keep = '\''keep'\'';
import '\''./setup'\'';
import { defineConfig }'`


## `vpt write-file setup.ts 'export {};
'`


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

Keep statements and side-effect imports between matching imports.

```
import type { Plugin } from 'vite-plus';
const keep = 'keep';
import './setup';
import { defineConfig } from 'vite-plus';

const noop = (): Plugin => ({ name: 'noop' });

export default defineConfig({
  plugins: [noop()],
});
```
