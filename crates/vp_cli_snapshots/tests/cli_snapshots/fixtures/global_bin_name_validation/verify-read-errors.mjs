import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const home = process.env.VP_HOME;
const packageName = 'vp-bin-read-error-fixture';
const metadataPath = path.join(home, 'packages', `${packageName}.json`);
const packageRoot = path.join(home, 'packages', packageName);
const args = ['install', '-g', '--ignore-scripts', '--node', process.version.slice(1)];

function run(command, args = [], expectSuccess = true) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    timeout: 30_000,
    env: {
      ...process.env,
      npm_config_offline: 'true',
      npm_config_audit: 'false',
      npm_config_update_notifier: 'false',
    },
  });
  assert.ifError(result.error);
  assert.notEqual(result.status, null, result.stdout + result.stderr);
  assert.equal(result.status === 0, expectSuccess, result.stdout + result.stderr);
  return result.stdout + result.stderr;
}

function writePackage(directory, version, bin) {
  fs.mkdirSync(`${directory}/dist/directory.js`, { recursive: true });
  fs.writeFileSync(`${directory}/package.json`, JSON.stringify({
    name: packageName, version, bin,
    scripts: { postinstall: 'node -e "process.exit(1)"' },
  }));
  fs.writeFileSync(`${directory}/cli.js`, `#!/usr/bin/env node\nconsole.log("command ${version}");\n`);
}

function installations() {
  return fs.existsSync(packageRoot)
    ? fs.readdirSync(packageRoot, { withFileTypes: true }).filter((entry) => entry.isDirectory()).map((entry) => entry.name)
    : [];
}

function assertFailedInstall(directory) {
  const output = run('vp', [...args, `./${directory}`], false);
  assert.match(output, /Failed to inspect binary 'unreadable-command'/);
  assert.ok(output.includes(packageName), output);
  assert.equal(fs.existsSync(path.join(home, 'bins', 'unreadable-command.json')), false);
  for (const suffix of ['', '.exe', '.cmd', '.ps1', '.shim']) {
    assert.equal(fs.existsSync(path.join(home, 'bin', `unreadable-command${suffix}`)), false);
  }
}

const invalidPackages = ['', '.', 'dist', 'dist/directory.js'].map((target, index) => {
  const directory = `invalid-package-${index}`;
  writePackage(directory, '2.0.0', {
    'readable-command': './cli.js',
    'unreadable-command': target,
  });
  assertFailedInstall(directory);
  assert.equal(fs.existsSync(metadataPath), false);
  assert.equal(fs.existsSync(path.join(home, 'bins', 'readable-command.json')), false);
  assert.deepEqual(installations(), []);
  return directory;
});
console.log('Empty and directory bin targets fail installation without registering commands or leaving installations.');

writePackage('valid-package', '1.0.0', { 'readable-command': './cli.js' });
run('vp', [...args, './valid-package']);
const metadata = fs.readFileSync(metadataPath, 'utf8');
assert.equal(run('readable-command').trim(), 'command 1.0.0');
for (const directory of invalidPackages) {
  assertFailedInstall(directory);
  assert.equal(fs.readFileSync(metadataPath, 'utf8'), metadata);
  assert.deepEqual(installations(), [JSON.parse(metadata).installId]);
  assert.equal(run('readable-command').trim(), 'command 1.0.0');
}
run('vp', ['uninstall', '-g', packageName]);
console.log('Failed updates preserve the previous installation, metadata, and working command.');
