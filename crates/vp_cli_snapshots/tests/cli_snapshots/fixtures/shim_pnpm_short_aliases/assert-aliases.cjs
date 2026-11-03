const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { join } = require('node:path');

// Invoke the actual vp shims even when a parent injected the managed pnpm bin directory.
const run = (tool, args) => execFileSync(
  join(process.env.VP_HOME, 'bin', tool + (process.platform === 'win32' ? '.exe' : '')),
  args,
  { encoding: 'utf8' },
).trim();

assert.equal(run('pn', ['--version']), process.argv[2]);
assert.equal(run('pn', ['--version']), run('pnpm', ['--version']));
const help = run('pnx', ['--help']);
assert.equal(help, run('pnpx', ['--help']));
assert.match(help, /dlx|pnx/);
console.log('pn and pnx use the selected pnpm and dlx binaries');
