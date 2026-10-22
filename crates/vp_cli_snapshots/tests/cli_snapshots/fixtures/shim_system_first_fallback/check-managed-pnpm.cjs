// Run the pnpm shim and report whether it selected the system pnpm or a managed
// release. The managed launcher's own output is platform-specific, so it is not
// printed.
const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const path = require('node:path');

const result = spawnSync('pnpm', ['--version'], {
  encoding: 'utf8',
  shell: process.platform === 'win32',
  timeout: 120000,
});
console.log(`system pnpm: ${String(result.stdout).includes('system pnpm')}`);
const managed = path.join(process.env.VP_HOME, 'package_manager', 'pnpm', '10.18.0');
console.log(`managed pnpm 10.18.0 installed: ${existsSync(managed)}`);
