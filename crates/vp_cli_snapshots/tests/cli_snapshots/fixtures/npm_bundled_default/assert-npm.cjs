const assert = require('node:assert/strict');
const { execSync } = require('node:child_process');
const { dirname, resolve } = require('node:path');

const bundled = require(resolve(
  dirname(process.execPath),
  process.platform === 'win32' ? 'node_modules/npm/package.json' : '../lib/node_modules/npm/package.json',
)).version;
const expected = process.argv[2] || bundled;
const run = (command) => execSync(command, { encoding: 'utf8' }).trim();

// Clear inherited tool markers so each shim resolves this project's configuration itself.
delete process.env.VP_PATH_INJECTED_TOOLS;
assert.equal(run('npm --version'), expected);
assert.equal(run('npx --version'), expected);
assert.equal(JSON.parse(run('vp pm version --json')).npm, expected);
for (const scope of ['pm', 'npm']) {
  const current = JSON.parse(run(`vp env current ${scope} --json`)).package_manager;
  assert.equal(current.version, expected);
  if (!process.argv[2]) {
    assert.equal(current.source, 'Node.js bundled npm');
  }
}
console.log(process.argv[2] ? `npm/npx and vp use configured npm ${expected}` : 'npm/npx and vp use Node bundled npm');
