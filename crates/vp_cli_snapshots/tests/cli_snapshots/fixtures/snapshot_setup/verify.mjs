import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const prepared = fs.realpathSync(process.env.VP_HOME);
const referenceHome = path.resolve('reference-home');
const reference = path.join(referenceHome, '.vite-plus');
const windows = process.platform === 'win32';
assert.equal(process.env.VP_SELF_SETUP_NO_MODIFY_PATH, undefined);
if (windows) {
  const result = spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-Command',
      '[Environment]::GetEnvironmentVariable("Path", "User")',
    ],
    { encoding: 'utf8', timeout: 30000 },
  );
  assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
  assert.ok(!result.stdout.toLowerCase().includes(prepared.toLowerCase()));
}
const binaryName = windows ? 'vp.exe' : 'vp';
const bin = path.join(reference, 'current', 'bin');
fs.mkdirSync(bin, { recursive: true });
for (const name of windows ? [binaryName, 'vp-shim.exe'] : [binaryName]) {
  fs.copyFileSync(path.join(prepared, 'current', 'bin', name), path.join(bin, name));
}
// Provision the case's own vite-plus package like the prepared home. Without
// it, setup installs vite-plus@<binary version> from npm, which does not exist
// until that version is released.
const packageDir = path.join(reference, 'current', 'node_modules', 'vite-plus');
fs.mkdirSync(path.dirname(packageDir), { recursive: true });
fs.symlinkSync(
  fs.realpathSync(path.join(prepared, 'current', 'node_modules', 'vite-plus')),
  packageDir,
  windows ? 'junction' : 'dir',
);

const env = { ...process.env };
for (const name of Object.keys(env)) {
  if (name.startsWith('VP_') || name.startsWith('XDG_')) {
    delete env[name];
  }
}
Object.assign(env, {
  VP_HOME: reference,
  VP_CLI_TEST: '1',
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  HOME: referenceHome,
  USERPROFILE: referenceHome,
  PATH: windows
    ? (process.env.PATH ?? '')
        .split(path.delimiter)
        .filter((entry) => !entry.startsWith(path.dirname(prepared)))
        .join(path.delimiter)
    : '/usr/bin:/bin:/usr/sbin:/sbin',
});
for (const args of [
  ['env', 'setup', '--refresh'],
  ['env', 'on', 'pm'],
]) {
  const result = spawnSync(path.join(bin, binaryName), args, {
    env,
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
}

function installation(home) {
  const files = {};
  const normalize = (value) => value.replaceAll(home, '<home>');
  function visit(relative) {
    const file = path.join(home, relative);
    const stat = fs.lstatSync(file);
    if (stat.isSymbolicLink()) {
      files[relative] = { target: normalize(fs.readlinkSync(file)) };
    } else if (stat.isDirectory()) {
      files[relative] = 'directory';
      for (const name of fs.readdirSync(file).sort()) {
        visit(path.join(relative, name));
      }
    } else if (file.endsWith('.exe')) {
      files[relative] = {
        sha256: createHash('sha256').update(fs.readFileSync(file)).digest('hex'),
      };
    } else {
      files[relative] = { contents: normalize(fs.readFileSync(file, 'utf8')) };
    }
  }
  for (const name of fs.readdirSync(home).sort()) {
    if (
      name === 'config.json' ||
      name === 'bin' ||
      name === 'fallback-bin' ||
      /^env(?:\.|$)/.test(name)
    ) {
      visit(name);
    }
  }
  return files;
}

const actual = installation(prepared);
const expected = installation(reference);
assert.deepEqual(Object.keys(actual), Object.keys(expected));
for (const name of Object.keys(expected)) {
  assert.deepEqual(actual[name], expected[name], name);
}
console.log('Prepared preferences, environment files, and shims match the CLI setup sequence.');

// Mutating the reference must not change the case's own configuration.
const config = path.join(prepared, 'config.json');
const original = fs.readFileSync(config, 'utf8');
fs.writeFileSync(path.join(reference, 'config.json'), '{}');
assert.equal(fs.readFileSync(config, 'utf8'), original);
console.log('Configuration changes remain isolated to their own home.');
