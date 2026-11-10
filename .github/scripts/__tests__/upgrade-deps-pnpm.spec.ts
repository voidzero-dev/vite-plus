/// <reference types="node" />

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { expect, onTestFinished, test, vi } from 'vite-plus/test';

import { getPnpmInstallerIntegrity, updatePnpmInstallerPins } from '../upgrade-deps-utils.ts';

const digest = Buffer.alloc(64, 0xab);
const sri = `sha512-${digest.toString('base64')}`;
const integrity = `sha512.${digest.toString('hex')}`;

test('converts the official pnpm release integrity from SRI to packageManager hex', async () => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  const fetchMock = vi
    .fn()
    .mockResolvedValue(Response.json({ version: '10.40.2', dist: { integrity: sri } }));
  vi.stubGlobal('fetch', fetchMock);
  await expect(getPnpmInstallerIntegrity('10.40.2')).resolves.toBe(integrity);
  expect(fetchMock).toHaveBeenCalledExactlyOnceWith('https://registry.npmjs.org/pnpm/10.40.2');
});

test.each([
  undefined,
  null,
  123,
  '',
  `sha1-${digest.toString('base64')}`,
  integrity,
  'sha512-not-base64!',
  `sha512-${Buffer.alloc(32).toString('base64')}`,
  `sha512-${Buffer.alloc(65).toString('base64')}`,
  // The low four bits of the last base64 digit must be zero for a 64-byte digest.
  `sha512-${'A'.repeat(85)}B==`,
])('rejects missing or malformed pnpm SHA-512 integrity: %s', async (value) => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue(Response.json({ version: '10.40.2', dist: { integrity: value } })),
  );
  await expect(getPnpmInstallerIntegrity('10.40.2')).rejects.toThrow('dist.integrity');
});

test.each([
  {
    response: () => new Response('Unavailable', { status: 503, statusText: 'Unavailable' }),
    error: 'Failed to fetch pnpm 10.40.2: 503 Unavailable',
  },
  {
    response: () => Response.json({ version: '10.40.1', dist: { integrity: sri } }),
    error: 'version mismatch',
  },
  { response: () => Response.json(null), error: 'version mismatch' },
  { response: () => Response.json({ version: '10.40.2' }), error: 'dist.integrity' },
])('rejects unusable pnpm release metadata: $error', async ({ response, error }) => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response()));
  await expect(getPnpmInstallerIntegrity('10.40.2')).rejects.toThrow(error);
});

function createPinFixture() {
  const root = mkdtempSync(join(tmpdir(), 'vite-plus-pnpm-pins-'));
  onTestFinished(() => rmSync(root, { recursive: true, force: true }));
  const oldVersion = '10.34.7';
  const oldIntegrity = `sha512.${'01'.repeat(64)}`;
  const rustVersion = `const PINNED_PNPM_VERSION: &str = "${oldVersion}";`;
  const paths = [
    'crates/vp_setup/src/install.rs',
    'packages/cli/generate-install-lockfile.ts',
    'crates/vp_pm_cli/tests/package_manager_pin_verification.rs',
  ].map((file) => join(root, file));
  const contents = [
    `${rustVersion}\nconst PINNED_PNPM_INTEGRITY: &str = "${oldIntegrity}";\n`,
    `export const INSTALL_PNPM_VERSION = '${oldVersion}';\n`,
    `${rustVersion}\n`,
  ].map((content) => `// Keep surrounding content.\n${content}`);
  for (const [index, path] of paths.entries()) {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, contents[index]);
  }
  const read = () => paths.map((path) => readFileSync(path, 'utf8'));
  return { root, paths, contents, oldVersion, oldIntegrity, read };
}

test('updates all pnpm pins together and is idempotent', () => {
  const fixture = createPinFixture();
  expect(updatePnpmInstallerPins(fixture.root, '10.40.2', integrity)).toEqual({
    oldVersion: fixture.oldVersion,
    oldIntegrity: fixture.oldIntegrity,
  });
  const expected = fixture.contents.map((content) =>
    content.replace(fixture.oldVersion, '10.40.2').replace(fixture.oldIntegrity, integrity),
  );
  expect(fixture.read()).toEqual(expected);
  expect(updatePnpmInstallerPins(fixture.root, '10.40.2', integrity)).toEqual({
    oldVersion: '10.40.2',
    oldIntegrity: integrity,
  });
  expect(fixture.read()).toEqual(expected);
});

test('repairs integrity even when the selected pnpm version is unchanged', () => {
  const fixture = createPinFixture();
  updatePnpmInstallerPins(fixture.root, fixture.oldVersion, integrity);
  expect(fixture.read()).toEqual(
    fixture.contents.map((content) => content.replace(fixture.oldIntegrity, integrity)),
  );
});

test.each([
  { file: 0, pin: 'PINNED_PNPM_VERSION' },
  { file: 0, pin: 'PINNED_PNPM_INTEGRITY' },
  { file: 1, pin: 'INSTALL_PNPM_VERSION' },
  { file: 2, pin: 'PINNED_PNPM_VERSION' },
])('fails before writing when $pin in file $file is missing or duplicated', ({ file, pin }) => {
  const fixture = createPinFixture();
  for (const content of [
    fixture.contents[file].replace(pin, 'RENAMED_PIN'),
    fixture.contents[file] + fixture.contents[file],
  ]) {
    writeFileSync(fixture.paths[file], content);
    const before = fixture.read();
    expect(() => updatePnpmInstallerPins(fixture.root, '10.40.2', integrity)).toThrow(
      'Expected exactly one',
    );
    expect(fixture.read()).toEqual(before);
  }
});

test.each([1, 2])('fails before writing when pnpm version copy %s drifts', (file) => {
  const fixture = createPinFixture();
  writeFileSync(fixture.paths[file], fixture.contents[file].replace(fixture.oldVersion, '10.34.8'));
  const before = fixture.read();
  expect(() => updatePnpmInstallerPins(fixture.root, '10.40.2', integrity)).toThrow('out of sync');
  expect(fixture.read()).toEqual(before);
});
