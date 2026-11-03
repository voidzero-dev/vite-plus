// Pack the checked-in template with Node.js builtins, without fetching data.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { createTarball } from '../../../registry/archive.mjs';

const source = import.meta.dirname;
const files = [
  'package.json',
  'cli.mjs',
  'LICENSE',
  ...readdirSync(path.join(source, 'template'), { recursive: true })
    .filter((filename) => statSync(path.join(source, 'template', filename)).isFile())
    .map((filename) => `template/${filename.split(path.sep).join('/')}`)
    .toSorted(),
];
const tarball = createTarball(
  files.map((filename) => ({
    name: `package/${filename}`,
    contents: readFileSync(path.join(source, filename)),
    mode: filename === 'cli.mjs' ? 0o755 : 0o644,
  })),
);
const pkg = JSON.parse(readFileSync(path.join(source, 'package.json'), 'utf8'));
pkg.dist = {
  tarball: '{REGISTRY}/degit/-/degit-0.0.0.tgz',
  integrity: `sha512-${createHash('sha512').update(tarball).digest('base64')}`,
};
const manifest = {
  degit: { name: 'degit', 'dist-tags': { latest: '0.0.0' }, versions: { '0.0.0': pkg } },
};
for (const name of ['new_vite_monorepo', 'new_vite_monorepo_bun']) {
  const fixture = path.resolve(source, '../..', name);
  mkdirSync(path.join(fixture, 'tarballs'), { recursive: true });
  writeFileSync(path.join(fixture, 'tarballs', 'degit-0.0.0.tgz'), tarball);
  writeFileSync(path.join(fixture, 'mock-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
}
