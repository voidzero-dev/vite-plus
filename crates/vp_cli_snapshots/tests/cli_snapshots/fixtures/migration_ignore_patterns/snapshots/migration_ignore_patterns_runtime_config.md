# migration_ignore_patterns_runtime_config

A runtime config export retains the policy, and the root config can be a plain object.

## `vpt write-file vite.config.ts 'const excluded = ['\''templates/*'\'', '\'''\!'templates/managed/'\''];
export default () => ({ migrate: { ignorePatterns: excluded } });
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
• 4 files had imports rewritten
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
