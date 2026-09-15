# version_gate_preserves_sources

A version check after initial setup must abort before Vitest compatibility edits.

## `vpt write-file node_modules/vite/package.json '{"name":"vite","version":"6.4.0"}
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

✘ vite@6.4.0 in package.json is not supported by auto migration

Please upgrade vite to version >=7.0.0 first
Vite+ cannot automatically migrate this project yet.
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    reporters: ['json', ['junit', {}]],
    projects: [{ test: { name: 'unit' } }, { extends: true, test: { name: 'inherited' } }],
  },
});
```

## `vpt print-file example.test.ts`

```
import { expect, test } from 'vitest';

test.sequential('compatibility', () => {
  expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow('');
});
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

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```
