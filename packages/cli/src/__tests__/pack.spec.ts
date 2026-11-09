import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { expect, test } from 'vitest';

// `vite-plus/pack` re-exports the tsdown build bundled into core, so it must keep tsdown's runtime API.
test('exposes every tsdown value export from vite-plus/pack', async () => {
  const [pack, tsdown] = await Promise.all([import('../pack.js'), import('tsdown')]);
  const expected = Object.keys(tsdown).filter((key) => key !== 'default');
  expect(expected.length, 'sanity: tsdown should expose value exports').toBeGreaterThan(0);
  const missing = expected.filter((key) => (pack as Record<string, unknown>)[key] === undefined);
  expect(missing, 'tsdown value exports missing from vite-plus/pack').toEqual([]);
});

test('builds each workspace member in its own directory with member configs disabled', async ({
  onTestFinished,
}) => {
  const { build } = await import('../pack.js');
  const root = mkdtempSync(join(tmpdir(), 'vp-pack-workspace-'));
  onTestFinished(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(join(root, 'index.js'), 'export default 99');

  for (const [name, value] of Object.entries({ a: 11, b: 55 })) {
    const member = join(root, 'packages', name);
    mkdirSync(member, { recursive: true });
    writeFileSync(join(member, 'index.js'), `export default ${value}`);
    writeFileSync(join(member, 'tsdown.config.mjs'), 'throw new Error("member config loaded")');
  }

  await build({
    cwd: root,
    workspace: { include: 'packages/*', config: false },
    entry: ['index.js'],
    format: 'cjs',
    fixedExtension: true,
    dts: false,
  });

  const require = createRequire(join(root, 'package.json'));
  expect(require('./packages/a/dist/index.cjs')).toBe(11);
  expect(require('./packages/b/dist/index.cjs')).toBe(55);
  expect(existsSync(join(root, 'dist'))).toBe(false);
});
