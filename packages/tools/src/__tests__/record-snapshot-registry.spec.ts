import { execFileSync } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { expect, it } from 'vitest';

it('refreshes selected versions, removes obsolete tarballs, and preserves other packages', () => {
  // Exercise URL escaping as well as Windows drive-letter paths.
  const root = mkdtempSync(path.join(tmpdir(), 'vp-record-registry-test-#'));
  try {
    const directory = path.join(root, 'crates/vp_cli_snapshots/tests/cli_snapshots/registry');
    cpSync(
      fileURLToPath(
        new URL(
          '../../../../crates/vp_cli_snapshots/tests/cli_snapshots/registry',
          import.meta.url,
        ),
      ),
      directory,
      { recursive: true },
    );
    const script = path.join(root, 'packages/tools/src/record-snapshot-registry.ts');
    mkdirSync(path.dirname(script), { recursive: true });
    cpSync(fileURLToPath(new URL('../record-snapshot-registry.ts', import.meta.url)), script);

    const manifestPath = path.join(directory, 'manifest.json');
    const original = JSON.parse(readFileSync(manifestPath, 'utf8'));
    const incoming = structuredClone(original);
    for (const [name, version] of [
      ['is-number', '0.1.0'],
      ['yocto-queue', '0.0.0'],
    ]) {
      const metadata = incoming[name];
      const entry = structuredClone(metadata.versions[metadata['dist-tags'].latest]);
      const basename = `${name}-${version}.tgz`;
      cpSync(
        path.join(directory, 'tarballs', path.posix.basename(entry.dist.tarball)),
        path.join(directory, 'tarballs', basename),
      );
      entry.version = version;
      entry.dist.tarball = `{REGISTRY}/${name}/-/${basename}`;
      metadata.versions[version] = entry;
      metadata.time[version] = metadata.time.created;
      metadata['dist-tags'].obsolete = version;
    }
    writeFileSync(manifestPath, JSON.stringify(incoming));
    writeFileSync(path.join(root, 'incoming.json'), JSON.stringify(incoming));
    const mock = path.join(root, 'mock-fetch.mjs');
    writeFileSync(
      mock,
      `import { appendFileSync, readFileSync } from 'node:fs';
      import path from 'node:path';
      const metadata = JSON.parse(readFileSync('incoming.json', 'utf8').replaceAll('{REGISTRY}', 'https://registry.npmjs.org'));
      globalThis.fetch = async (url) => {
        appendFileSync('requests.jsonl', JSON.stringify(url) + '\\n');
        const pathname = new URL(url).pathname.slice(1);
        if (['is-number', 'yocto-queue'].includes(pathname)) return Response.json(metadata[pathname]);
        const basename = path.posix.basename(pathname);
        if (!['is-number-6.0.0.tgz', 'is-number-7.0.0.tgz', 'yocto-queue-0.1.0.tgz',
          'yocto-queue-1.0.0.tgz', 'yocto-queue-1.2.2.tgz'].includes(basename)) {
          throw new Error('Unexpected download: ' + url);
        }
        return new Response(readFileSync(path.join(${JSON.stringify(directory)}, 'tarballs', basename)));
      };`,
    );
    execFileSync(
      process.execPath,
      ['--import', pathToFileURL(mock).href, script, 'is-number', 'yocto-queue'],
      {
        cwd: root,
        timeout: 10_000,
      },
    );
    expect(JSON.parse(readFileSync(manifestPath, 'utf8'))).toEqual(original);
    expect(existsSync(path.join(directory, 'tarballs/is-number-0.1.0.tgz'))).toBe(false);
    expect(existsSync(path.join(directory, 'tarballs/yocto-queue-0.0.0.tgz'))).toBe(false);
    expect(readFileSync(path.join(root, 'requests.jsonl'), 'utf8').trim().split('\n')).toHaveLength(
      7,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
