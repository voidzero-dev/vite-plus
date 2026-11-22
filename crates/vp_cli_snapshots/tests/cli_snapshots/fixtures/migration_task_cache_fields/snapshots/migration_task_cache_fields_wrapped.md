# migration_task_cache_fields_wrapped

Move task cache settings when the defineConfig callback is wrapped in a type assertion.

## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\'';
import type { UserConfigFn } from '\''vite-plus'\'';

export default defineConfig((() => {
  return {
    run: { tasks: { build: { command: '\''vp build'\'', env: ['\''DEPLOY_TARGET'\''] } } },
  };
}) as UserConfigFn);
'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite             → <version>
• 1 config update applied
• Task cache settings moved under `cache`
• Package manager settings configured
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';
import type { UserConfigFn } from 'vite-plus';

export default defineConfig((() => {
  return {
    run: { tasks: { build: { command: 'vp build', cache: { env: ['DEPLOY_TARGET'] } } } },
  };
}) as UserConfigFn);
```
