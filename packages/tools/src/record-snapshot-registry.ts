// Refresh the small, published test packages used by CLI snapshots.
// This is an explicit maintenance command, never part of a test run.
import { createHash } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

interface Metadata {
  versions: Record<string, { dist: { tarball: string; integrity: string } }>;
  'dist-tags': Record<string, string>;
  time?: Record<string, string>;
}

// Keep the initial versions, update targets, and an out-of-range latest version.
const selectedVersions: Record<string, string[]> = {
  'is-number': ['6.0.0', '7.0.0'],
  'yocto-queue': ['0.1.0', '1.0.0', '1.2.2'],
};

const directory = fileURLToPath(
  new URL('../../../crates/vp_cli_snapshots/tests/cli_snapshots/registry/', import.meta.url),
);
const manifestPath = path.join(directory, 'manifest.json');
const previous = JSON.parse(await readFile(manifestPath, 'utf8')) as Record<string, Metadata>;
const manifest = { ...previous };
const requested = process.argv.slice(2);
const names = requested.length ? requested : Object.keys(previous);
await mkdir(path.join(directory, 'tarballs'), { recursive: true });

for (const name of names.toSorted()) {
  const response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(name)}`, {
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok) {
    throw new Error(`Cannot record ${name}: HTTP ${response.status}`);
  }
  const metadata = (await response.json()) as Metadata;
  const selected = selectedVersions[name];
  if (selected) {
    if (!selected.includes(metadata['dist-tags'].latest)) {
      throw new Error(
        `Update selectedVersions for ${name}: latest is ${metadata['dist-tags'].latest}`,
      );
    }
    metadata.versions = Object.fromEntries(
      selected.map((version) => {
        const entry = metadata.versions[version];
        if (!entry) {
          throw new Error(`Cannot record missing version ${name}@${version}`);
        }
        return [version, entry];
      }),
    );
    metadata['dist-tags'] = Object.fromEntries(
      Object.entries(metadata['dist-tags']).filter(([, version]) => selected.includes(version)),
    );
    if (metadata.time) {
      metadata.time = Object.fromEntries(
        Object.entries(metadata.time).filter(
          ([key]) => key === 'created' || key === 'modified' || selected.includes(key),
        ),
      );
    }
  }
  for (const version of Object.values(metadata.versions)) {
    const response = await fetch(version.dist.tarball, { signal: AbortSignal.timeout(30_000) });
    if (!response.ok) {
      throw new Error(`Cannot record ${version.dist.tarball}: HTTP ${response.status}`);
    }
    const bytes = Buffer.from(await response.arrayBuffer());
    const integrity = `sha512-${createHash('sha512').update(bytes).digest('base64')}`;
    if (integrity !== version.dist.integrity) {
      throw new Error(`Integrity mismatch for ${version.dist.tarball}`);
    }
    const basename = path.posix.basename(new URL(version.dist.tarball).pathname);
    await writeFile(path.join(directory, 'tarballs', basename), bytes);
    version.dist.tarball = `{REGISTRY}/${name}/-/${basename}`;
  }
  manifest[name] = metadata;
}
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

const retainedTarballs = new Set(
  Object.values(manifest).flatMap((metadata) =>
    Object.values(metadata.versions).map((version) => path.posix.basename(version.dist.tarball)),
  ),
);
for (const name of names) {
  for (const version of Object.values(previous[name]?.versions ?? {})) {
    const basename = path.posix.basename(version.dist.tarball);
    if (!retainedTarballs.has(basename)) {
      await rm(path.join(directory, 'tarballs', basename), { force: true });
    }
  }
}
