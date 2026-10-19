import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFile, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { promisify } from 'node:util';
import { gunzipSync } from 'node:zlib';

import { parseTriple } from '@napi-rs/cli';
import { parseTar } from 'nanotar';
import { parse } from 'yaml';

import pkg from './package.json' with { type: 'json' };

export const INSTALL_LOCKFILE = 'vp-install-lock.yaml';
// Keep in sync with the global installer in crates/vp_setup/src/install.rs.
export const INSTALL_PNPM_VERSION = '10.34.7';
export const PREVIEW_REGISTRY = 'https://registry-bridge.viteplus.dev';
const exec = promisify(execFile);

export function installManifest(version: string) {
  return {
    name: 'vp-global',
    version,
    private: true,
    packageManager: `pnpm@${INSTALL_PNPM_VERSION}`,
    dependencies: { 'vite-plus': version },
  };
}

export function validateInstallLockfile(content: string, version: string): void {
  const lock = parse(content) as {
    lockfileVersion?: string;
    importers?: Record<
      string,
      { dependencies?: Record<string, { specifier: string; version: string }> }
    >;
    packages?: Record<string, { resolution?: { integrity?: string; tarball?: string } }>;
  } | null;
  const cli = lock?.importers?.['.']?.dependencies?.['vite-plus'];
  const integrity = lock?.packages?.[`vite-plus@${version}`]?.resolution?.integrity;
  if (
    String(lock?.lockfileVersion) !== '9.0' ||
    cli?.specifier !== version ||
    (cli.version !== version && !cli.version?.startsWith(`${version}(`)) ||
    !integrity ||
    // Releases remain portable across mirrors. Previews need the bridge's
    // content-addressed URLs so republishing a commit cannot change their bytes.
    Object.entries(lock?.packages ?? {}).some(([key, entry]) => {
      const tarball = entry.resolution?.tarball;
      if (!tarball) {
        return false;
      }
      const name = key.slice(0, key.lastIndexOf('@'));
      const prefix = `${PREVIEW_REGISTRY}/tarballs/${name}/${version}/`;
      return (
        !/^0\.0\.0-commit\.[a-f0-9]{40}$/.test(version) ||
        key !== `${name}@${version}` ||
        !tarball.startsWith(prefix) ||
        !/^[a-f0-9]{40}\.tgz$/.test(tarball.slice(prefix.length))
      );
    })
  ) {
    throw new Error(`Invalid global installation lockfile for vite-plus@${version}`);
  }
}

/** Reuse the first published platform's lockfile after a partial release failure. */
export async function publishedInstallLockfile(
  version: string,
  registry: string,
  platformPackages: string[],
  fetchImpl: typeof fetch = fetch,
): Promise<string | undefined> {
  for (const name of platformPackages) {
    const response = await fetchImpl(`${registry.replace(/\/$/, '')}/${name}/${version}`, {
      signal: AbortSignal.timeout(30_000),
    });
    if (response.status === 404) {
      continue;
    }
    if (!response.ok) {
      throw new Error(`Could not check ${name}@${version}: HTTP ${response.status}`);
    }
    const metadata = (await response.json()) as {
      vpInstallLockfile?: boolean;
      dist: { tarball: string; integrity: string };
    };
    if (!metadata.vpInstallLockfile) {
      throw new Error(`${name}@${version} was published without an installation lockfile`);
    }
    const archive = await fetchImpl(metadata.dist.tarball, { signal: AbortSignal.timeout(60_000) });
    if (!archive.ok) {
      throw new Error(`Could not download ${name}@${version}: HTTP ${archive.status}`);
    }
    const bytes = Buffer.from(await archive.arrayBuffer());
    if (
      `sha512-${createHash('sha512').update(bytes).digest('base64')}` !== metadata.dist.integrity
    ) {
      throw new Error(`Integrity mismatch for ${name}@${version}`);
    }
    const files = parseTar(new Uint8Array(gunzipSync(bytes)));
    const lockfile = files.find((file) => file.name === `package/${INSTALL_LOCKFILE}`);
    if (!lockfile?.data) {
      throw new Error(`${name}@${version} is missing ${INSTALL_LOCKFILE}`);
    }
    const content = new TextDecoder().decode(lockfile.data);
    validateInstallLockfile(content, version);
    return content;
  }
  return undefined;
}

export async function generateInstallLockfile(
  version: string,
  registry: string,
  runPnpm = async (directory: string, args: string[]) => {
    await exec(
      'npm',
      ['exec', '--yes', `--package=pnpm@${INSTALL_PNPM_VERSION}`, '--', 'pnpm', ...args],
      {
        cwd: directory,
        env: { ...process.env, CI: 'true' },
        maxBuffer: 10 * 1024 * 1024,
      },
    );
  },
): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), 'vp-install-lock-'));
  try {
    await writeFile(join(directory, 'package.json'), JSON.stringify(installManifest(version)));
    await writeFile(join(directory, '.npmrc'), 'minimum-release-age=0\n');
    await runPnpm(directory, [
      'install',
      '--ignore-workspace',
      '--lockfile-only',
      '--ignore-scripts',
      '--no-frozen-lockfile',
      '--registry',
      registry,
    ]);
    const content = await readFile(join(directory, 'pnpm-lock.yaml'), 'utf8');
    validateInstallLockfile(content, version);
    return content;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

/** Add the same lockfile to the standalone archives before computing checksums. */
export async function addInstallLockfileToArchives(directory: string): Promise<void> {
  const lockfile = join(directory, INSTALL_LOCKFILE);
  for (const name of await readdir(directory)) {
    if (!name.startsWith('vp-')) {
      continue;
    }
    const archive = resolve(directory, name);
    if (name.endsWith('.zip')) {
      await exec('zip', ['-j', archive, lockfile]);
    } else if (name.endsWith('.tar.gz')) {
      const temporary = await mkdtemp(join(tmpdir(), 'vp-release-archive-'));
      try {
        await exec('tar', ['-xzf', archive, '-C', temporary]);
        await copyFile(lockfile, join(temporary, INSTALL_LOCKFILE));
        await exec('tar', ['-czf', archive, '-C', temporary, ...(await readdir(temporary))]);
      } finally {
        await rm(temporary, { recursive: true, force: true });
      }
    }
  }
}

async function main(): Promise<void> {
  const version = process.env.VERSION;
  const directory = resolve('binary-release');
  if (!version || version !== pkg.version) {
    throw new Error('VERSION must match packages/cli/package.json');
  }
  const registry = process.env.PUBLISH_REGISTRY ?? 'https://registry.npmjs.org';
  const platformPackages = pkg.napi.targets.map(
    (target) => `@voidzero-dev/vite-plus-cli-${parseTriple(target).platformArchABI}`,
  );
  let content: string | undefined;
  try {
    content = await readFile(join(directory, INSTALL_LOCKFILE), 'utf8');
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
      throw error;
    }
  }
  content ??=
    (await publishedInstallLockfile(version, registry, platformPackages)) ??
    (await generateInstallLockfile(version, registry));
  validateInstallLockfile(content, version);
  await writeFile(join(directory, INSTALL_LOCKFILE), content);
  await addInstallLockfileToArchives(directory);
  // eslint-disable-next-line no-console
  console.log(`Prepared ${basename(directory)}/${INSTALL_LOCKFILE} for vite-plus@${version}`);
}

const invokedPath = process.argv[1];
if (invokedPath && pathToFileURL(resolve(invokedPath)).href === import.meta.url) {
  main().catch((error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  });
}
