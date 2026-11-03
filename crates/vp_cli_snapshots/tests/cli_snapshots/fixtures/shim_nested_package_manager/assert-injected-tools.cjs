const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const [manager, version, nodeVersion] = process.argv.slice(2);
const tools = process.env.VP_PATH_INJECTED_TOOLS.split(',');
const aliases = { pnpm: 'pnpx', yarn: 'yarnpkg', bun: 'bunx' };
assert.equal(process.version, `v${nodeVersion}`);
for (const tool of ['node', 'npm', 'npx', manager, aliases[manager]]) {
  assert.ok(tools.includes(tool), `${tool} must be recorded`);
}
assert.equal(new Set(tools).size, tools.length);
const binary = path.join(
  process.env.VP_HOME,
  'bin',
  manager + (process.platform === 'win32' ? '.exe' : ''),
);
const child = spawnSync(binary, ['--version'], { encoding: 'utf8' });
assert.equal(child.status, 0, child.stderr);
assert.equal(child.stdout.trim(), version);
console.log(`Injected ${manager} resolves to ${version} on Node ${nodeVersion}`);
