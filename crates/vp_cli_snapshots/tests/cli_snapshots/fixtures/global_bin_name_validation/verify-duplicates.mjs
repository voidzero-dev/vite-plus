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
    '@a/native-first': nativePath,
    '@b/native-first': './cli.js',
    '@a/js-first': './cli.js',
    '@b/js-first': nativePath,
  },
}));

run('vp', ['install', '-g', '--ignore-scripts', '--node', process.version.slice(1), './duplicate-package']);
assert.equal(run('native-first', ['-p', '"native target"']), 'native target');
assert.equal(run('js-first'), 'JavaScript target');
assert.equal(run('vp', ['env', 'exec', '--node', process.version.slice(1), 'native-first', '-p', '"native target"']), 'native target');
const metadata = JSON.parse(fs.readFileSync(path.join(process.env.VP_HOME, 'packages', `${packageName}.json`), 'utf8'));
assert.deepEqual(metadata.bins.sort(), ['js-first', 'native-first']);
assert.deepEqual(metadata.jsBins, ['js-first']);
run('vp', ['uninstall', '-g', packageName]);
console.log('Duplicate normalized names keep the first valid target and its execution type.');
