import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const home = process.env.VP_HOME;
const packageName = 'vp-bin-name-fixture';
const outside = fs.mkdtempSync(path.join(path.dirname(home), 'bin-name-outside-'));
const escapedName = `../../${path.basename(outside)}/victim`;
const sentinels = ['victim', 'victim.json'];
for (const name of sentinels) fs.writeFileSync(path.join(outside, name), 'keep me');

function run(command, args = []) {
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
  assert.equal(result.status, 0, result.stdout + result.stderr);
  return result.stdout;
}

function writePackage(directory, version, bin) {
  fs.mkdirSync(`${directory}/dist`, { recursive: true });
  fs.writeFileSync(`${directory}/package.json`, JSON.stringify({
    name: packageName, version, bin,
    scripts: { postinstall: 'node -e "process.exit(1)"' },
  }));
  fs.writeFileSync(`${directory}/dist/cli.js`, `#!/usr/bin/env node\nconsole.log("command ${version}");\n`);
}

function assertOutsideUnchanged() {
  assert.deepEqual(fs.readdirSync(outside).sort(), sentinels);
  for (const name of sentinels) assert.equal(fs.readFileSync(path.join(outside, name), 'utf8'), 'keep me');
}

try {
  writePackage('valid-package', '1.0.0', { 'safe-command': './dist/cli.js' });
  const unsafeBins = {
    [escapedName]: './dist/cli.js',
    [`${escapedName}-new`]: './dist/cli.js',
    '@scope/../../victim': './dist/cli.js',
    'with space': './dist/cli.js',
    'outside-target': path.join(outside, 'victim'),
    'relative-outside-target': '../outside.js',
  };
  writePackage('mixed-package', '2.0.0', {
    'safe-command': './dist/cli.js',
    '@scope/scoped-command': './dist/cli.js',
    ...unsafeBins,
  });
  writePackage('invalid-package', '3.0.0', unsafeBins);
  const args = ['install', '-g', '--ignore-scripts', '--node', process.version.slice(1)];
  run('vp', [...args, './valid-package']);
  assert.equal(run('safe-command').trim(), 'command 1.0.0');

  run('vp', [...args, './mixed-package']);
  const metadataPath = path.join(home, 'packages', `${packageName}.json`);
  const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
  assert.equal(metadata.version, '2.0.0');
  assert.deepEqual(metadata.bins.sort(), ['safe-command', 'scoped-command']);
  assert.equal(run('safe-command').trim(), 'command 2.0.0');
  assert.equal(run('scoped-command').trim(), 'command 2.0.0');
  assert.match(run('vp', ['env', 'which', 'scoped-command']), /cli\.js/);
  assert.equal(run('vp', ['env', 'exec', '--node', process.version.slice(1), 'scoped-command']).trim(), 'command 2.0.0');
  assertOutsideUnchanged();
  for (const name of ['outside-target', 'relative-outside-target', 'with space']) {
    assert.equal(fs.existsSync(path.join(home, 'bins', `${name}.json`)), false);
  }
  console.log('Mixed update installs safe and scoped commands, and skips unsafe names and targets.');

  run('vp', [...args, './invalid-package']);
  const latest = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
  assert.equal(latest.version, '3.0.0');
  assert.deepEqual(latest.bins, []);
  for (const name of ['safe-command', 'scoped-command']) {
    assert.equal(fs.existsSync(path.join(home, 'bins', `${name}.json`)), false);
    for (const suffix of ['', '.exe', '.cmd']) {
      assert.equal(fs.existsSync(path.join(home, 'bin', `${name}${suffix}`)), false);
    }
  }
  const installs = fs.readdirSync(path.join(home, 'packages', packageName), { withFileTypes: true })
    .filter((entry) => entry.isDirectory()).map((entry) => entry.name);
  assert.deepEqual(installs, [latest.installId]);
  run('vp', ['uninstall', '-g', packageName]);
  assertOutsideUnchanged();
  console.log('Invalid-only update installs without commands; uninstall preserves outside files.');
} finally {
  fs.rmSync(outside, { recursive: true, force: true });
}
