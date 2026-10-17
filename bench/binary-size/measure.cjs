// Use the same gzip implementation and level as vp-binary-size.yml.
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { gzipSync } = require('node:zlib');

const artifacts = Object.fromEntries(
  process.argv.slice(2).map((path) => {
    const bytes = readFileSync(path);
    return [
      path,
      {
        raw: bytes.length,
        gzip: gzipSync(bytes, { level: 9 }).length,
        sha256: createHash('sha256').update(bytes).digest('hex'),
      },
    ];
  }),
);
process.stdout.write(
  JSON.stringify({ node: process.version, zlib: process.versions.zlib, artifacts }, null, 2) + '\n',
);
