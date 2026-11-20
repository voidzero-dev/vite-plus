# migration_import_rule_reporter_workspace

Migration and lint autofix respect the root overrides for templates and questions, while rewriting app imports.

## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\'';
export default defineConfig({ lint: { jsPlugins: ['\''vite-plus/oxlint-plugin'\''], rules: { '\''vite-plus/prefer-vite-plus-imports'\'': '\''error'\'' }, overrides: [{ files: ['\''templates/**'\'', '\''questions/**'\''], rules: { '\''vite-plus/prefer-vite-plus-imports'\'': '\''off'\'' } }] } });
'`


## `vpt write-file questions/demo.test.ts 'import { test } from '\''vitest'\'';
test('\''question'\'', () => {});
'`


## `vpt write-file packages/app/demo.test.ts 'import { test } from '\''vitest'\'';
test('\''app'\'', () => {});
'`


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

## `vpt print-file questions/demo.test.ts`

```
import { test } from 'vitest';
test('question', () => {});
```

## `vpt print-file packages/app/demo.test.ts`

```
import { test } from 'vite-plus/test';
test('app', () => {});
```

## `vpt write-file packages/app/demo.test.ts 'import { test } from '\''vitest'\'';
test('\''app'\'', () => {});
'`


## `vp lint --threads=1 --fix templates/starter/vite.config.ts questions/demo.test.ts packages/app/demo.test.ts`

```
Found 0 warnings and 0 errors.
Finished in <duration> on 3 files with <n> rules using <n> threads.
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

## `vpt print-file questions/demo.test.ts`

```
import { test } from 'vitest';
test('question', () => {});
```

## `vpt print-file packages/app/demo.test.ts`

```
import { test } from 'vite-plus/test';
test('app', () => {});
```
