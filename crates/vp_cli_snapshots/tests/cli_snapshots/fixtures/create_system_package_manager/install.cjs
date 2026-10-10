const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const env = { ...process.env };
function run(args, envs = env) {
  const result = spawnSync('vp', args, { env: envs, encoding: 'utf8' });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, result.stdout + result.stderr);
  return result.stdout;
}

// Provision a real npm, then copy it outside the managed installation so the
// system-first resolver sees an independent executable on PATH.
run(['env', 'install', 'npm@10.9.3']);
const current = JSON.parse(run(['env', 'current', 'npm', '--json'], { ...env, VP_NPM_VERSION: '10.9.3' })).package_manager;
const npmRoot = path.dirname(path.dirname(fs.realpathSync(current.bin_paths.npm)));
fs.cpSync(npmRoot, 'system-npm', { recursive: true });
const binDir = path.resolve('system-bin');
fs.mkdirSync(binDir);
fs.writeFileSync(path.join(binDir, 'npm'), `#!${process.execPath}\nrequire(${JSON.stringify(path.resolve('system-npm/bin/npm-cli.js'))});\n`, { mode: 0o755 });
env.PATH = [path.join(env.VP_HOME, 'bin'), binDir, env.PATH].join(path.delimiter);
run(['env', 'off', 'npm']);
delete env.VP_SKIP_INSTALL;
const output = run(['create', 'vite:application', '--directory', 'app', '--package-manager', 'npm',
  '--no-interactive', '--no-agent', '--no-editor', '--no-hooks', '--no-git']);
assert.doesNotMatch(output, /EBADDEVENGINES/);
const pin = JSON.parse(fs.readFileSync('app/package.json', 'utf8')).devEngines.packageManager;
assert.equal(pin.version, '10.9.3');
assert.ok(fs.statSync('app/node_modules/vite-plus/package.json').isFile());
assert.ok(fs.statSync('app/package-lock.json').isFile());
console.log('System npm version pinned and dependencies installed successfully');
