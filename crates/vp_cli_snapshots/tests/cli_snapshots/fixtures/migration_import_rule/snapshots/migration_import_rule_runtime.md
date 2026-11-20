# migration_import_rule_runtime

Dynamic lint configuration uses normal config resolution.

## `vpt write-file vite.config.ts 'const files = ['\''templates/**'\'', '\''examples/**'\''];
export default () => ({ lint: { overrides: [{ files, excludeFiles: ['\''templates/managed/**'\''], rules: { '\''vite-plus/prefer-vite-plus-imports'\'': '\''off'\'' } }] } });
'`

```
```

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

## `vpt print-file templates/starter/vite.config.ts`

```
import { defineConfig } from 'vite';
export default defineConfig({test: {
    // Vitest v4 compatibility: preserve mock call history.
    // Remove after tests no longer rely on calls from setup or earlier tests.
    // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
    // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
    clearMocks: false,
    poolOptions: {threads: {singleThread: true}}}});
```

## `vpt print-file templates/starter/unit.test.ts`

```
import { expect, test } from 'vitest';
debugger;
test('template', () => expect(1).toBe(1));
```

## `vpt print-file templates/managed/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('included', () => expect(1).toBe(1));
```

## `vpt print-file examples/standalone/unit.test.ts`

```
import { expect, test } from 'vitest';
test('outside-workspace', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('application', () => expect(1).toBe(1));
```
