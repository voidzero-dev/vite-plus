# migration_skip_vite_peer_dependency

## `vp migrate --no-interactive`

migration should preserve vite peer contracts

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file src/index.ts`

vite imports stay public, vitest imports rewrite

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

vite peer range is preserved

```
{
  "name": "migration-skip-vite-peer-dependency",
  "peerDependencies": {
    "vite": "^6.0.0"
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
