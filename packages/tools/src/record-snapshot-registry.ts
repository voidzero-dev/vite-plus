// Refresh the small, published test packages used by CLI snapshots.
// This is an explicit maintenance command, never part of a test run.
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = fileURLToPath(
  new URL('../../../crates/vp_cli_snapshots/tests/cli_snapshots/registry/', import.meta.url),
);
const manifestPath = path.join(directory, 'manifest.json');
const previous = JSON.parse(await readFile(manifestPath, 'utf8')) as Record<string, unknown>;
const manifest: Record<string, unknown> = { ...previous };
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
  const metadata = (await response.json()) as {
    versions: Record<string, { dist: { tarball: string; integrity: string } }>;
  };
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
