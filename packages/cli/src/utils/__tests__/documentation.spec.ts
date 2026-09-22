import { afterEach, expect, it, vi } from 'vitest';

import { documentationUrl as nativeDocumentationUrl } from '../../../binding/index.js';

vi.mock('../../../binding/index.js', () => ({ documentationUrl: vi.fn() }));

afterEach(() => {
  vi.resetAllMocks();
  vi.unstubAllEnvs();
  vi.resetModules();
});

it.each([
  'https://viteplus.dev',
  'https://rfc-vitest-v5-upgrade-viteplus-dev.voidzero-docs.workers.dev',
])('uses the native documentation origin %s', async (origin) => {
  vi.mocked(nativeDocumentationUrl).mockImplementation((path) => `${origin}${path}`);
  vi.stubEnv('VITE_PLUS_DOCS_ORIGIN', 'https://runtime-override.invalid');
  vi.resetModules();
  const { documentationUrl, rewriteDocumentationLinks } = await import('../documentation.ts');
  const { compatibilityProperty } = await import('../../migration/vitest-v5/compatibility.ts');
  const { vitestV5Documentation } = await import('../../migration/vitest-v5/documentation.ts');

  expect(documentationUrl('/guide/create#organization-templates')).toBe(
    `${origin}/guide/create#organization-templates`,
  );
  expect(nativeDocumentationUrl).toHaveBeenCalledWith('/guide/create#organization-templates');
  expect(documentationUrl('/config/')).toBe(`${origin}/config/`);
  expect(vitestV5Documentation('node-runtime')).toBe(`${origin}/guide/vitest-v5#node-runtime`);
  expect(vitestV5Documentation('future-finding')).toBe(
    `${origin}/guide/vitest-v5#resolve-migration-findings`,
  );
  expect(vitestV5Documentation('benchmark-api')).toBe(
    'https://vitest.dev/guide/migration/#benchmarking-api-rewrite',
  );
  const property = compatibilityProperty('clearMocks');
  expect(property.comment).toContain(
    `// ${origin}/guide/vitest-v5#remove-unneeded-compatibility-settings`,
  );
  expect(property.comment).toContain(
    '// https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default',
  );
  const agent = 'Docs: https://viteplus.dev/guide/. Upstream: https://vitest.dev/guide/migration/';
  const rewritten = rewriteDocumentationLinks(agent);
  expect(rewritten).toBe(`Docs: ${origin}/guide/. Upstream: https://vitest.dev/guide/migration/`);
  expect(rewriteDocumentationLinks(rewritten)).toBe(rewritten);
});
