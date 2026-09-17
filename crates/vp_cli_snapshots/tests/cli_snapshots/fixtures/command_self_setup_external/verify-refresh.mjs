import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const action = process.argv[2];
const root = path.resolve(process.env.REPLACEMENT_ROOT ?? 'replacement');
const publicVp = path.join(root, 'brew/bin/vp');
const oldPrefix = path.join(root, 'brew/Cellar/vite-plus/old');
const newPrefix = path.join(root, 'brew/Cellar/vite-plus/new');

const managed = { VP_NODE_MANAGER: 'yes', VP_PM_MANAGER: 'yes' };
const mixed = { VP_NODE_MANAGER: 'yes', VP_PM_MANAGER: 'no', VP_PNPM_MANAGER: 'yes' };

function createBundle(source, prefix, label) {
  const bin = path.join(prefix, 'bin/vp');
  const pkg = path.join(prefix, 'node_modules/vite-plus');
  fs.mkdirSync(path.dirname(bin), { recursive: true });
  fs.copyFileSync(source, bin);
  fs.chmodSync(bin, 0o555);
  fs.mkdirSync(path.join(pkg, 'dist'), { recursive: true });
  fs.writeFileSync(path.join(pkg, 'package.json'), '{"name":"vite-plus"}');
  fs.writeFileSync(path.join(pkg, 'dist/bin.js'), `console.log(${JSON.stringify(label)});`);
  return bin;
}

function createEnvironment(directory) {
  const home = path.join(directory, 'home');
  const system = path.join(directory, 'system/bin');
  const runtime = path.join(home, 'js_runtime/node', process.versions.node, 'bin');
  fs.mkdirSync(system, { recursive: true });
  fs.mkdirSync(runtime, { recursive: true });
  fs.symlinkSync(process.execPath, path.join(system, 'node'));
  fs.symlinkSync(process.execPath, path.join(runtime, 'node'));
  fs.writeFileSync(path.join(directory, '.node-version'), process.versions.node);
  const env = { ...process.env };
  for (const key of Object.keys(env)) {
    if (key.startsWith('VP_') || key.startsWith('XDG_') || key === 'CI') delete env[key];
  }
  return {
    ...env,
    HOME: home,
    VP_HOME: home,
    VP_SELF_SETUP_NO_MODIFY_PATH: '1',
    NPM_CONFIG_REGISTRY: 'http://127.0.0.1:9',
    PATH: [path.join(home, 'bin'), path.join(directory, 'brew/bin'), system, env.PATH].join(
      path.delimiter,
    ),
  };
}

function run(binary, args, cwd, env, expectedStatus = 0) {
  const result = spawnSync(binary, args, { cwd, env, encoding: 'utf8', timeout: 30000 });
  assert.equal(result.status, expectedStatus, result.error?.message ?? result.stdout + result.stderr);
  return result.stdout;
}

function readSettings(env) {
  return JSON.parse(fs.readFileSync(path.join(env.VP_HOME, 'config.json'), 'utf8'));
}

function verifyDoctor(source) {
  const directory = path.resolve('doctor');
  const prefix = path.join(directory, 'cellar/vite-plus/0.3.2');
  const binary = createBundle(source, prefix, 'bundled CLI');
  const receipt = path.join(prefix, 'INSTALL_RECEIPT.json');
  fs.writeFileSync(receipt, '{"homebrew_version":"7.0.2"}');
  const env = createEnvironment(directory);
  const publicBin = path.join(directory, 'brew/bin');
  const shimBin = path.join(env.VP_HOME, 'bin');
  const systemBin = path.join(directory, 'system/bin');
  fs.mkdirSync(publicBin, { recursive: true });
  fs.symlinkSync(binary, path.join(publicBin, 'vp'));
  run(path.join(publicBin, 'vp'), ['--help'], directory, {
    ...env,
    VP_NODE_MANAGER: 'no',
    VP_PM_MANAGER: 'no',
  });

  const cases = [
    { label: 'Homebrew with shims on PATH', paths: [shimBin, publicBin, systemBin], status: 0 },
    { label: 'Homebrew without shims on PATH', paths: [publicBin, systemBin], status: 1 },
    {
      label: 'Missing vp in the shim directory',
      paths: [shimBin, systemBin],
      status: 1,
      missingVp: true,
    },
    {
      label: 'External package without a Homebrew receipt',
      paths: [shimBin, publicBin, systemBin],
      status: 0,
      external: true,
    },
  ];
  const shim = path.join(shimBin, 'vp');
  for (const { label, paths, status, missingVp, external } of cases) {
    if (missingVp) fs.renameSync(shim, `${shim}.hidden`);
    if (external) fs.unlinkSync(receipt);
    let output;
    try {
      output = run(
        binary,
        ['env', 'doctor', 'node'],
        directory,
        { ...env, PATH: paths.join(path.delimiter) },
        status,
      );
    } finally {
      if (missingVp) fs.renameSync(`${shim}.hidden`, shim);
    }
    const text = output.replace(/\u001b\[[0-9;]*m/g, '');
    assert.equal(/CLI source\s+Homebrew/.test(text), !external, text);
    console.log(label);
    console.log(
      text
        .split('\n')
        .filter((line) => /CLI source|CLI binary|[✓✗] (vp|Shim dir)\s/.test(line))
        .join('\n'),
    );
  }
}

function verifyPreferences(source) {
  for (const [label, choices] of [
    ['managed', managed],
    ['mixed', mixed],
  ]) {
    const directory = path.resolve(label);
    const binary = createBundle(source, path.join(directory, 'external'), label);
    const env = createEnvironment(directory);
    run(binary, ['--help'], directory, { ...env, ...choices });
    const before = readSettings(env);
    const modified = fs.statSync(binary).mtimeMs + 1000;
    fs.utimesSync(binary, new Date(modified), new Date(modified));
    run(binary, ['--help'], directory, env);
    assert.deepEqual(readSettings(env), before, `${label} choices changed after receipt expiry`);
    console.log(`${label} preferences survive executable replacement`);
    fs.utimesSync(binary, new Date(modified + 1000), new Date(modified + 1000));
    run(binary, ['--help'], directory, { ...env, VP_NODE_MANAGER: 'no', VP_PNPM_MANAGER: 'no' });
    assert.deepEqual(readSettings(env), {
      ...before,
      nodeShimMode: 'system_first',
      packageManagerShimModes: { ...before.packageManagerShimModes, pnpm: 'system_first' },
    });
  }
  console.log('explicit overrides still apply during setup without changing other preferences');
}

function verifyReplacement(source) {
  createBundle(source, oldPrefix, 'bundle-old');
  createBundle(source, newPrefix, 'bundle-new');
  fs.mkdirSync(path.dirname(publicVp), { recursive: true });
  fs.symlinkSync(path.join(oldPrefix, 'bin/vp'), publicVp);
  const env = createEnvironment(root);
  run(publicVp, ['--help'], root, { ...env, ...mixed });
  const before = readSettings(env);
  const script = [
    'set -e',
    'vp sync-versions --json',
    'test "$(hash -t vp)" = "$VP_HOME/bin/vp"',
    '"$TEST_NODE" "$TEST_SCRIPT" switch',
    '"$VP_HOME/bin/node" -p "20 + 1"',
    'vp sync-versions --json',
    '"$TEST_NODE" "$TEST_SCRIPT" remove',
    '"$VP_HOME/bin/vp" sync-versions --json',
    'vp sync-versions --json',
    '"$VP_HOME/bin/node" -p "40 + 2"',
  ].join('\n');
  const output = run('bash', ['--noprofile', '--norc', '-c', script], root, {
    ...env,
    REPLACEMENT_ROOT: root,
    TEST_NODE: process.execPath,
    TEST_SCRIPT: fileURLToPath(import.meta.url),
  });
  assert.equal(output.trim(), 'bundle-old\n21\nbundle-new\nbundle-new\nbundle-new\n42');
  assert.deepEqual(readSettings(env), before, 'mixed choices changed after a package upgrade');
  // Explicit setup must preserve the public entrypoint too.
  run(publicVp, ['env', 'setup', '--refresh'], root, env);
  assert.equal(fs.readlinkSync(path.join(env.VP_HOME, 'bin/vp')), publicVp);
  console.log('vp follows the public entrypoint while the old package still exists');
  console.log('direct vp, cached Bash vp, and node shims survive removal of the old package');
  console.log('mixed preferences survive a new package path');
}

function main() {
  // These actions run between commands in the same Bash session.
  if (action === 'switch') {
    fs.unlinkSync(publicVp);
    fs.symlinkSync(path.join(newPrefix, 'bin/vp'), publicVp);
    return;
  }
  if (action === 'remove') {
    fs.rmSync(oldPrefix, { recursive: true });
    return;
  }

  const source = path.join(process.env.VP_HOME, 'bin/vp');
  if (action === 'preferences') {
    verifyPreferences(source);
    return;
  }
  if (action === 'doctor') {
    verifyDoctor(source);
    return;
  }
  assert.equal(action, 'replacement');
  verifyReplacement(source);
}

main();
