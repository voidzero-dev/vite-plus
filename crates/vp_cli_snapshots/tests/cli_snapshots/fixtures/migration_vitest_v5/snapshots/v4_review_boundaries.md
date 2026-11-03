# v4_review_boundaries

Review kebab-case and config name filters, descriptor restoration callbacks, matcher type aliases, and Temporal initialized in a separate setup file.

## `vpt json-edit package.json scripts.test 'vitest run --test-name-pattern '\''suite test'\'''`


## `vpt write-file vite.config.ts 'export default { test: { setupFiles: ['\''./setup.ts'\''], testNamePattern: '\''suite test'\'' } };
'`


## `vpt write-file setup.ts 'import '\''temporal-polyfill/global'\'';
'`


## `vpt write-file environment.ts 'import { populateGlobal } from '\''vitest/environments'\'';
const { originals } = populateGlobal(global, window);
originals.forEach((value, key) => { global[key] = value; });
'`


## `vpt write-file matchers.d.ts 'import type { Assertion as A } from '\''vitest'\'';
declare global { namespace jest { interface Matchers<R, T> { toBeCustom(): R; } } }
type Result = A<string>;
'`


## `vpt write-file clock.test.ts 'import { vi, test } from '\''vitest'\'';
test('\''clock'\'', () => { vi.setSystemTime(0); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 files had imports rewritten
! Warnings:
  - Vitest v5: 6 review items

clock.test.ts
  2:23 REVIEW [temporal-system-time] vi.setSystemTime now changes Temporal even without fake timers; toNotFake does not preserve this behavior.
    Docs: https://vitest.dev/guide/migration/#fake-timers-and-setsystemtime-now-mock-temporal

environment.ts
  3:37 REVIEW [global-descriptors] populateGlobal().originals stores descriptors; restore them with Object.defineProperty, not assignment.
    Docs: https://vitest.dev/guide/migration/#populateglobal-returns-descriptors-in-originals

matchers.d.ts
  2:35 REVIEW [assertion-types] Update custom matcher declarations for the v5 return and received type parameters; review jest.Matchers augmentations.
    Docs: https://vitest.dev/guide/migration/#assertion-types-expose-return-and-received-types
  3:15 REVIEW [assertion-types] Review old assertion generics; v5 includes return and received types.
    Docs: https://vitest.dev/guide/migration/#assertion-types-expose-return-and-received-types

package.json
  10:1 REVIEW [test-name-pattern] Review test-name patterns across suite boundaries; full names now use > separators.
    Docs: https://vitest.dev/guide/migration/#testnamepattern-matches-the-joined-full-name

vite.config.ts
  17:35 REVIEW [test-name-pattern] Review test-name patterns across suite boundaries; full names now use > separators.
    Docs: https://vitest.dev/guide/migration/#testnamepattern-matches-the-joined-full-name
```
