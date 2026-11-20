// Run with node to regenerate this fixture's local template tarball and metadata.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { createTarball } from '../../registry/archive.mjs';

const root = import.meta.dirname;
const pkg = {
  name: '@override/create',
  version: '1.0.0',
  createConfig: {
    templates: [
      { name: 'demo', description: 'Package manager override fixture', template: './template' },
    ],
  },
};
const tarball = createTarball([
  { name: 'package/package.json', contents: Buffer.from(JSON.stringify(pkg)), mode: 0o644 },
  {
    name: 'package/template/package.json',
    contents: readFileSync(path.join(root, 'template/package.json')),
    mode: 0o644,
  },
  {
    name: 'package/template/record-install.cjs',
    contents: readFileSync(path.join(root, 'record-install.cjs')),
    mode: 0o644,
  },
]);
mkdirSync(path.join(root, 'tarballs'), { recursive: true });
writeFileSync(path.join(root, 'tarballs/create-1.0.0.tgz'), tarball);
writeFileSync(
  path.join(root, 'mock-manifest.json'),
  JSON.stringify(
    {
      [pkg.name]: {
        name: pkg.name,
        'dist-tags': { latest: pkg.version },
        versions: {
          [pkg.version]: {
            ...pkg,
            dist: {
              tarball: '{REGISTRY}/@override/create/-/create-1.0.0.tgz',
              integrity: `sha512-${createHash('sha512').update(tarball).digest('base64')}`,
            },
          },
        },
      },
    },
    null,
    2,
  ) + '\n',
);
