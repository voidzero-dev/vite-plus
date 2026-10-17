import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { stripVTControlCharacters } from 'node:util';

// Execute the same JS entry point as the local vp binary on every platform.
const cli = fileURLToPath(import.meta.resolve('vite-plus/bin'));
const args = process.argv.includes('--before') ? ['lint', '-c', '.oxlintrc.json'] : ['lint'];
function lint(extraArgs = []) {
  const result = spawnSync(process.execPath, [cli, ...args, ...extraArgs], {
    encoding: 'utf8', timeout: 30000,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  return result;
}

// Exercise plain `vp lint` after migration, without selecting a config or adding flags.
const plain = lint();
assert.match(stripVTControlCharacters(plain.stdout + plain.stderr), /Found 2 warnings and 3 errors/);

// Compare each rule's severity and file without depending on worker completion order.
const { diagnostics } = JSON.parse(lint(['--format', 'json']).stdout);
const actual = diagnostics.map(({ filename, code, severity }) => `${filename}: ${severity} ${code}`).sort();
assert.deepEqual(actual, [
  'example.test.js: error override(no-bar)',
  'example.test.js: warning eslint(no-debugger)',
  'index.js: error eslint(no-debugger)',
  'index.js: error local(no-foo)',
  'index.js: warning eslint(no-console)',
]);
console.log(`vp ${args.join(' ')}: 3 errors and 2 warnings`);
console.log(actual.join('\n'));
