const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const path = require('node:path');

const tools = (process.env.VP_PATH_INJECTED_TOOLS || '').split(',');
assert.ok(tools.includes('pnpm'));
assert.ok(tools.includes('pnpx'));
const shim = path.join(process.env.VP_HOME, 'bin', 'pnpm');
assert.equal(execFileSync(shim, ['--version'], { encoding: 'utf8' }).trim(), '10.19.0');
console.log('Workspace exec preserves injected pnpm');
