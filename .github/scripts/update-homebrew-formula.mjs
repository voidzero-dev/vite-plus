import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { waitForNpmPackagesFromEnv } from './wait-for-npm-packages.ts';

export const targets = [
  ['macos', 'arm', 'aarch64-apple-darwin', 'darwin-arm64'],
  ['macos', 'intel', 'x86_64-apple-darwin', 'darwin-x64'],
  ['linux', 'arm', 'aarch64-unknown-linux-gnu', 'linux-arm64-gnu'],
  ['linux', 'intel', 'x86_64-unknown-linux-gnu', 'linux-x64-gnu'],
];
const repository = 'voidzero-dev/vite-plus';
const assetBlock = /  # BEGIN RELEASE ASSETS\n[\s\S]*?  # END RELEASE ASSETS/;
const launchGate = /  # The updater removes this gate[^\n]*\n  disable! [^\n]*\n\n/;

function stableVersion(version) {
  assert.match(
    version,
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/,
    'Expected a stable release version',
  );
  return version.split('.').map(Number);
}

export function releaseAssets(version, checksums) {
  stableVersion(version);
  return targets.map(([os, arch, target]) => {
    const name = `vp-${target}.tar.gz`;
    const entries = checksums
      .trim()
      .split('\n')
      .filter((line) => line.trim().split(/\s+/)[1] === name);
    assert.equal(entries.length, 1, `Expected one checksum for ${name}`);
    const sha256 = entries[0].split(/\s+/)[0];
    assert.match(sha256, /^[a-f0-9]{64}$/, `Invalid checksum for ${name}`);
    return {
      os,
      arch,
      name,
      sha256,
      url: `https://github.com/${repository}/releases/download/v${version}/${name}`,
    };
  });
}

export function updateFormula(source, version, assets) {
  const next = stableVersion(version);
  const currentVersion = source.match(/\/releases\/download\/v(\d+\.\d+\.\d+)\//)?.[1];
  const current = stableVersion(currentVersion ?? '');
  const difference = next.map((part, i) => part - current[i]).find((part) => part !== 0) ?? 0;
  assert.ok(difference >= 0, 'Refusing to downgrade the formula');
  assert.equal(assets.length, targets.length, 'Expected all four release targets');
  assert.equal(
    source.match(new RegExp(assetBlock.source, 'g'))?.length,
    1,
    'Missing release asset block',
  );
  const blocks = ['macos', 'linux']
    .map(
      (os) =>
        `  on_${os} do\n${assets
          .filter((asset) => asset.os === os)
          .map((asset) => {
            assert.ok(targets.some(([targetOs, arch]) => targetOs === os && arch === asset.arch));
            assert.match(asset.sha256, /^[a-f0-9]{64}$/);
            // URLs are supplied by the release validator, or a local e2e fixture.
            assert.ok(!/["\\\n\r#]/.test(asset.url), 'Unsafe formula URL');
            return `    on_${asset.arch} do\n      url "${asset.url}"\n      sha256 "${asset.sha256}"\n    end`;
          })
          .join('\n')}\n  end`,
    )
    .join('\n\n');
  let updated = source
    .replace(assetBlock, `  # BEGIN RELEASE ASSETS\n${blocks}\n  # END RELEASE ASSETS`)
    .replace(launchGate, '');
  if (difference > 0) updated = updated.replace(/^  revision \d+\n/m, '');
  return updated;
}

export async function verifyRelease(version, release, fetchImpl = fetch) {
  stableVersion(version);
  assert.equal(release.tag_name, `v${version}`);
  assert.ok(!release.draft && !release.prerelease, 'Release must be published and stable');
  const get = async (url) => {
    const response = await fetchImpl(url, { signal: AbortSignal.timeout(120_000) });
    assert.ok(response.ok, `Download failed: ${url} (HTTP ${response.status})`);
    return response;
  };
  // Old tags do not contain the authenticated bootstrap or the bare Homebrew runtime.
  // Check the release tag, never main, before enabling installation.
  await get(
    `https://raw.githubusercontent.com/${repository}/v${version}/crates/vp_pm_cli/src/package_manager/bootstrap.rs`,
  );
  await get(`https://raw.githubusercontent.com/${repository}/v${version}/HomebrewFormula/vp.rb`);
  const checksumUrl = `https://github.com/${repository}/releases/download/v${version}/vp-checksums.txt`;
  const assets = releaseAssets(version, await (await get(checksumUrl)).text());
  for (const asset of assets) {
    assert.ok(
      release.assets.some(
        (published) =>
          published.name === asset.name && published.browser_download_url === asset.url,
      ),
      `Missing release asset: ${asset.name}`,
    );
    const data = await (await get(asset.url)).arrayBuffer();
    assert.equal(
      createHash('sha256').update(Buffer.from(data)).digest('hex'),
      asset.sha256,
      `Checksum mismatch: ${asset.name}`,
    );
  }
  return assets;
}

async function main() {
  const [version] = process.argv.slice(2);
  stableVersion(version ?? '');
  const release = JSON.parse(
    execFileSync('gh', ['api', `repos/${repository}/releases/tags/v${version}`], {
      encoding: 'utf8',
    }),
  );
  const assets = await verifyRelease(version, release);
  const packages = [
    'vite-plus',
    '@voidzero-dev/vite-plus-core',
    '@voidzero-dev/vite-plus-prompts',
    ...targets.map(([, , , platform]) => `@voidzero-dev/vite-plus-${platform}`),
  ];
  await waitForNpmPackagesFromEnv(packages.map((name) => ({ name, version })));
  const file = new URL('../../HomebrewFormula/vp.rb', import.meta.url);
  const source = await readFile(file, 'utf8');
  const result = updateFormula(source, version, assets);
  const guide = new URL('../../docs/guide/homebrew.md', import.meta.url);
  const guideSource = await readFile(guide, 'utf8');
  const guideResult = guideSource
    .replace(/::: warning Availability\n[\s\S]*?:::\n\n/, '')
    .replace('When the tap becomes available, run:', 'Install the tap:');
  if (result !== source) await writeFile(file, result);
  if (guideResult !== guideSource) await writeFile(guide, guideResult);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href)
  await main();
