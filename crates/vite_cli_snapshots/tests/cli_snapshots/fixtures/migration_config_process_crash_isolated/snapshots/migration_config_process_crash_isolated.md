# migration_config_process_crash_isolated

## `vp migrate --no-interactive --no-hooks`

project config process handlers must not terminate migration

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file vite.config.ts`

migration still rewrites the config after its compatibility probe crashes

```
import { defineConfig } from 'vite-plus';

// Models a project plugin that installs a process-level error backstop while
// its config is loaded. Re-throwing from this handler makes Node exit with code
// 7, which used to terminate `vp migrate` during its best-effort compatibility
// check instead of allowing migration to continue.
process.on('uncaughtException', (error) => {
  throw error;
});
queueMicrotask(() => {
  throw new Error('simulated project config crash');
});

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```
