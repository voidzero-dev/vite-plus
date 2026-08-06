# migration_skip_vite_dependency

## `vp migrate --no-interactive`

migration should skip rewriting vite imports when vite is in dependencies

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file src/index.ts`

vite imports should NOT be rewritten, vitest imports SHOULD be rewritten

```
import { defineConfig, type Plugin } from 'vite';
import { describe, it, expect } from 'vite-plus/test';

export function myApp(): Plugin {
  return {
    name: 'my-app',
    configResolved(config) {
      console.log(config);
    },
  };
}

describe('myApp', () => {
  it('should work', () => {
    expect(myApp()).toBeDefined();
  });
});

export default defineConfig({
  plugins: [myApp()],
});
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-skip-vite-dependency",
  "dependencies": {
    "vite": "catalog:"
  },
  "devDependencies": {
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
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
