const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { mkdirSync, writeFileSync } = require('node:fs');

assert.equal(process.version, 'v20.18.0');
assert.ok(process.env.VP_PATH_INJECTED_TOOLS.split(',').includes('node'));
const mode = process.argv[2];
const args = ['env', 'exec', 'node', '--version'];
const env = { ...process.env };
if (mode === 'project') {
  mkdirSync('project-b');
  writeFileSync('project-b/.node-version', '22.18.0');
  writeFileSync('project-b/package.json', '{}');
  args.unshift('-C', 'project-b');
} else {
  env.VP_NODE_VERSION = '22.18.0';
}
if (mode === 'wrapper') {
  env.VP_SHIM_WRAPPER = '1';
  args.splice(3, 0, '--');
}
const version = execFileSync('vp', args, { env, encoding: 'utf8' }).trim();
assert.equal(version, mode === 'wrapper' ? 'v20.18.0' : 'v22.18.0');
console.log(`env exec ${mode}: ${version}`);
