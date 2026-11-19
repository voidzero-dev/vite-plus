# migration_import_rule_runtime_cjs

Isolated config loading preserves CommonJS support and inherited lint overrides.

## `vpt rm vite.config.ts`


## `vpt write-file vite.config.cjs 'module.exports = () => ({ lint: { extends: [{ overrides: [{ files: ['\''templates/**'\'', '\''examples/**'\''], rules: { '\''vite-plus/prefer-vite-plus-imports'\'': '\''off'\'' } }] }], overrides: [{ files: ['\''templates/managed/**'\''], rules: { '\''vite-plus/prefer-vite-plus-imports'\'': '\''error'\'' } }] } });
'`


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
