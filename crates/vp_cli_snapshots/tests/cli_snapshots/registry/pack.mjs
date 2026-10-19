// Pack local CLI fixtures with Node.js builtins, without running package scripts.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { createTarball } from './archive.mjs';

const root = import.meta.dirname;
const destination = path.join(root, 'local');
mkdirSync(path.join(destination, 'tarballs'), { recursive: true });
const manifest = {};
for (const directory of readdirSync(path.join(root, 'packages')).toSorted()) {
  const source = path.join(root, 'packages', directory);
  const pkg = JSON.parse(readFileSync(path.join(source, 'package.json'), 'utf8'));
  const { name, version } = pkg;
  const tarball = createTarball(
    readdirSync(source)
      .toSorted()
      .map((filename) => ({
        name: `package/${filename}`,
        contents: readFileSync(path.join(source, filename)),
        mode: filename.endsWith('.cjs') ? 0o755 : 0o644,
      })),
  );
  const filename = `${name}-${version}.tgz`;
  writeFileSync(path.join(destination, 'tarballs', filename), tarball);
  pkg.dist = {
    tarball: `{REGISTRY}/${name}/-/${filename}`,
    integrity: `sha512-${createHash('sha512').update(tarball).digest('base64')}`,
  };
  manifest[name] = { name, 'dist-tags': { latest: version }, versions: { [version]: pkg } };
}
writeFileSync(path.join(destination, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
