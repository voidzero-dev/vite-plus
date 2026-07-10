# migration_rewrite_declare_module

## `vp migrate --no-interactive`

retained vitest augmentations should keep a package-local vitest

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file src/index.ts`

declare module 'vite'/'vitest' outside config files are preserved: through the core alias a 'vite' augmentation reaches the UserConfig that types vite-plus defineConfig, while vite-plus exports no UserConfig symbol to merge a rewritten augmentation with

```
import type { RuntimeEnvConfig } from './runtime.env.config.js';
import type { RuntimeHtmlConfig } from './runtime.html.config.js';

declare module 'vite' {
  interface UserConfig {
    /**
     * Options for vite-plugin-runtime-env
     */
    runtimeEnv?: RuntimeEnvConfig;
    /**
     * Options for vite-plugin-runtime-html
     */
    runtimeHtml?: RuntimeHtmlConfig;
  }
}

declare module 'vitest' {
  export const describe: any;
  export const it: any;
  export const expect: any;
  export const beforeAll: any;
  export const afterAll: any;
}

declare module 'vitest/config' {
  export function defineConfig(config: any): any;
  const _default: typeof defineConfig;
  export default _default;
}
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-rewrite-declare-module",
  "devDependencies": {
    "vite": "catalog:",
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```
