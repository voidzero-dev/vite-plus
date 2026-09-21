import { afterEach, expect, it, vi } from 'vitest';

import { documentationUrl as nativeDocumentationUrl } from '../../binding/index.js';
import { compatibilityProperty } from '../migration/vitest-v5/compatibility.ts';
import { vitestV5Documentation } from '../migration/vitest-v5/documentation.ts';
import { documentationUrl, rewriteDocumentationLinks } from '../utils/documentation.ts';

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetModules();
});

it('uses the compiled native origin for all JavaScript documentation links', () => {
  const guide = nativeDocumentationUrl('/guide/vitest-v5');
  expect(documentationUrl('/guide/vitest-v5')).toBe(guide);
  expect(vitestV5Documentation('node-runtime')).toBe(`${guide}#node-runtime`);
  expect(compatibilityProperty('clearMocks').comment).toContain(
    `${guide}#remove-unneeded-compatibility-settings`,
  );
  expect(rewriteDocumentationLinks('Docs: https://viteplus.dev/guide/vitest-v5')).toBe(
    `Docs: ${guide}`,
  );
  expect(vitestV5Documentation('benchmark-api')).toBe(
    'https://vitest.dev/guide/migration/#benchmarking-api-rewrite',
  );
});

it('ignores runtime environment overrides, including on a fresh JavaScript import', async () => {
  const expected = nativeDocumentationUrl('/guide/migrate');
  vi.stubEnv('VITE_PLUS_DOCS_ORIGIN', 'https://runtime-override.invalid');
  vi.resetModules();
  const reloaded = await import('../utils/documentation.ts');
  expect(nativeDocumentationUrl('/guide/migrate')).toBe(expected);
  expect(reloaded.documentationUrl('/guide/migrate')).toBe(expected);
});
