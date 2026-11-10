# migration_ignore_patterns_upgrade

Preserve templates and their shared catalogs and overrides, re-include one template, and leave non-workspace examples alone.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 3 files had imports rewritten
• Package manager settings configured
```

## `vpt print-file templates/starter/package.json`

```
{
  "name": "starter", "private": true, "type": "module",
  "scripts": { "build": "vite build", "test": "vitest run" },
  "devDependencies": { "vite": "catalog:", "vitest": "catalog:legacy", "@vitest/coverage-v8": "catalog:legacy" }
}
```

## `vpt print-file templates/starter/vite.config.ts`

```
import { defineConfig } from 'vite';
export default defineConfig({test: {poolOptions: {threads: {singleThread: true}}}});
```

## `vpt print-file templates/starter/unit.test.ts`

```
import { expect, test } from 'vitest';
debugger;
test('template', () => expect(1).toBe(1));
```

## `vpt print-file templates/starter/tsconfig.json`

```
{ "compilerOptions": { "types": ["vite/client", "vitest/globals"], "esModuleInterop": false } }
```

## `vpt print-file examples/standalone/unit.test.ts`

```
import { expect, test } from 'vitest';
test('outside-workspace', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vp build",
    "test": "vp test run"
  },
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>",
    "@vitest/coverage-v8": "<version>"
  }
}
```

## `vpt print-file packages/app/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('application', () => expect(1).toBe(1));
```

## `vpt print-file templates/managed/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('included', () => expect(1).toBe(1));
```

## `vpt print-file pnpm-workspace.yaml`

```
packages:
  - packages/*
  - templates/*
catalog:
  vite-plus: latest
  vite: ^8.0.0
catalogs:
  legacy:
    vitest: ^4.1.11
    '@vitest/coverage-v8': ^4.1.11
overrides:
  unrelated-package: 1.0.0
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

Rerunning uses the same policy and explicit managed dependency specs.

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt write-file packages/app/unit.test.ts 'import { test } from '\''vitest'\'';
test('\''autofix'\'', () => {});
'`

```
```

## `vp lint --threads=1 --fix templates/starter/unit.test.ts packages/app/unit.test.ts`

Ignored imports remain upstream; other rules still report the template's debugger statement.

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[templates/starter/unit.test.ts:2:1]
 1 │ import { expect, test } from 'vitest';
 2 │ debugger;
   · ─────────
 3 │ test('template', () => expect(1).toBe(1));
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt print-file templates/starter/unit.test.ts`

```
import { expect, test } from 'vitest';
debugger;
test('template', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/unit.test.ts`

```
import { test } from 'vite-plus/test';
test('autofix', () => {});
```
