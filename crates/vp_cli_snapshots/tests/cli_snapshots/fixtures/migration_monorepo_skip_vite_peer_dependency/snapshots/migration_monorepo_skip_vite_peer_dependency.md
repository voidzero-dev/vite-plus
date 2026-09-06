# migration_monorepo_skip_vite_peer_dependency

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"4.1.11"}'`

record the original runner version without adding a direct dependency


## `vp migrate --no-interactive`

migration should preserve vite peer contracts in workspace packages

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

packages/vite-plugin/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

packages/vite-plugin/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file packages/vite-plugin/src/index.ts`

vite-plugin has vite in peerDeps: vite imports stay public, vitest rewrites

```
import { defineConfig, type Plugin } from 'vite';
import { describe, it, expect } from 'vite-plus/test';

export function myVitePlugin(): Plugin {
  return {
    name: 'my-vite-plugin',
    configResolved(config) {
      console.log(config);
    },
  };
}

describe('myVitePlugin', () => {
  it('should work', () => {
    expect(myVitePlugin()).toBeDefined();
  });
});

export default defineConfig({
  plugins: [myVitePlugin()],
});
```

## `vpt print-file package.json`

check root package.json (no peerDependencies)

```
{
  "name": "migration-monorepo-skip-vite-peer-dependency",
  "devDependencies": {
    "vite": "catalog:",
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

## `vpt print-file packages/vite-plugin/package.json`

vite peer range is preserved

```
{
  "name": "my-vite-plugin",
  "peerDependencies": {
    "vite": "^6.0.0"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
```
