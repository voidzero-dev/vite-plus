import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { resolveDocsSiteOrigin } from '../../../docs/.vitepress/site-origin.ts';
import { previewUrl } from '../docs-fork-preview.mjs';
import { previewDocsOrigin } from '../preview-docs-origin.ts';

const pr = {
  number: 2551,
  head: { ref: 'rfc/vitest-v5-upgrade', repo: { full_name: 'voidzero-dev/vite-plus' } },
  base: { repo: { full_name: 'voidzero-dev/vite-plus' } },
};

const read = (file) => readFileSync(new URL(file, import.meta.url), 'utf8');

await test('uses the head branch docs for same-repo and stacked PRs', () => {
  assert.equal(
    previewDocsOrigin(pr),
    'https://rfc-vitest-v5-upgrade-viteplus-dev.voidzero-docs.workers.dev',
  );
  for (const branch of ['--RFC//Vitest_V5-Upgrade--', `feature/${'a'.repeat(80)}`]) {
    assert.equal(
      previewDocsOrigin({ ...pr, head: { ...pr.head, ref: branch } }),
      resolveDocsSiteOrigin({ WORKERS_CI: '1', WORKERS_CI_BRANCH: branch }),
    );
  }
});

await test('uses the PR alias for forks, never a same-named repository branch', () => {
  assert.equal(
    previewDocsOrigin({
      ...pr,
      head: { ...pr.head, repo: { full_name: 'contributor/vite-plus' } },
    }),
    previewUrl(pr.number),
  );
});

await test('passes the origin to native builds and includes it in the native cache key', () => {
  const preview = read('../../workflows/publish-preview.yml');
  const release = read('../../workflows/reusable-release-build.yml');
  const build = read('../../actions/build-upstream/action.yml');
  assert.match(preview, /previewDocsOrigin\(context\.payload\.pull_request\)/);
  assert.match(preview, /docs-origin: \$\{\{ steps\.docs\.outputs\.origin \}\}/);
  assert.match(preview, /docs-origin: \$\{\{ needs\.prepare\.outputs\.docs-origin \}\}/);
  assert.match(release, /default: 'https:\/\/viteplus\.dev'/);
  assert.match(release, /VITE_PLUS_DOCS_ORIGIN: \$\{\{ inputs\.docs-origin \}\}/);
  assert.match(build, /process\.env\.VITE_PLUS_DOCS_ORIGIN/);
  assert.match(build, /napi-binding-v3-.*\$\{DOCS_ORIGIN_HASH\}/);
});
