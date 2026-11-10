const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { version } = require('vite-plus/package.json');

const [expected, packageManager] = process.argv.slice(2);
const agent = process.env.VP_USER_AGENT;
assert.equal(
  agent,
  expected === 'current' ? `vp/${version}` : expected === 'unset' ? undefined : expected,
);
if (packageManager) {
  assert.equal(process.env.npm_config_user_agent?.split('/')[0], packageManager);
}

// A managed node shim and its children must preserve the marker, including an
// inherited version that differs from the installed CLI's version.
const child = spawnSync('node', ['-p', 'process.env.VP_USER_AGENT'], { encoding: 'utf8' });
assert.equal(child.status, 0, child.stderr || child.error?.message);
assert.equal(child.stdout.trim(), String(agent));
console.log(`VP_USER_AGENT=${agent ?? '(unset)'}`);
console.log('Node descendants preserve the marker.');
if (packageManager) {
  console.log(`npm_config_user_agent identifies ${packageManager}.`);
}
