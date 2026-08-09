# lint_oxlint_plugin_api

## `vp lint src/uses-foo.ts`

the local JS plugin imports its API from vite-plus/lint/plugins. It declares no @oxlint/plugins dependency. A reported diagnostic therefore proves the export resolved and loaded

**Exit code:** 1

```

  × local(no-foo): Do not name things "foo".
   ╭─[src/uses-foo.ts:1:14]
 1 │ export const foo = 1;
   ·              ───
 2 │ export const bar = 2;
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint src/legacy-imports.ts`

prefer-vite-plus-imports reports the three legacy authoring specifiers

**Exit code:** 1

```

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/lint/plugins' instead of 'oxlint' in Vite+ projects.
   ╭─[src/legacy-imports.ts:1:28]
 1 │ import { defineRule } from 'oxlint';
   ·                            ────────
 2 │ import { definePlugin } from '@oxlint/plugins';
   ╰────

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/lint/plugins' instead of '@oxlint/plugins' in Vite+ projects.
   ╭─[src/legacy-imports.ts:2:30]
 1 │ import { defineRule } from 'oxlint';
 2 │ import { definePlugin } from '@oxlint/plugins';
   ·                              ─────────────────
 3 │ import { RuleTester } from 'oxlint/plugins-dev';
   ╰────

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/lint/plugins-dev' instead of 'oxlint/plugins-dev' in Vite+ projects.
   ╭─[src/legacy-imports.ts:3:28]
 2 │ import { definePlugin } from '@oxlint/plugins';
 3 │ import { RuleTester } from 'oxlint/plugins-dev';
   ·                            ────────────────────
 4 │
   ╰────

Found 0 warnings and 3 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint src/config-surface.ts`

oxlint still owns defineConfig and OxlintOverride, so these are clean

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --fix src/legacy-imports.ts`

the autofix matches what vp migrate rewrites

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt print-file src/legacy-imports.ts`

```
import { defineRule } from 'vite-plus/lint/plugins';
import { definePlugin } from 'vite-plus/lint/plugins';
import { RuleTester } from 'vite-plus/lint/plugins-dev';

export { defineRule, definePlugin, RuleTester };
```

## `vp lint src/legacy-imports.ts`

confirm the rewritten file is clean

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
