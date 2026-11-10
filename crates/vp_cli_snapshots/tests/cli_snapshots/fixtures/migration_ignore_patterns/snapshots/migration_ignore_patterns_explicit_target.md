# migration_ignore_patterns_explicit_target

An explicit non-workspace template target must not bypass the workspace policy.

## `vp migrate examples/standalone --full --no-interactive --no-hooks --no-agent --no-editor`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

■ Cannot migrate examples/standalone: excluded by migrate.ignorePatterns in <workspace>/vite.config.ts. No project files were changed.
Error: Cannot migrate examples/standalone: excluded by migrate.ignorePatterns in <workspace>/vite.config.ts. No project files were changed.
```

## `vpt print-file examples/standalone/package.json`

```
{ "name": "standalone-template", "private": true, "type": "module", "devDependencies": { "vite": "^8.0.0", "vitest": "^4.1.11" } }
```

## `vpt print-file examples/standalone/unit.test.ts`

```
import { expect, test } from 'vitest';
test('outside-workspace', () => expect(1).toBe(1));
```
