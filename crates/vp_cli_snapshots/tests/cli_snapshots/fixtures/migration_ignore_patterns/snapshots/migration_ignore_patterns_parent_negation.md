# migration_ignore_patterns_parent_negation

## `vpt write-file vite.config.ts 'export default { migrate: { ignorePatterns: ['\''templates/'\'', '\'''\!'templates/managed/unit.test.ts'\''] } };
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

## `vpt print-file templates/managed/unit.test.ts`

```
import { expect, test } from 'vitest';
test('included', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('application', () => expect(1).toBe(1));
```
