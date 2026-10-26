import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [packageManager, mode] = process.argv.slice(2);
const env = { ...process.env };
const pathKey = Object.keys(env).find((key) => key.toUpperCase() === 'PATH');
// Keep system tools such as Git, but remove every directory that exposes vp.
const vpExecutables = ['vp', 'vp.exe', 'vp.cmd', 'vp.bat'];
const pathsWithoutVp = env[pathKey]
  .split(path.delimiter)
  .filter((dir) => !vpExecutables.some((bin) => existsSync(path.join(dir, bin))));
env[pathKey] = [path.dirname(process.execPath), ...pathsWithoutVp].join(path.delimiter);
delete env.VP_CLI_BIN;
assert.equal(spawnSync('vp', ['--version'], { env }).error?.code, 'ENOENT');

if (mode === 'unrelated') {
  const binDir = path.resolve('unrelated-bin');
  mkdirSync(binDir);
  writeFileSync(path.join(binDir, 'vp'), '#!/bin/sh\necho unrelated-vp >&2\nexit 99\n', {
    mode: 0o755,
  });
  writeFileSync(path.join(binDir, 'vp.cmd'), '@echo unrelated-vp >&2\r\n@exit /b 99\r\n');
  env[pathKey] = `${binDir}${path.delimiter}${env[pathKey]}`;
}

let npm;
if (packageManager.startsWith('npm@')) {
  // Provision a real older launcher through the test registry before removing the global home.
  const { downloadPackageManager } = await import(
    new URL('./binding/index.js', import.meta.resolve('vite-plus/package.json'))
  );
  npm = await downloadPackageManager({ name: 'npm', version: packageManager.split('@')[1] });
  env[pathKey] = `${npm.binPrefix}${path.delimiter}${env[pathKey]}`;
  env.VP_HOME = path.resolve('no-global-home');
  delete env.npm_config_user_agent;
} else {
  env.npm_config_user_agent = packageManager.replace('@', '/');
}
const cli = fileURLToPath(import.meta.resolve('vite-plus/bin'));
const projectDir = 'projects with spaces/app';
// Reuse the local bundled-template fixture, without an external template download.
const result = spawnSync(
  process.execPath,
  [
    ...(npm ? [path.join(npm.binPrefix, 'npm-cli.js'), 'exec', '--', 'node'] : []),
    cli,
    'create',
    '@your-org:demo',
    '--directory',
    projectDir,
    '--no-interactive',
    '--no-agent',
    '--no-editor',
    '--git',
    '--hooks',
    '--verbose',
  ],
  { env, encoding: 'utf8', timeout: 210000 },
);
const output = `${result.stdout}\n${result.stderr}`;
assert.equal(result.status, 0, output || result.error?.message);
assert.match(output, /Dependencies installed/);
assert.match(output, /Code formatted/);
assert.match(output, /Git hooks configured/);
assert.doesNotMatch(
  output,
  /unrelated-vp|Failed to install git hooks|Install failed|Format failed/,
);
assert.ok(existsSync(path.join(projectDir, 'node_modules/vite-plus/package.json')));
assert.match(readFileSync(path.join(projectDir, '.git/config'), 'utf8'), /hooksPath/);
const lockfile = npm
  ? 'package-lock.json'
  : packageManager.startsWith('pnpm@')
    ? 'pnpm-lock.yaml'
    : 'yarn.lock';
assert.ok(existsSync(path.join(projectDir, lockfile)));
if (npm) {
  const pkg = JSON.parse(readFileSync(path.join(projectDir, 'package.json'), 'utf8'));
  assert.equal(pkg.devEngines.packageManager.version, npm.version);
  assert.match(output, /npx vp run/);
  const next = spawnSync(process.execPath, [path.join(npm.binPrefix, 'npx-cli.js'), 'vp', 'run'], {
    cwd: projectDir,
    env,
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(next.status, 0, `${next.stdout}\n${next.stderr}` || next.error?.message);
  console.log(`Generated npm ${npm.version} pin matches the launcher; npx vp run succeeds.`);
}
console.log(
  `Created a project with ${packageManager.split('@')[0]} and ${mode === 'missing' ? 'no' : 'an unrelated'} vp on PATH.`,
);
console.log('Dependencies installed, code formatted, and Git hooks configured.');
