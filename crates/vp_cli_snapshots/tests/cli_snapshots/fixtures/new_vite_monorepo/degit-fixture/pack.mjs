import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { join, sep } from 'node:path';
import { gzipSync } from 'node:zlib';

// Pack the checked-in template without dependencies or external data.
const source = import.meta.dirname;
const files = [
  'package.json',
  'cli.mjs',
  'LICENSE',
  ...readdirSync(join(source, 'template'), { recursive: true })
    .map((name) => `template/${name.split(sep).join('/')}`)
    .filter((name) => statSync(join(source, name)).isFile())
    .sort(),
];

const blocks = [];
for (const name of files) {
  const body = readFileSync(join(source, name));
  const archiveName = `package/${name}`;
  assert(Buffer.byteLength(archiveName) <= 100, `Archive path is too long: ${archiveName}`);

  // Write a ustar header with fixed ownership and timestamps.
  const header = Buffer.alloc(512);
  header.write(archiveName);
  header.write(name === 'cli.mjs' ? '0000755\0' : '0000644\0', 100);
  header.write('0000000\0', 108);
  header.write('0000000\0', 116);
  header.write(body.length.toString(8).padStart(11, '0') + '\0', 124);
  header.write('00000000000\0', 136);
  header.fill(' ', 148, 156);
  header.write('0', 156);
  header.write('ustar', 257);
  header.write('00', 263);
  const checksum = header.reduce((sum, byte) => sum + byte, 0);
  header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148);
  blocks.push(header, body, Buffer.alloc((512 - (body.length % 512)) % 512));
}

// End with two empty blocks and pad to tar's standard 20-block record size.
const archive = Buffer.concat([...blocks, Buffer.alloc(1024)]);
const padding = Buffer.alloc((10240 - (archive.length % 10240)) % 10240);
const tarball = gzipSync(Buffer.concat([archive, padding]), { level: 9 });
// Omit the host OS from the gzip header to keep the hash platform-independent.
tarball[9] = 255;
const integrity = `sha512-${createHash('sha512').update(tarball).digest('base64')}`;
const pkg = JSON.parse(readFileSync(join(source, 'package.json'), 'utf8'));
pkg.dist = { tarball: '{REGISTRY}/degit/-/degit-0.0.0.tgz', integrity };
const manifest = {
  degit: {
    name: 'degit',
    'dist-tags': { latest: '0.0.0' },
    versions: { '0.0.0': pkg },
  },
};
for (const name of ['new_vite_monorepo', 'new_vite_monorepo_bun']) {
  const fixture = join(source, '..', '..', name);
  mkdirSync(join(fixture, 'tarballs'), { recursive: true });
  writeFileSync(join(fixture, 'tarballs', 'degit-0.0.0.tgz'), tarball);
  writeFileSync(join(fixture, 'mock-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
}
