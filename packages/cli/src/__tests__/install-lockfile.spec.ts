import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';

import { createTarGzip } from 'nanotar';
import { describe, expect, it, vi } from 'vitest';

import {
  addInstallLockfileToArchives,
  generateInstallLockfile,
  INSTALL_LOCKFILE,
  installManifest,
  publishedInstallLockfile,
  validateInstallLockfile,
} from '../../generate-install-lockfile.ts';

const exec = promisify(execFile);
const version = '1.4.0';
const lockfile = `lockfileVersion: '9.0'
importers:
  .:
    dependencies:
      vite-plus:
        specifier: ${version}
        version: ${version}
packages:
  vite-plus@${version}:
    resolution: {integrity: sha512-fixture}
`;

describe('release installation lockfile', () => {
  it('generates an isolated global wrapper with the installer pnpm version', async () => {
    let temporary = '';
    const result = await generateInstallLockfile(
      version,
      'https://registry.example',
      async (directory, args, env) => {
        temporary = directory;
        expect(JSON.parse(await readFile(join(directory, 'package.json'), 'utf8'))).toEqual(
          installManifest(version),
        );
        expect(await readFile(join(directory, '.npmrc'), 'utf8')).toBe('minimum-release-age=0\n');
        expect(args).toEqual([
          'install',
          '--ignore-workspace',
          '--config.auto-install-peers=true',
          '--config.exclude-links-from-lockfile=false',
          '--config.dedupe-peers=false',
          '--config.inject-workspace-packages=false',
          '--lockfile-only',
          '--ignore-scripts',
          '--no-frozen-lockfile',
          '--registry',
          'https://registry.example',
        ]);
        expect(env).toEqual({
          npm_config_peers_suffix_max_length: '1000',
          NPM_CONFIG_PEERS_SUFFIX_MAX_LENGTH: '1000',
        });
        // A repository workspace lockfile cannot be reused for this wrapper.
        await expect(access(join(directory, 'pnpm-lock.yaml'))).rejects.toThrow();
        await writeFile(join(directory, 'pnpm-lock.yaml'), lockfile);
      },
    );
    expect(result).toBe(lockfile);
    await expect(access(temporary)).rejects.toThrow();
    const rust = await readFile(
      new URL('../../../../crates/vp_setup/src/install.rs', import.meta.url),
      'utf8',
    );
    expect(rust).toContain(
      `const PINNED_PNPM_VERSION: &str = "${installManifest(version).packageManager.slice(5)}";`,
    );
    for (const name of ['install-legacy.sh', 'install-legacy.ps1']) {
      const script = await readFile(new URL(`../../${name}`, import.meta.url), 'utf8');
      expect(script).toContain(`"${installManifest(version).packageManager}"`);
    }
  });

  it('cleans up and stops when resolution fails', async () => {
    let temporary = '';
    await expect(
      generateInstallLockfile(version, 'https://registry.example', async (directory) => {
        temporary = directory;
        throw new Error('registry unavailable');
      }),
    ).rejects.toThrow('registry unavailable');
    await expect(access(temporary)).rejects.toThrow();
  });

  it('rejects a lockfile for a different wrapper or without the CLI integrity', () => {
    expect(() => validateInstallLockfile(lockfile, version)).not.toThrow();
    expect(() => validateInstallLockfile(lockfile, '1.5.0')).toThrow(
      'Invalid global installation lockfile',
    );
    expect(() =>
      validateInstallLockfile(
        lockfile.replace('integrity: sha512-fixture', 'tarball: file:cli.tgz'),
        version,
      ),
    ).toThrow('Invalid global installation lockfile');
    expect(() =>
      validateInstallLockfile(
        lockfile.replace(
          'integrity: sha512-fixture',
          'integrity: sha512-fixture, tarball: http://localhost:1234/cli.tgz',
        ),
        version,
      ),
    ).toThrow('Invalid global installation lockfile');
    expect(() =>
      validateInstallLockfile(
        lockfile.replace(
          'integrity: sha512-fixture',
          'integrity: sha512-fixture, tarball: https://registry.npmjs.org/vite-plus/-/vite-plus-1.4.0.tgz',
        ),
        version,
      ),
    ).toThrow('Invalid global installation lockfile');
  });

  it('recovers the published lockfile after a partially completed release', async () => {
    const archive = Buffer.from(
      await createTarGzip([{ name: `package/${INSTALL_LOCKFILE}`, data: lockfile }]),
    );
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(null, { status: 404 }))
      .mockResolvedValueOnce(
        Response.json({
          vpInstallLockfile: true,
          dist: {
            tarball: 'https://registry.example/cli.tgz',
            integrity: `sha512-${createHash('sha512').update(archive).digest('base64')}`,
          },
        }),
      )
      .mockResolvedValueOnce(new Response(archive));
    await expect(
      publishedInstallLockfile(
        version,
        'https://registry.example/',
        ['@scope/first', '@scope/second'],
        fetchImpl,
      ),
    ).resolves.toBe(lockfile);
    expect(fetchImpl.mock.calls.map(([url]) => url)).toEqual([
      `https://registry.example/@scope/first/${version}`,
      `https://registry.example/@scope/second/${version}`,
      'https://registry.example/cli.tgz',
    ]);
  });

  it('does not generate a different lockfile on registry errors or corrupt published archives', async () => {
    await expect(
      publishedInstallLockfile(
        version,
        'https://registry.example',
        ['@scope/cli'],
        async () => new Response(null, { status: 503 }),
      ),
    ).rejects.toThrow('HTTP 503');
    await expect(
      publishedInstallLockfile(version, 'https://registry.example', ['@scope/cli'], async () =>
        Response.json({}),
      ),
    ).rejects.toThrow('published without an installation lockfile');
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        Response.json({
          vpInstallLockfile: true,
          dist: { tarball: 'https://registry.example/cli.tgz', integrity: 'sha512-expected' },
        }),
      )
      .mockResolvedValueOnce(new Response('modified archive'));
    await expect(
      publishedInstallLockfile(version, 'https://registry.example', ['@scope/cli'], fetchImpl),
    ).rejects.toThrow('Integrity mismatch');
  });

  it.runIf(process.platform !== 'win32')(
    'adds the same lockfile to tar and zip releases without losing binary permissions',
    async () => {
      const directory = await mkdtemp(join(tmpdir(), 'vp-release-lock-test-'));
      try {
        await writeFile(join(directory, 'vp'), '#!/bin/sh\n');
        await chmod(join(directory, 'vp'), 0o755);
        await writeFile(join(directory, INSTALL_LOCKFILE), lockfile);
        await exec('tar', ['-czf', join(directory, 'vp-linux.tar.gz'), '-C', directory, 'vp']);
        await exec('zip', ['-j', join(directory, 'vp-windows.zip'), join(directory, 'vp')]);
        await addInstallLockfileToArchives(directory);
        expect(
          (await exec('tar', ['-xOf', join(directory, 'vp-linux.tar.gz'), INSTALL_LOCKFILE]))
            .stdout,
        ).toBe(lockfile);
        expect(
          (await exec('unzip', ['-p', join(directory, 'vp-windows.zip'), INSTALL_LOCKFILE])).stdout,
        ).toBe(lockfile);
        expect((await exec('tar', ['-tvf', join(directory, 'vp-linux.tar.gz')])).stdout).toMatch(
          /-rwxr-xr-x.* vp/,
        );
      } finally {
        await rm(directory, { recursive: true, force: true });
      }
    },
  );
});
