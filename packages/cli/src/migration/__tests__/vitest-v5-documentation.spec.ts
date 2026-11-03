import fs from 'node:fs';
import path from 'node:path';

import { expect, it } from 'vitest';

import { documentationUrl } from '../../utils/documentation.ts';
import { formatVitestV5Findings } from '../migrator.ts';
import { localSections, vitestV5Documentation } from '../vitest-v5/documentation.ts';

it.each([
  ['benchmark-api', 'https://vitest.dev/guide/migration/#benchmarking-api-rewrite'],
  ['node-runtime', 'https://viteplus.dev/guide/vitest-v5#node-runtime'],
  ['source-version', 'https://viteplus.dev/guide/vitest-v5#before-you-migrate'],
  ['global-api-ownership', 'https://viteplus.dev/guide/vitest-v5#resolve-migration-findings'],
  [
    'unawaited-assertion',
    'https://vitest.dev/guide/migration/#unawaited-asynchronous-assertions-fail-the-test',
  ],
  ['future-finding', 'https://viteplus.dev/guide/vitest-v5#resolve-migration-findings'],
])('links %s to its documentation', (code, url) => {
  expect(vitestV5Documentation(code)).toBe(url);
});

it('links every block and review, retaining locations and deduplication', () => {
  const rootDir = process.cwd();
  const block = {
    file: path.join(rootDir, 'example.bench.ts'),
    line: 3,
    column: 5,
    code: 'benchmark-api',
    severity: 'block' as const,
    message: 'Review the benchmark.',
  };
  const review = {
    ...block,
    line: 7,
    code: 'future-finding',
    severity: 'review' as const,
    message: 'Review the configuration.',
  };
  const report = formatVitestV5Findings({ rootDir, findings: [block, review, block] });
  expect(report).toContain('Vitest v5: 2 review items (1 block dependency updates)');
  expect(report).toContain(
    `3:5 BLOCK [benchmark-api] Review the benchmark.\n    Docs: ${vitestV5Documentation('benchmark-api')}`,
  );
  expect(report).toContain(
    `7:5 REVIEW [future-finding] Review the configuration.\n    Docs: ${vitestV5Documentation('future-finding')}`,
  );
  expect(report.match(/Docs:/g)).toHaveLength(2);
  expect(formatVitestV5Findings({ rootDir, findings: [] })).toBe('');
});

it('keeps local documentation anchors valid', () => {
  const guide = fs.readFileSync(
    new URL('../../../../../docs/guide/vitest-v5.md', import.meta.url),
    'utf8',
  );
  const headings = Array.from(guide.matchAll(/^## (.+)$/gm), (match) =>
    match[1].toLowerCase().replaceAll(' ', '-'),
  );
  // `future-finding` has no entry and exercises the fallback anchor.
  for (const code of [...Object.keys(localSections), 'future-finding']) {
    const [page, anchor] = vitestV5Documentation(code).split('#');
    expect(page).toBe(documentationUrl('/guide/vitest-v5'));
    expect(headings).toContain(anchor);
  }
});
