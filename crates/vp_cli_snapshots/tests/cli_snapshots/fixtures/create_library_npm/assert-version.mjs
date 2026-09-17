import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));
const pkg = read('library/package.json');
const version = pkg.overrides.vite.replace('npm:@voidzero-dev/vite-plus-core@', '');
assert.equal(pkg.devDependencies['vite-plus'], version);
assert.equal(read('library/node_modules/vite-plus/package.json').version, version);
console.log('Library dependency and installed Vite+ match the toolchain override.');
