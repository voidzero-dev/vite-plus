import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { createTarGzip, parseTarGzip } from 'nanotar';
import { describe, expect, it, vi } from 'vitest';
import { stringify } from 'yaml';

import {
  INSTALL_LOCKFILE,
  PREVIEW_REGISTRY,
  validateInstallLockfile,
} from '../../generate-install-lockfile.ts';
import { preparePreviewPackages } from '../../generate-preview-install-lockfile.ts';

const version = '0.0.0-commit.0123456789abcdef0123456789abcdef01234567';
const names = [
  '@voidzero-dev/vite-plus-cli-linux-x64-gnu',
  '@voidzero-dev/vite-plus-cli-win32-arm64-msvc',
];
const decoder = new TextDecoder();
const noPublishedLockfile = async () => undefined;
const shasum = 'a'.repeat(40);
const lockfile = stringify({
  lockfileVersion: '9.0',
  importers: { '.': { dependencies: { 'vite-plus': { specifier: version, version } } } },
  packages: {
    [`vite-plus@${version}`]: {
      resolution: {
        integrity: 'sha512-published-cli',
        tarball: `${PREVIEW_REGISTRY}/tarballs/vite-plus/${version}/${shasum}.tgz`,
      },
    },
    // Pnpm keeps these URLs because the bridge proxies npm package metadata.
    'sirv@3.0.2': {
      resolution: {
        integrity: 'sha512-published-sirv',
        tarball: 'https://registry.npmjs.org/sirv/-/sirv-3.0.2.tgz',
      },
    },
    '@babel/code-frame@7.29.7': {
      resolution: {
        integrity: 'sha512-published-code-frame',
        tarball: 'https://registry.npmjs.org/@babel/code-frame/-/code-frame-7.29.7.tgz',
      },
    },
  },
});

async function withBatch(run: (directory: string) => Promise<void>) {
  const directory = await mkdtemp(join(tmpdir(), 'vp-preview-test-'));
  try {
    for (const [index, name] of names.entries()) {
      await writeFile(
        join(directory, `pkg-${index}.tgz`),
        await createTarGzip([
          { name: 'package/package.json', data: JSON.stringify({ name, version, files: ['vp'] }) },
          { name: 'package/vp', data: 'binary', attrs: { mode: '755' } },
        ]),
      );
    }
    await run(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

describe('preview installation lockfile', () => {
  it('resolves the published commit version and bundles the unchanged pnpm lockfile', async () => {
    await withBatch(async (directory) => {
      const resolveLockfile = vi.fn(async () => lockfile);
      expect(
        await preparePreviewPackages(directory, version, resolveLockfile, noPublishedLockfile),
      ).toBe(lockfile);
      expect(resolveLockfile).toHaveBeenCalledExactlyOnceWith(version, PREVIEW_REGISTRY);
      expect(await readdir(directory)).toEqual(['pkg-0.tgz', 'pkg-1.tgz']);
      for (const index of [0, 1]) {
        const entries = await parseTarGzip(
          new Uint8Array(await readFile(join(directory, `pkg-${index}.tgz`))),
        );
        const manifest = JSON.parse(
          decoder.decode(entries.find((entry) => entry.name === 'package/package.json')!.data),
        );
        expect(manifest.vpInstallLockfile).toBe(true);
        expect(manifest.files).toEqual(['vp', INSTALL_LOCKFILE]);
        expect(
          decoder.decode(
            entries.find((entry) => entry.name === `package/${INSTALL_LOCKFILE}`)!.data,
          ),
        ).toBe(lockfile);
        const binary = entries.find((entry) => entry.name === 'package/vp')!;
        expect(decoder.decode(binary.data)).toBe('binary');
        expect(Number.parseInt(binary.attrs!.mode!, 8)).toBe(0o755);
      }
    });
  });

  it('leaves the archives unchanged when the published local CLI cannot be resolved', async () => {
    await withBatch(async (directory) => {
      const before = await readFile(join(directory, 'pkg-0.tgz'));
      await expect(
        preparePreviewPackages(
          directory,
          version,
          async () => {
            throw new Error('local CLI is not published');
          },
          noPublishedLockfile,
        ),
      ).rejects.toThrow('local CLI is not published');
      expect(await readFile(join(directory, 'pkg-0.tgz'))).toEqual(before);
    });
  });

  it('does not repack local CLI dependency archives', async () => {
    await withBatch(async (directory) => {
      const archive = await createTarGzip([
        { name: 'package/package.json', data: JSON.stringify({ name: 'vite-plus', version }) },
      ]);
      await writeFile(join(directory, 'pkg-2.tgz'), archive);
      const resolveLockfile = vi.fn(async () => lockfile);
      await expect(
        preparePreviewPackages(directory, version, resolveLockfile, noPublishedLockfile),
      ).rejects.toThrow('Expected unique global CLI platform packages');
      expect(resolveLockfile).not.toHaveBeenCalled();
      expect(new Uint8Array(await readFile(join(directory, 'pkg-2.tgz')))).toEqual(archive);
    });
  });

  it('rejects a circular hash dependency on the binary package carrying the lockfile', async () => {
    await withBatch(async (directory) => {
      const circular = lockfile + `  "${names[0]}@${version}": {}\n`;
      await expect(
        preparePreviewPackages(directory, version, async () => circular, noPublishedLockfile),
      ).rejects.toThrow('must not depend on @voidzero-dev/vite-plus-cli-');
    });
  });

  it('replaces an existing companion without duplicate archive or manifest entries', async () => {
    await withBatch(async (directory) => {
      await preparePreviewPackages(directory, version, async () => lockfile, noPublishedLockfile);
      await preparePreviewPackages(directory, version, async () => lockfile, noPublishedLockfile);
      const entries = await parseTarGzip(
        new Uint8Array(await readFile(join(directory, 'pkg-0.tgz'))),
      );
      expect(entries.filter((entry) => entry.name === `package/${INSTALL_LOCKFILE}`)).toHaveLength(
        1,
      );
      const manifest = JSON.parse(
        decoder.decode(entries.find((entry) => entry.name === 'package/package.json')!.data),
      );
      expect(manifest.files).toEqual(['vp', INSTALL_LOCKFILE]);
    });
  });

  it('reuses the published lockfile after a partial publication without a saved artifact', async () => {
    await withBatch(async (directory) => {
      const resolveLockfile = vi.fn(async () => lockfile.replace('published-cli', 'new-cli'));
      const recoverLockfile = vi.fn(async () => lockfile);
      expect(
        await preparePreviewPackages(directory, version, resolveLockfile, recoverLockfile),
      ).toBe(lockfile);
      expect(recoverLockfile).toHaveBeenCalledExactlyOnceWith(version, PREVIEW_REGISTRY, names);
      expect(resolveLockfile).not.toHaveBeenCalled();
      for (const file of await readdir(directory)) {
        const entries = await parseTarGzip(new Uint8Array(await readFile(join(directory, file))));
        expect(
          decoder.decode(
            entries.find((entry) => entry.name === `package/${INSTALL_LOCKFILE}`)!.data,
          ),
        ).toBe(lockfile);
      }
    });
  });

  it('does not resolve again when recovery of an existing publication fails', async () => {
    await withBatch(async (directory) => {
      const before = await readFile(join(directory, 'pkg-0.tgz'));
      const resolveLockfile = vi.fn(async () => lockfile);
      await expect(
        preparePreviewPackages(directory, version, resolveLockfile, async () => {
          throw new Error('registry unavailable');
        }),
      ).rejects.toThrow('registry unavailable');
      expect(resolveLockfile).not.toHaveBeenCalled();
      expect(await readFile(join(directory, 'pkg-0.tgz'))).toEqual(before);
    });
  });

  it('accepts npm dependencies while keeping preview packages on content-addressed bridge URLs', () => {
    expect(() => validateInstallLockfile(lockfile, version)).not.toThrow();
    for (const changed of [
      lockfile.replace(PREVIEW_REGISTRY, 'http://localhost:1234'),
      lockfile.replace(`/tarballs/vite-plus/${version}/`, '/tarballs/vite-plus/0.0.0-commit.old/'),
      lockfile.replace(`/${shasum}.tgz`, '.tgz'),
      lockfile.replace(PREVIEW_REGISTRY, 'https://registry.npmjs.org'),
      lockfile.replace('https://registry.npmjs.org/', 'http://localhost:1234/'),
      lockfile.replace('https://registry.npmjs.org/', 'https://registry.npmjs.org.example/'),
      lockfile.replace('integrity: sha512-published-sirv', 'integrity: ""'),
    ]) {
      expect(() => validateInstallLockfile(changed, version)).toThrow(
        'Invalid global installation lockfile',
      );
    }
  });
});
