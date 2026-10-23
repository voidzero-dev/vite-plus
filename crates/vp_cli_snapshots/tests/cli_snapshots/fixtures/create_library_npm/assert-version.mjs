import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const read = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));
const directory = path.resolve(process.argv[2] ?? 'library');
const pkg = read(path.join(directory, 'package.json'));
const version = process.argv[3] ?? pkg.overrides.vite.replace('npm:@voidzero-dev/vite-plus-core@', '');
for (const relative of ['.', 'apps/website', 'packages/utils']) {
  const manifest = path.join(directory, relative, 'package.json');
  if (relative !== '.' && !fs.existsSync(manifest)) continue;
  const spec = read(manifest).devDependencies['vite-plus'];
  assert.ok(spec === version || spec === 'catalog:', `${relative}: unexpected vite-plus ${spec}`);
  const require = createRequire(manifest);
  const cliPath = require.resolve('vite-plus/package.json');
  assert.equal(read(cliPath).version, version, `${relative}: installed vite-plus version`);
  const corePath = createRequire(cliPath).resolve('vite/package.json');
  assert.equal(read(corePath).name, '@voidzero-dev/vite-plus-core');
  assert.equal(read(corePath).version, version, `${relative}: resolved core version`);
}
if (process.argv[2]) {
  console.log('All scaffolded packages resolve the expected Vite+ and core versions.');
} else {
  console.log('Library dependency and installed Vite+ match the toolchain override.');
}
