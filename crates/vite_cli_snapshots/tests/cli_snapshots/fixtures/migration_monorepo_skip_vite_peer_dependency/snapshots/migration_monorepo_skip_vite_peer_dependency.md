# migration_monorepo_skip_vite_peer_dependency

## `vp migrate --no-interactive`

migration should preserve vite peer contracts in workspace packages

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
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
