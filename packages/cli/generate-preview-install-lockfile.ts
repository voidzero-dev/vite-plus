import { execFile } from 'node:child_process';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

import { createTarGzip, parseTarGzip } from 'nanotar';
import { build } from 'vite/rolldown';
import { parse, parseAllDocuments } from 'yaml';

import {
  generateInstallLockfile,
  INSTALL_LOCKFILE,
  PREVIEW_REGISTRY,
  validateInstallLockfile,
} from './generate-install-lockfile.ts';

interface PreviewPackage {
  tarball: Uint8Array;
  packageJson: { name: string; version: string; [key: string]: unknown };
  integrity: string;
  shasum: string;
}

type BuildPreviewTarball = (
  tarball: Uint8Array,
  name: string,
  version: string,
  batch: ReadonlySet<string>,
) => Promise<PreviewPackage>;

const exec = promisify(execFile);
const platformPrefix = '@voidzero-dev/vite-plus-cli-';
const decoder = new TextDecoder();

/** Use the same pinned implementation as the publishing action, without its credentials. */
export async function loadPreviewBuilder(source: string): Promise<BuildPreviewTarball> {
  const bridgeLocks = parseAllDocuments(
    await readFile(resolve(dirname(source), '../../pnpm-lock.yaml'), 'utf8'),
  );
  const nanotarPath = fileURLToPath(import.meta.resolve('nanotar'));
  const nanotarManifest = JSON.parse(
    await readFile(resolve(dirname(nanotarPath), '../package.json'), 'utf8'),
  );
  const expectedVersion = bridgeLocks
    .map((lock) => lock.getIn(['importers', '.', 'dependencies', 'nanotar', 'version']))
    .find((version) => version !== undefined);
  if (expectedVersion !== nanotarManifest.version) {
    throw new Error('The preview builder and publishing action must use the same nanotar version');
  }
  const { output } = await build({
    input: source,
    platform: 'node',
    tsconfig: false,
    resolve: { alias: { nanotar: nanotarPath } },
    write: false,
    output: { format: 'esm' },
  });
  const chunk = output.find((entry) => entry.type === 'chunk');
  if (!chunk) {
    throw new Error('Could not bundle the registry bridge archive builder');
  }
  const module = await import(
    `data:text/javascript;base64,${Buffer.from(chunk.code).toString('base64')}`
  );
  return module.buildPreviewTarball;
}

function tarballUrl(pkg: PreviewPackage): string {
  return `${PREVIEW_REGISTRY}/tarballs/${pkg.packageJson.name}/${pkg.packageJson.version}/${pkg.shasum}.tgz`;
}

/** Resolve metadata locally; the future bridge URLs are recorded but not downloaded. */
async function resolvePreviewLockfile(
  version: string,
  packages: PreviewPackage[],
): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), 'vp-preview-lock-'));
  try {
    const manifest = Object.fromEntries(
      packages.map((pkg) => [
        pkg.packageJson.name,
        {
          name: pkg.packageJson.name,
          'dist-tags': { latest: version },
          versions: {
            [version]: {
              ...pkg.packageJson,
              dist: { integrity: pkg.integrity, shasum: pkg.shasum, tarball: tarballUrl(pkg) },
            },
          },
        },
      ]),
    );
    await writeFile(join(directory, 'mock-manifest.json'), JSON.stringify(manifest));
    await exec(
      process.execPath,
      [
        fileURLToPath(new URL('../tools/src/local-npm-registry.ts', import.meta.url)),
        '--live',
        '--',
        process.execPath,
        fileURLToPath(import.meta.url),
        '--resolve',
        version,
      ],
      { cwd: directory, maxBuffer: 10 * 1024 * 1024 },
    );
    return await readFile(join(directory, INSTALL_LOCKFILE), 'utf8');
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

export async function preparePreviewPackages(
  directory: string,
  version: string,
  buildPreviewTarball: BuildPreviewTarball,
  resolveLockfile = resolvePreviewLockfile,
): Promise<string> {
  if (!/^0\.0\.0-commit\.[a-f0-9]{40}$/.test(version)) {
    throw new Error('Expected a full commit preview version');
  }
  const archives = [];
  for (const file of (await readdir(directory)).filter((name) => /^pkg-\d+\.tgz$/.test(name))) {
    const tarball = new Uint8Array(await readFile(join(directory, file)));
    const entries = await parseTarGzip(tarball);
    const manifest = entries.find((entry) => entry.name === 'package/package.json');
    if (!manifest?.data) {
      throw new Error(`${file} is missing package/package.json`);
    }
    const { name } = JSON.parse(decoder.decode(manifest.data)) as { name: string };
    archives.push({ file, name, tarball });
  }
  const batch = new Set(archives.map(({ name }) => name));
  if (batch.size !== archives.length || !batch.has('vite-plus')) {
    throw new Error('Expected a preview batch with unique packages and vite-plus');
  }
  if (!archives.some(({ name }) => name.startsWith(platformPrefix))) {
    throw new Error('Preview batch is missing CLI platform packages');
  }

  // Normalize before resolving: upload mode rewrites manifests and rebuilds the
  // archives. A lockfile made from raw pnpm pack output would have wrong hashes.
  const packages = [];
  for (const archive of archives) {
    const pkg = await buildPreviewTarball(archive.tarball, archive.name, version, batch);
    packages.push({ ...pkg, file: archive.file });
  }
  const content = await resolveLockfile(version, packages);
  validateInstallLockfile(
    content,
    version,
    packages.find((pkg) => pkg.packageJson.name === 'vite-plus')!.tarball,
  );
  const lock = parse(content) as {
    packages: Record<string, { resolution: { integrity: string; tarball?: string } }>;
  };
  for (const pkg of packages) {
    const name = pkg.packageJson.name;
    const resolution = lock.packages[`${name}@${version}`]?.resolution;
    if (name.startsWith(platformPrefix) && resolution) {
      // The companion can only go in packages outside the locked dependency
      // graph; otherwise its own contents would change a hash it records.
      throw new Error(`Installation lockfile must not depend on ${name}`);
    }
    if (
      resolution &&
      (resolution.integrity !== pkg.integrity || resolution.tarball !== tarballUrl(pkg))
    ) {
      throw new Error(`Installation lockfile does not match the preview archive for ${name}`);
    }
  }

  for (const pkg of packages) {
    let tarball = pkg.tarball;
    const name = pkg.packageJson.name;
    if (name.startsWith(platformPrefix)) {
      const entries = (await parseTarGzip(tarball)).filter(
        (entry) => entry.name !== `package/${INSTALL_LOCKFILE}`,
      );
      const manifest = entries.find((entry) => entry.name === 'package/package.json')!;
      manifest.data = new TextEncoder().encode(
        JSON.stringify({
          ...pkg.packageJson,
          vpInstallLockfile: true,
          files: [...new Set([...((pkg.packageJson.files as string[]) ?? []), INSTALL_LOCKFILE])],
        }),
      );
      tarball = await createTarGzip([
        ...entries,
        { name: `package/${INSTALL_LOCKFILE}`, data: content },
      ]);
    }
    const prepared = await buildPreviewTarball(tarball, name, version, batch);
    if (!name.startsWith(platformPrefix) && prepared.integrity !== pkg.integrity) {
      throw new Error(`Registry bridge archive rebuild changed the integrity of ${name}`);
    }
    await writeFile(join(directory, pkg.file), prepared.tarball);
  }
  // Upload mode accepts only its manifest and pkg-*.tgz files. The companion
  // travels inside the platform archives, never as an extra artifact entry.
  return content;
}

async function main(): Promise<void> {
  if (process.argv[2] === '--resolve') {
    const registry = process.env.NPM_CONFIG_REGISTRY;
    if (!registry) {
      throw new Error('Missing local preview registry');
    }
    await writeFile(INSTALL_LOCKFILE, await generateInstallLockfile(process.argv[3], registry));
    return;
  }
  const [source, directory] = process.argv.slice(2);
  if (!source || !directory || !process.env.VERSION) {
    throw new Error('Expected bridge archive builder path, artifact directory, and VERSION');
  }
  await preparePreviewPackages(
    resolve(directory),
    process.env.VERSION,
    await loadPreviewBuilder(resolve(source)),
  );
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  main().catch((error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  });
}
