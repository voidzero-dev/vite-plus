# migration_task_cache_fields_manual

Warn about tasks whose cache settings cannot be moved safely, move the rest, and keep warning without migrating again.

## `vpt cp manual.config.txt vite.config.ts`


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
! Warnings:
  - vite.config.ts: Move `env`, `untrackedEnv`, `input`, and `output` under `cache` manually in tasks `build`, `dev`; they were left unchanged. See https://viteplus.dev/config/run#cache
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

const shared = { env: ['NODE_ENV'] };

export default defineConfig({
  run: {
    tasks: {
      build: { ...shared, command: 'vp build', input: ['src/**'] },
      dev: { command: 'vp dev', cache: false, env: ['PORT'] },
      check: { command: 'vp check', cache: { env: ['CI'] } },
    },
  },
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

vite.config.ts: Move `env`, `untrackedEnv`, `input`, and `output` under `cache` manually in tasks `build`, `dev`; they were left unchanged. See https://viteplus.dev/config/run#cache
This project is already using Vite+! Happy coding!
```
