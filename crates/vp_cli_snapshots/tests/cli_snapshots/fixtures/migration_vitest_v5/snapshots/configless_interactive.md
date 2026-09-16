# configless_interactive

Complete interactive migration without prompting to create a Vitest compatibility config in a configless workspace member.

## `vpt write-file pnpm-workspace.yaml 'packages:
  - packages/*
'`


## `vpt write-file packages/unit/package.json '{"name":"unit","private":true,"type":"module","devDependencies":{"vitest":"4.1.11"}}
'`


## `vpt cp example.test.ts packages/unit/example.test.ts`


## `vpt rm example.test.ts`


## `vp pm config get registry`


## `vp migrate --interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt stat-file packages/unit/vite.config.ts --assert missing`

```
packages/unit/vite.config.ts: missing
```

## `vpt print-file packages/unit/example.test.ts`

```
import { expect, test } from 'vite-plus/test';

test('compatibility', { concurrent: false }, async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow(/^$/);
});
```
