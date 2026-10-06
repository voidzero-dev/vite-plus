const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { basename } = require('node:path');
const { spawnSync } = require('node:child_process');

const [mode, tool, ...args] = process.argv.slice(2);
assert.equal(basename(process.execPath), `${tool}.exe`);
assert.equal(process.versions.node, readFileSync('.node-version', 'utf8'));
assert.equal(process.env.SHIM_TEST_VALUE, 'inherited value');
if (mode === 'stdin') {
  assert.equal(readFileSync(0, 'utf8'), 'piped input\n');
  console.log(`${tool}: piped stdin preserved`);
} else {
  assert.deepEqual(args, ['value with spaces', 'path\\file']);
  console.log(`${tool}: native executable, arguments and environment preserved`);
  if (mode === 'nested') {
    // A package script resolves the same executable from its inherited PATH,
    // instead of falling back to the adjacent .cmd wrapper.
    const child = spawnSync('cmd.exe', ['/d', '/c', tool, 'probe.cjs', 'child', tool, ...args], {
      stdio: 'inherit',
    });
    assert.equal(child.status, 0);
    console.log(`${tool}: nested script uses the native executable`);
  }
}
process.exit(Number(process.env.SHIM_TEST_EXIT_CODE || 0));
