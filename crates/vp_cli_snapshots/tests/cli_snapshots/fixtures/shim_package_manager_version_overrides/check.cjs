const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { delimiter } = require('node:path');

// Start each child at the shim even though the parent Node process has injected tool paths.
const env = {
  ...process.env,
  PATH: [process.env.VP_HOME + '/bin', '/usr/bin', '/bin'].join(delimiter),
  VP_PATH_INJECTED_TOOLS: '',
};
const [tool, expected] = process.argv.slice(2);
const args = tool === 'vp' ? ['install', '--', '--version'] : ['--version'];
const actual = execFileSync(tool, args, { env, encoding: 'utf8' }).trim();
assert.equal(actual, expected);
console.log(`${tool} uses the expected version`);
