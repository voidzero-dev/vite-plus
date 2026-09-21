# callback_workspace_scopes

Resolve static callback test options independently of a tooling-only workspace root and a sibling browser suite. Preserve Node text matchers and unrelated collect calls.

## `vpt json-edit package.json scripts {}`


## `vpt rm example.test.ts`


## `vpt write-file vite.config.ts 'export default { fmt: { semi: false } };
'`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - web
  - browser
'`


## `vpt write-file web/package.json '{"name":"web","private":true,"type":"module","scripts":{"test":"vitest"},"devDependencies":{"vitest":"4.1.11"}}
'`


## `vpt write-file browser/package.json '{"name":"browser","private":true,"type":"module","scripts":{"test":"vitest"},"devDependencies":{"vitest":"4.1.11"}}
'`


## `vpt write-file web/vite.config.ts 'import { defineConfig } from '\''vitest/config'\'';
export default defineConfig(({ mode }) => {
  const isTest = mode === '\''test'\'';
  return {
    plugins: isTest ? [] : [],
    ...('\!'isTest ? { server: { port: 3000 } } : {}),
    test: { globals: true, setupFiles: ['\''./setup.ts'\''] },
  };
});
'`


## `vpt write-file web/setup.ts 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file web/example.test.ts 'import { expect, vi } from '\''vitest'\'';
vi.mock('\''./dependency'\'');
export function checkText(element) { expect(element).toHaveTextContent('\''partial'\''); }
test.sequential('\''works'\'', () => { expect(1).toBe(1); });
'`


## `vpt write-file browser/vitest.config.ts 'export default { test: { globals: true, browser: { enabled: true } } };
'`


## `vpt write-file browser/example.test.ts 'test('\''text'\'', async () => { expect.element(element).toHaveTextContent('\''partial'\''); });
'`


## `vpt write-file web/public/editor.js 'export const result = collector.collect(options);
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file web/vite.config.ts web/setup.ts web/example.test.ts browser/example.test.ts web/public/editor.js`

```
import { defineConfig } from 'vite-plus';
export default defineConfig(({ mode }) => {
  const isTest = mode === 'test';
  return {
    plugins: isTest ? [] : [],
    ...(!isTest ? { server: { port: 3000 } } : {}),
    test: {
      // Vitest v4 compatibility: preserve mock call history.
      // Remove after tests no longer rely on calls from setup or earlier tests.
      // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
      // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
      clearMocks: false,
      globals: true, setupFiles: ['./setup.ts'] },
  };
});
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
import { expect, vi } from 'vite-plus/test';
vi.mock('./dependency');
export function checkText(element) { expect(element).toHaveTextContent('partial'); }
test('works', { concurrent: false }, () => { expect(1).toBe(1); });
test('text', async () => { await expect.element(element).toMatchTextContent('partial'); });
export const result = collector.collect(options);
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
