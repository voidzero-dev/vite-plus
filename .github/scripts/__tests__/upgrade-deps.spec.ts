/// <reference types="node" />

import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { expect, test, vi } from 'vitest';

test('upgrades lint-staged across major versions and includes it in upgrade metadata', async ({
  onTestFinished,
}) => {
  const root = resolve(import.meta.dirname, '../../..');
  const tempDir = mkdtempSync(join(tmpdir(), 'vite-plus-upgrade-deps-'));
  const metaDir = join(tempDir, 'meta');
  onTestFinished(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
    vi.resetModules();
    rmSync(tempDir, { recursive: true, force: true });
  });

  for (const file of [
    'pnpm-workspace.yaml',
    'packages/tools/.upstream-versions.json',
    'packages/cli/src/utils/constants.ts',
  ]) {
    mkdirSync(dirname(join(tempDir, file)), { recursive: true });
    copyFileSync(join(root, file), join(tempDir, file));
  }
  const workspacePath = join(tempDir, 'pnpm-workspace.yaml');
  writeFileSync(
    workspacePath,
    readFileSync(workspacePath, 'utf8').replace(/  lint-staged: .+/, '  lint-staged: ^16.2.6'),
  );

  vi.spyOn(process, 'cwd').mockReturnValue(tempDir);
  vi.spyOn(console, 'log').mockImplementation(() => {});
  vi.stubEnv('UPGRADE_DEPS_META_DIR', metaDir);
  vi.stubGlobal('fetch', async (url: string) => {
    if (url.startsWith('https://api.github.com/repos/')) {
      return Response.json([{ name: 'v1.2.3', commit: { sha: 'a'.repeat(40) } }]);
    }
    if (url === 'https://registry.npmjs.org/vitest') {
      return Response.json({ versions: { '4.1.11': {}, '5.0.0': {} } });
    }
    if (url === 'https://registry.npmjs.org/@tsdown/css/latest') {
      return Response.json({ dependencies: { lightningcss: '^1.33.0' } });
    }
    if (url === 'https://registry.npmjs.org/lint-staged/latest') {
      return Response.json({ version: '17.5.1' });
    }
    if (url.startsWith('https://registry.npmjs.org/') && url.endsWith('/latest')) {
      return Response.json({ version: '1.2.3' });
    }
    throw new Error(`Unexpected request: ${url}`);
  });

  vi.resetModules();
  await import('../upgrade-deps.ts');

  expect(readFileSync(workspacePath, 'utf8')).toContain('\n  lint-staged: ^17.5.1\n');
  const versions = JSON.parse(readFileSync(join(metaDir, 'versions.json'), 'utf8'));
  expect(versions['lint-staged']).toEqual({ old: '16.2.6', new: '17.5.1' });
  expect(readFileSync(join(metaDir, 'commit-message.txt'), 'utf8')).toContain(
    '- lint-staged: 16.2.6 -> 17.5.1',
  );
  expect(readFileSync(join(metaDir, 'pr-body.md'), 'utf8')).toContain(
    '| `lint-staged` | `16.2.6` | `17.5.1` |',
  );
});
