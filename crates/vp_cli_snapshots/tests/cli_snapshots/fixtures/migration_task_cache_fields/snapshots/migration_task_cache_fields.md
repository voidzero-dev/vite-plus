# migration_task_cache_fields

Move task cache settings under cache on an existing Vite+ project without --full, then keep them on a second migration.

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

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
        cache: {
        // Rebuild when the deploy target changes.
        env: ['DEPLOY_TARGET'],
        untrackedEnv: ['CI'],
        input: [{ auto: true }, '!dist/**'],
        output: ['dist/**'], // restored on cache hits
        },
      },
      typecheck: {
        command: 'tsc --noEmit',
        cache: {
        env: ['TSC_MODE'],
        },
      },
      lint: {
        command: 'vp lint',
        cache: {
          env: ['LINT_LEVEL'],
          input: ['src/**'],
        },
      },
    },
  },
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
        cache: {
        // Rebuild when the deploy target changes.
        env: ['DEPLOY_TARGET'],
        untrackedEnv: ['CI'],
        input: [{ auto: true }, '!dist/**'],
        output: ['dist/**'], // restored on cache hits
        },
      },
      typecheck: {
        command: 'tsc --noEmit',
        cache: {
        env: ['TSC_MODE'],
        },
      },
      lint: {
        command: 'vp lint',
        cache: {
          env: ['LINT_LEVEL'],
          input: ['src/**'],
        },
      },
    },
  },
});
```
