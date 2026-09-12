# node_preflight

## `vpt write-file .node-version '20.19.0
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item (1 block dependency updates)

.node-version
  1:1 BLOCK [node-runtime] .node-version (20.19.0) cannot run Vite+ with Vitest v5. Select Node ^22.18.0 || ^24.11.0 || >=26.0.0; use vp env pin 22 --force for a runtime pin. Do not widen a library's engines.node contract automatically.
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
```

## `vpt print-file package.json`

```
{
  "name": "migration-vitest-v5",
  "private": true,
  "type": "module",
  "packageManager": "pnpm@11.24.0",
  "scripts": {
    "test": "vitest list"
  },
  "devDependencies": {
    "vite": "^8.0.0",
    "vitest": "<version>"
  }
}
```

## `vpt print-file example.test.ts`

```
import { expect, test } from 'vitest';

test.sequential('compatibility', () => {
  expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow('');
});
```
