import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

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
  return result.stdout.trim();
}

const packageName = 'vp-bin-dedup-fixture';
const nativePath = process.platform === 'win32' ? './native-tool.exe' : './native-tool';
fs.mkdirSync('duplicate-package');
fs.copyFileSync(process.execPath, path.join('duplicate-package', nativePath));
fs.writeFileSync('duplicate-package/cli.js', '#!/usr/bin/env node\nconsole.log("JavaScript target");\n');
fs.writeFileSync('duplicate-package/package.json', JSON.stringify({
  name: packageName,
  version: '1.0.0',
  bin: {
    '@z/native-first': nativePath,
    '@a/native-first': './cli.js',
    '@z/js-first': './cli.js',
    '@a/js-first': nativePath,
  },
}));

run('vp', ['install', '-g', '--ignore-scripts', '--node', process.version.slice(1), './duplicate-package']);
assert.equal(run('native-first'), 'JavaScript target');
assert.equal(run('js-first', ['-p', '"native target"']), 'native target');
assert.match(run('vp', ['env', 'which', 'native-first']), /cli\.js/);
assert.match(run('vp', ['env', 'which', 'js-first']), /native-tool/);
assert.equal(run('vp', ['env', 'exec', '--node', process.version.slice(1), 'js-first', '-p', '"native target"']), 'native target');
const metadata = JSON.parse(fs.readFileSync(path.join(process.env.VP_HOME, 'packages', `${packageName}.json`), 'utf8'));
assert.deepEqual(metadata.bins.sort(), ['js-first', 'native-first']);
assert.deepEqual(metadata.jsBins, ['native-first']);
run('vp', ['uninstall', '-g', packageName]);

if (process.platform !== 'win32') {
  // npm only makes its own last-wins target executable, so Vite+ must fix the retained one.
  const modePackageName = 'vp-bin-mode-fixture';
  fs.mkdirSync('mode-package');
  fs.writeFileSync('mode-package/native.sh', '#!/bin/sh\necho native target\n', { mode: 0o644 });
  fs.writeFileSync('mode-package/ignored.sh', '#!/bin/sh\necho ignored target\n', { mode: 0o644 });
  fs.writeFileSync('mode-package/package.json', JSON.stringify({
    name: modePackageName,
    version: '1.0.0',
    bin: { 'probe-native': './native.sh', '@scope/probe-native': '../ignored.sh' },
  }));
  run('vp', ['install', '-g', '--ignore-scripts', '--node', process.version.slice(1), './mode-package']);
  assert.equal(run('probe-native'), 'native target');
  run('vp', ['uninstall', '-g', modePackageName]);
}
console.log('Duplicate normalized names keep the last valid target and its execution type.');
