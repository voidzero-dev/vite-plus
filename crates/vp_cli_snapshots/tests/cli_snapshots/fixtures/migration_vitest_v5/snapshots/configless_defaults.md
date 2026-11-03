# configless_defaults

Migrate a configless workspace member's source APIs without a review or a new compatibility config.

## `vpt write-file pnpm-workspace.yaml 'packages:
  - packages/*
'`


## `vpt write-file packages/unit/package.json '{"name":"unit","private":true,"type":"module","devDependencies":{"vitest":"4.1.11"}}
'`


## `vpt cp example.test.ts packages/unit/example.test.ts`


## `vpt rm example.test.ts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

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

## `vpt stat-file packages/unit/.gitignore --assert missing`

```
packages/unit/.gitignore: missing
```

## `vpt print-file .gitignore`

```
.vitest/
```

## `vpt print-file packages/unit/example.test.ts`

```
import { expect, test } from 'vite-plus/test';

test('compatibility', { concurrent: false }, async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow(/^$/);
});
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
