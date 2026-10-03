const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { dirname, join, relative } = require('node:path');

// Replace npm only inside this unseeded case's runtime, using the same Node.js.
const prefix = process.platform === 'win32' ? dirname(process.execPath) : dirname(dirname(process.execPath));
assert.ok(!relative(process.env.VP_HOME, prefix).startsWith('..'));
const npm = join(prefix, process.platform === 'win32' ? 'node_modules/npm' : 'lib/node_modules/npm');
const result = spawnSync(process.execPath, [join(npm, 'bin/npm-cli.js'), 'install', '-g', 'npm@12.0.2', '--prefix', prefix, '--ignore-scripts'], { stdio: 'inherit' });
assert.equal(result.status, 0);
assert.equal(require(join(npm, 'package.json')).version, '12.0.2');
