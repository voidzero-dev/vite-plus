import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { createTarGzip, parseTarGzip } from 'nanotar';
import { describe, expect, it } from 'vitest';
import { parse, stringify } from 'yaml';

import {
  INSTALL_LOCKFILE,
  PREVIEW_REGISTRY,
  validateInstallLockfile,
} from '../../generate-install-lockfile.ts';
import { preparePreviewPackages } from '../../generate-preview-install-lockfile.ts';

const version = '0.0.0-commit.0123456789abcdef0123456789abcdef01234567';
const names = [
  'vite-plus',
  '@voidzero-dev/vite-plus-core',
  '@voidzero-dev/vite-plus-cli-linux-x64-gnu',
  '@voidzero-dev/vite-plus-cli-win32-arm64-msvc',
];
const decoder = new TextDecoder();

// Identity builder: the fixture archives already carry the preview version.
// The workflow uses the bridge's real builder, including its manifest rewrite.
async function preparedPackage(tarball: Uint8Array) {
  const entries = await parseTarGzip(tarball);
  const packageJson = JSON.parse(
    decoder.decode(entries.find((entry) => entry.name === 'package/package.json')!.data),
  );
  return {
    tarball,
    packageJson,
    integrity: `sha512-${createHash('sha512').update(tarball).digest('base64')}`,
    shasum: createHash('sha1').update(tarball).digest('hex'),
  };
}

function installLock(packages: Awaited<ReturnType<typeof preparedPackage>>[]) {
  return stringify({
    lockfileVersion: '9.0',
    importers: { '.': { dependencies: { 'vite-plus': { specifier: version, version } } } },
    packages: Object.fromEntries(
      packages.map((pkg) => [
        `${pkg.packageJson.name}@${version}`,
        {
          resolution: {
            integrity: pkg.integrity,
            tarball: `${PREVIEW_REGISTRY}/tarballs/${pkg.packageJson.name}/${version}/${pkg.shasum}.tgz`,
          },
        },
      ]),
    ),
  });
}

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
  it('includes the same lock in every binary package without changing dependency archives', async () => {
    await withBatch(async (directory) => {
      const cliBefore = await readFile(join(directory, 'pkg-0.tgz'));
      const content = await preparePreviewPackages(
        directory,
        version,
        preparedPackage,
        async (_, packages) => installLock(packages.slice(0, 2)),
      );
      expect(await readdir(directory)).toEqual(names.map((_, index) => `pkg-${index}.tgz`));
      validateInstallLockfile(content, version, cliBefore);
      expect(await readFile(join(directory, 'pkg-0.tgz'))).toEqual(cliBefore);
      for (const index of [2, 3]) {
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
        ).toBe(content);
        expect(
          Number.parseInt(entries.find((entry) => entry.name === 'package/vp')!.attrs!.mode!, 8),
        ).toBe(0o755);
      }
    });
  });

  it('rejects a circular hash dependency on the binary package carrying the lockfile', async () => {
    await withBatch(async (directory) => {
      await expect(
        preparePreviewPackages(directory, version, preparedPackage, async (_, packages) =>
          installLock(packages),
        ),
      ).rejects.toThrow('must not depend on @voidzero-dev/vite-plus-cli-');
    });
  });

  it('rejects stale dependency hashes and URLs from a different preview build', async () => {
    await withBatch(async (directory) => {
      for (const field of ['integrity', 'shasum'] as const) {
        await expect(
          preparePreviewPackages(directory, version, preparedPackage, async (_, packages) => {
            const core = packages[1];
            return installLock([
              packages[0],
              { ...core, [field]: field === 'shasum' ? 'f'.repeat(40) : 'sha512-stale' },
            ]);
          }),
        ).rejects.toThrow('does not match the preview archive');
      }
    });
  });

  it('refuses an archive builder that changes a locked package on the publishing rebuild', async () => {
    await withBatch(async (directory) => {
      let calls = 0;
      await expect(
        preparePreviewPackages(
          directory,
          version,
          async (tarball) => {
            const pkg = await preparedPackage(tarball);
            if (++calls > names.length) {
              pkg.integrity = 'sha512-changed';
            }
            return pkg;
          },
          async (_, packages) => installLock(packages.slice(0, 2)),
        ),
      ).rejects.toThrow('archive rebuild changed the integrity');
    });
  });

  it('accepts only content-addressed bridge URLs for the exact preview version', async () => {
    const pkg = await preparedPackage(
      await createTarGzip([
        { name: 'package/package.json', data: JSON.stringify({ name: 'vite-plus', version }) },
      ]),
    );
    const content = installLock([pkg]);
    expect(() => validateInstallLockfile(content, version)).not.toThrow();
    for (const changed of [
      content.replace(PREVIEW_REGISTRY, 'http://localhost:1234'),
      content.replace(`/tarballs/vite-plus/${version}/`, '/tarballs/vite-plus/0.0.0-commit.old/'),
      content.replace(`/${pkg.shasum}.tgz`, '.tgz'),
    ]) {
      expect(() => validateInstallLockfile(changed, version)).toThrow(
        'Invalid global installation lockfile',
      );
    }
  });

  it('keeps the archive builder pinned to both publishing action invocations', async () => {
    const build = parse(
      await readFile(
        new URL('../../../../.github/workflows/publish-preview.yml', import.meta.url),
        'utf8',
      ),
    );
    const publish = parse(
      await readFile(
        new URL('../../../../.github/workflows/publish-preview-register.yml', import.meta.url),
        'utf8',
      ),
    );
    const steps = build.jobs.publish.steps as {
      uses?: string;
      with?: { repository?: string; ref?: string };
    }[];
    const checkout = steps.find(
      (step) => step.with?.repository === 'voidzero-dev/pkg-pr-registry-bridge',
    )!;
    const action = `voidzero-dev/pkg-pr-registry-bridge@${checkout.with!.ref}`;
    expect(checkout.with!.ref).toMatch(/^[a-f0-9]{40}$/);
    expect(steps.some((step) => step.uses === action)).toBe(true);
    expect(publish.jobs.publish.steps.some((step: { uses?: string }) => step.uses === action)).toBe(
      true,
    );
  });
});
