import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { stripVTControlCharacters } from 'node:util';

const [name, version, monorepo] = process.argv.slice(2);
const directory = monorepo ? 'packages/demo' : 'demo';
const args = [
  'create',
  '@override:demo',
  '--directory',
  directory,
  '--package-manager',
  'pnpm',
  '--no-interactive',
  '--no-agent',
  '--no-editor',
  '--no-hooks',
];
// Windows needs cmd to launch the local CLI's .cmd shim. All arguments are fixture literals.
const result =
  process.platform === 'win32'
    ? spawnSync('cmd.exe', ['/d', '/s', '/c', `vp ${args.join(' ')}`], { encoding: 'utf8' })
    : spawnSync('vp', args, { encoding: 'utf8' });
const output = stripVTControlCharacters(`${result.stdout ?? ''}${result.stderr ?? ''}`);
process.stdout.write(output);
assert.ifError(result.error);
assert.equal(result.status, 0);
assert.ok(output.includes(`${name} ${version}`), 'summary must report the effective version');
assert.ok(!output.includes('Install failed'), 'dependency installation must succeed');
const installRoot = monorepo ? '.' : directory;
const agent = readFileSync(`${installRoot}/installed-with.txt`, 'utf8');
assert.ok(agent.startsWith(`${name}/${version} `), `actual installer: ${agent}`);
const manifest = JSON.parse(readFileSync(`${installRoot}/package.json`, 'utf8'));
assert.deepEqual(manifest.devEngines.packageManager, {
  name: monorepo ? 'pnpm' : name,
  version: monorepo ? '12.11.2' : version,
  onFail: 'download',
});
if (name === 'pnpm') {
  assert.ok(existsSync(`${installRoot}/pnpm-lock.yaml`));
  if (!monorepo) {
    // pnpm 10.6.1 supports catalogs but still reads settings from package.json.
    assert.ok(manifest.pnpm?.overrides, 'settings must use the effective version');
  } else {
    const workspace = readFileSync('pnpm-workspace.yaml', 'utf8');
    assert.match(workspace, /packages\/\*/);
    assert.doesNotMatch(workspace, /^\s*- ['"]?\.['"]?\s*$/m);
  }
} else {
  assert.ok(existsSync(`${installRoot}/package-lock.json`));
  assert.ok(!existsSync(`${installRoot}/pnpm-workspace.yaml`));
}
console.log('Verified generated configuration, actual installer, and summary.');
