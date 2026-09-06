# migration_skip_vite_dependency

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"4.1.11"}'`

record the original runner version without adding a direct dependency


## `vp migrate --no-interactive`

migration should skip rewriting vite imports when vite is in dependencies

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
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
  vite@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
