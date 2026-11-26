import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, realpathSync } from 'node:fs';

// A stand-in for the selected package's Vite+ CLI. It checks that the global
// CLI runs it as `vp -C <dir>` would: in the package, with the package's
// Node.js pin, and with the directory already fixed.
const { version } = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const { name } = JSON.parse(readFileSync('package.json', 'utf8'));
const expectedNode = `v${readFileSync('.node-version', 'utf8').trim()}`;
const pathNode = execFileSync('node', ['--version'], { encoding: 'utf8' }).trim();
assert.equal(name, 'app');
assert.equal(process.version, expectedNode);
assert.equal(pathNode, expectedNode);
assert.equal(process.env.VP_EXPLICIT_CHDIR, '1');
if (process.platform !== 'win32' && realpathSync(process.env.PWD) !== realpathSync(process.cwd())) {
  throw new Error('PWD differs from cwd');
}

console.log(`Vite+ ${version}: ${process.argv.slice(2).join(' ')} in ${name}`);
console.log(`Process Node.js: ${process.version}`);
console.log(`PATH Node.js: ${pathNode}`);
