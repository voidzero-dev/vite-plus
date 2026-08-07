# migration_framework_shim_astro_vue

## `vp migrate --no-interactive --no-hooks`

migration should add both Vue and Astro shims

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
• TypeScript shim added for framework component files
```

## `vpt print-file src/env.d.ts`

check both shims were written

```
declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, unknown>;
  export default component;
}

/// <reference types="astro/client" />
```
