import { readFile, readdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { createTarGzip, parseTarGzip } from 'nanotar';

import {
  generateInstallLockfile,
  INSTALL_LOCKFILE,
  isCommitPreviewVersion,
  PREVIEW_REGISTRY,
  publishedInstallLockfile,
  validateInstallLockfile,
} from './generate-install-lockfile.ts';
import { parseJsonObject } from './src/utils/json-value.ts';

const platformPrefix = '@voidzero-dev/vite-plus-cli-';
const decoder = new TextDecoder();

/** Resolve the published local CLI with pnpm, then bundle its lockfile with each global binary. */
export async function preparePreviewPackages(
  directory: string,
  version: string,
  resolveLockfile = generateInstallLockfile,
  recoverLockfile = publishedInstallLockfile,
): Promise<string> {
  if (!isCommitPreviewVersion(version)) {
    throw new Error('Expected a full commit preview version');
  }
  const archives = [];
  const names = new Set<string>();
  for (const file of (await readdir(directory)).filter((name) => /^pkg-\d+\.tgz$/.test(name))) {
    const entries = await parseTarGzip(new Uint8Array(await readFile(join(directory, file))));
    const manifest = entries.find((entry) => entry.name === 'package/package.json');
    if (!manifest?.data) {
      throw new Error(`${file} is missing package/package.json`);
    }
    const packageJson = parseJsonObject(decoder.decode(manifest.data));
    const { name } = packageJson;
    if (typeof name !== 'string' || !name.startsWith(platformPrefix) || names.has(name)) {
      throw new Error(
        `Expected unique global CLI platform packages, found ${JSON.stringify(name)}`,
      );
    }
    names.add(name);
    archives.push({ file, entries, manifest, packageJson });
  }
  if (archives.length === 0) {
    throw new Error('Preview batch is missing CLI platform packages');
  }

  // A new registration run can follow a partial publication of this commit.
  // Reuse its lockfile even when this run has no prepared-package artifact.
  const content =
    (await recoverLockfile(version, PREVIEW_REGISTRY, [...names])) ??
    (await resolveLockfile(version, PREVIEW_REGISTRY));
  const lock = validateInstallLockfile(content, version);
  if (Object.keys(lock.packages).some((name) => name.startsWith(platformPrefix))) {
    throw new Error(`Installation lockfile must not depend on ${platformPrefix} packages`);
  }

  for (const { file, entries, manifest, packageJson } of archives) {
    const files = packageJson.files ?? [];
    if (!Array.isArray(files) || !files.every((name) => typeof name === 'string')) {
      throw new TypeError(`Invalid files list in ${file}`);
    }
    manifest.data = new TextEncoder().encode(
      JSON.stringify({
        ...packageJson,
        vpInstallLockfile: true,
        files: [...new Set([...files, INSTALL_LOCKFILE])],
      }),
    );
    await writeFile(
      join(directory, file),
      await createTarGzip([
        ...entries.filter((entry) => entry.name !== `package/${INSTALL_LOCKFILE}`),
        { name: `package/${INSTALL_LOCKFILE}`, data: content },
      ]),
    );
  }
  // The bridge validates and normalizes these global packages during upload.
  // Their bytes are outside the locked local CLI dependency graph.
  return content;
}

async function main(): Promise<void> {
  const directory = process.argv[2];
  if (!directory || !process.env.VERSION) {
    throw new Error('Expected global CLI artifact directory and VERSION');
  }
  await preparePreviewPackages(resolve(directory), process.env.VERSION);
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  await main().catch((error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  });
}
