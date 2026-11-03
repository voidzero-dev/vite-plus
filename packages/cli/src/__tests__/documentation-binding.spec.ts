import { afterEach, expect, it, vi } from 'vitest';

import { documentationUrl as nativeDocumentationUrl } from '../../binding/index.js';

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetModules();
});

it('ignores runtime environment overrides, including on a fresh JavaScript import', async () => {
  const expected = nativeDocumentationUrl('/guide/migrate');
  vi.stubEnv('VITE_PLUS_DOCS_ORIGIN', 'https://runtime-override.invalid');
  vi.resetModules();
  const reloaded = await import('../utils/documentation.ts');
  expect(nativeDocumentationUrl('/guide/migrate')).toBe(expected);
  expect(reloaded.documentationUrl('/guide/migrate')).toBe(expected);
});
