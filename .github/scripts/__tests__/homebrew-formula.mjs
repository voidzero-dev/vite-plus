import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

import {
  releaseAssets,
  targets,
  updateFormula,
  verifyRelease,
} from '../update-homebrew-formula.mjs';

const formula = await readFile(new URL('../../../HomebrewFormula/vp.rb', import.meta.url), 'utf8');
const version = '0.3.4';
const archive = Buffer.from('release fixture');
const digest = createHash('sha256').update(archive).digest('hex');
const checksums = targets.map(([, , target]) => `${digest}  vp-${target}.tar.gz`).join('\n');
const assets = releaseAssets(version, checksums);
const release = {
  tag_name: `v${version}`,
  draft: false,
  prerelease: false,
  assets: assets.map((asset) => ({ name: asset.name, browser_download_url: asset.url })),
};

function download(url) {
  return Promise.resolve(new Response(url.endsWith('vp-checksums.txt') ? checksums : archive));
}

await test('release publication requires the compatible tag and each verified archive', async () => {
  assert.deepEqual(await verifyRelease(version, release, download), assets);
  await assert.rejects(verifyRelease(version, { ...release, draft: true }, download), /published/);
  await assert.rejects(
    verifyRelease(version, { ...release, assets: release.assets.slice(1) }, download),
    /Missing release asset/,
  );
  await assert.rejects(
    verifyRelease(version, release, async (url) =>
      url.includes('bootstrap.rs') ? new Response('', { status: 404 }) : download(url),
    ),
    /Download failed/,
  );
  await assert.rejects(
    verifyRelease(version, release, async (url) =>
      url.endsWith('.tar.gz') ? new Response('corrupt') : download(url),
    ),
    /Checksum mismatch/,
  );
});

await test('missing, repeated, and invalid checksums cannot update a formula', () => {
  assert.throws(
    () => releaseAssets(version, checksums.split('\n').slice(1).join('\n')),
    /Expected one checksum/,
  );
  assert.throws(
    () => releaseAssets(version, `${checksums}\n${checksums}`),
    /Expected one checksum/,
  );
  assert.throws(
    () => releaseAssets(version, checksums.replace(digest, 'invalid')),
    /Invalid checksum/,
  );
  assert.throws(() => releaseAssets('0.3.4-beta.1', checksums), /stable release/);
});

await test('updates preserve recipe changes and reset only a previous version revision', () => {
  const revised = formula.replace('  license', '  revision 2\n  license');
  const result = updateFormula(revised, version, assets);
  assert.ok(!result.includes('disable!'));
  assert.ok(!result.includes('revision 2'));
  assert.ok(result.includes('conflicts_with "vite-plus"'));
  assert.ok(result.includes('  def install\n    bin.install "vp"'));
  const sameVersion = result.replace('  license', '  revision 1\n  license');
  assert.equal(updateFormula(sameVersion, version, assets), sameVersion);
  assert.throws(() => updateFormula(result, '0.3.3', assets), /downgrade/);
  assert.throws(
    () => updateFormula(formula.replace('BEGIN RELEASE ASSETS', 'missing'), version, assets),
    /Missing release asset block/,
  );
  assert.throws(
    () =>
      updateFormula(formula, version, [
        { ...assets[0], url: 'https://example.test/"; system("bad")' },
        ...assets.slice(1),
      ]),
    /Unsafe formula URL/,
  );
});
