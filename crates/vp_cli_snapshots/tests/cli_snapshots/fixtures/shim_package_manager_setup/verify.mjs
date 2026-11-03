import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const helper = fileURLToPath(import.meta.url);
const source = path.join(process.env.VP_HOME, 'current/bin/vp');
const families = {
  npm: ['npm', 'npx'],
  pnpm: ['pnpm', 'pnpx'],
  yarn: ['yarn', 'yarnpkg'],
  bun: ['bun', 'bunx'],
};

function run(binary, args, env, cwd = process.cwd()) {
  const result = spawnSync(binary, args, { env, cwd, encoding: 'utf8', timeout: 30000 });
  assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
  return result.stdout;
}

function writeFile(file, content) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, content);
}

function writeExecutable(file, label) {
  writeFile(file, `#!/bin/sh\nprintf '${label}\\n'\n`);
  fs.chmodSync(file, 0o755);
}

function createInstallation(name, settings = {}) {
  const root = path.resolve(name);
  const home = path.join(root, 'home');
  const bin = path.join(home, 'bin');
  const fallback = path.join(home, 'fallback-bin');
  const system = path.join(root, 'system');
  const binary = path.join(home, 'current/bin/vp');
  for (const dir of [bin, fallback, system, path.dirname(binary)]) {
    fs.mkdirSync(dir, { recursive: true });
  }
  fs.copyFileSync(source, binary);
  fs.chmodSync(binary, 0o755);
  writeFile(path.join(path.dirname(binary), '.vp-setup-complete'), '');
  writeFile(path.join(home, 'config.json'), JSON.stringify(settings));
  writeFile(path.join(root, '.node-version'), process.versions.node);
  writeFile(
    path.join(root, 'package.json'),
    '{"name":"setup-migration","private":true,"packageManager":"pnpm@10.18.0"}',
  );
  const node = path.join(home, 'js_runtime/node', process.versions.node, 'bin/node');
  fs.mkdirSync(path.dirname(node), { recursive: true });
  fs.symlinkSync(process.execPath, node);
  for (const tool of families.pnpm) {
    writeExecutable(path.join(home, 'package_manager/pnpm/10.18.0/pnpm/bin', tool), `managed-${tool}`);
  }
  const env = { ...process.env };
  for (const key of Object.keys(env)) {
    if (key.startsWith('VP_') || key.startsWith('XDG_')) {
      delete env[key];
    }
  }
  Object.assign(env, {
    VP_HOME: home,
    HOME: home,
    CI: 'true',
    VP_SKIP_DEPS_INSTALL: '1',
    VP_SELF_SETUP_NO_MODIFY_PATH: '1',
    VP_NODE_DIST_MIRROR: 'http://127.0.0.1:9',
    NPM_CONFIG_REGISTRY: 'http://127.0.0.1:9',
    PATH: [bin, system, fallback].join(path.delimiter),
  });
  return { root, home, bin, fallback, system, binary, env };
}

function createShim(test, dir, tool) {
  fs.symlinkSync(test.binary, path.join(dir, tool));
}

function readSettings(test) {
  return JSON.parse(fs.readFileSync(path.join(test.home, 'config.json'), 'utf8'));
}

function refreshSetup(test) {
  run(test.binary, ['env', 'setup', '--refresh'], test.env, test.root);
}

function assertModes(test, expected) {
  assert.deepEqual(readSettings(test).packageManagerShimModes, expected);
  for (const [kind, mode] of Object.entries(expected)) {
    const target = mode === 'managed' ? test.bin : test.fallback;
    for (const tool of families[kind]) {
      assert.equal(fs.realpathSync(path.join(target, tool)), test.binary);
    }
  }
}

function verifyPreferences() {
  const test = createInstallation('inferred');
  createShim(test, test.bin, 'npx'); // Ownership of an alias also preserves the family.
  createShim(test, test.fallback, 'yarnpkg');
  writeExecutable(path.join(test.system, 'pnpx'), 'system-pnpx');
  refreshSetup(test);
  assertModes(test, { npm: 'managed', pnpm: 'system_first', yarn: 'system_first', bun: 'managed' });
  const saved = fs.readFileSync(path.join(test.home, 'config.json'), 'utf8');
  fs.unlinkSync(path.join(test.system, 'pnpx'));
  refreshSetup(test);
  assert.equal(fs.readFileSync(path.join(test.home, 'config.json'), 'utf8'), saved);
  console.log(
    'Missing families: preserve main and fallback aliases, detect external aliases, default to managed.',
  );
  console.log('Repeated setup preserves saved preferences after the external tool disappears.');

  const mixed = createInstallation('mixed', {
    nodeShimMode: 'system_first',
    defaultNodeVersion: '22.18.0',
    defaultPackageManagerVersions: { pnpm: '10.18.0' },
    packageManagerShimModes: { npm: 'system_first', pnpm: 'managed' },
  });
  writeExecutable(path.join(mixed.system, 'pnpm'), 'system-pnpm');
  refreshSetup(mixed);
  assertModes(mixed, { npm: 'system_first', pnpm: 'managed', yarn: 'managed', bun: 'managed' });
  const mixedSettings = readSettings(mixed);
  assert.equal(mixedSettings.nodeShimMode, 'system_first');
  assert.equal(mixedSettings.defaultNodeVersion, '22.18.0');
  assert.deepEqual(mixedSettings.defaultPackageManagerVersions, { pnpm: '10.18.0' });
  console.log('Explicit mixed preferences, Node mode, and default versions remain unchanged.');

  const legacy = createInstallation('legacy-global', { shimMode: 'system_first' });
  const oldTool = path.join(legacy.home, 'old-package/pnpm');
  writeExecutable(oldTool, 'old-pnpm');
  fs.symlinkSync(oldTool, path.join(legacy.bin, 'pnpm'));
  writeFile(
    path.join(legacy.home, 'bins/pnpm.json'),
    JSON.stringify({
      name: 'pnpm',
      package: 'pnpm',
      version: '10.18.0',
      nodeVersion: process.versions.node,
      source: 'npm',
    }),
  );
  writeExecutable(path.join(legacy.system, 'pnpm'), 'system-pnpm');
  refreshSetup(legacy);
  const legacySettings = readSettings(legacy);
  assert.equal(legacySettings.nodeShimMode, 'system_first');
  assert.equal(legacySettings.packageManagerShimModes.pnpm, 'managed');
  assert.equal(fs.realpathSync(path.join(legacy.bin, 'pnpm')), oldTool);
  assert.equal(fs.existsSync(path.join(legacy.home, 'bins/pnpm.json')), false);
  console.log('Legacy global package ownership is captured before cleanup removes its metadata.');

  const shared = createInstallation('shared-bin');
  writeExecutable(path.join(shared.bin, 'pnpm'), 'foreign-pnpm');
  refreshSetup(shared);
  assert.equal(readSettings(shared).packageManagerShimModes.pnpm, 'system_first');
  assert.equal(run(path.join(shared.bin, 'pnpm'), [], shared.env).trim(), 'foreign-pnpm');
  console.log('A foreign tool in the shared bin directory remains intact.');

  const fresh = createInstallation('fresh-node-only');
  fs.unlinkSync(path.join(fresh.home, 'config.json'));
  const external = path.join(fresh.root, 'download/vp');
  fs.mkdirSync(path.dirname(external));
  fs.copyFileSync(source, external);
  fs.chmodSync(external, 0o755);
  fs.rmSync(path.join(fresh.home, 'current'), { recursive: true });
  writeExecutable(path.join(fresh.system, 'pnpm'), 'system-pnpm');
  run(
    external,
    ['--help'],
    { ...fresh.env, VP_NODE_MANAGER: 'no', VP_VERSION: 'setup-migration-test' },
    fresh.root,
  );
  assert.deepEqual(readSettings(fresh), {
    nodeShimMode: 'system_first',
    packageManagerShimModes: {
      npm: 'managed',
      pnpm: 'system_first',
      yarn: 'managed',
      bun: 'managed',
    },
  });
  console.log(
    'Unattended fresh setup with only a Node override initializes every package-manager family.',
  );

  const shellOnly = createInstallation('shell-only');
  fs.unlinkSync(path.join(shellOnly.home, 'config.json'));
  run(shellOnly.binary, ['env', 'setup', '--env-only'], shellOnly.env, shellOnly.root);
  assert.equal(fs.existsSync(path.join(shellOnly.home, 'config.json')), false);
  console.log('Shell-only setup does not change preferences.');
}

function prepareDispatch() {
  const test = createInstallation('dispatch');
  fs.unlinkSync(path.join(test.home, 'config.json'));
  createShim(test, test.bin, 'pnpm');
  writeExecutable(path.join(test.system, 'pnpm'), 'system-pnpm');
}

function verifyBashCache() {
  for (const owned of [false, true]) {
    const test = createInstallation(owned ? 'cached-owned' : 'cached-system');
    createShim(test, test.bin, 'npm'); // Core shim present in older releases.
    for (const tool of families.pnpm) {
      writeExecutable(path.join(test.system, tool), `system-${tool}`);
      if (owned) {
        createShim(test, test.bin, tool);
      }
    }
    const env = {
      ...test.env,
      PATH: [test.env.PATH, process.env.PATH].join(path.delimiter),
      SETUP_NODE: process.execPath,
      SETUP_HELPER: helper,
    };
    const output = run(
      'bash',
      ['--noprofile', '--norc', path.join(path.dirname(helper), 'bash-cache.sh')],
      env,
      test.root,
    );
    const lines = output.split('\n');
    const expected = owned ? 'managed' : 'system';
    assert.equal(lines.filter(line => line === `${expected}-pnpm`).length, 3, output);
    assert.equal(lines.filter(line => line === `${expected}-pnpx`).length, 2, output);
    for (const tool of families.pnpm) {
      const cached = path.join(owned ? test.bin : test.system, tool);
      assert.equal(lines.filter(line => line === cached).length, 2, output);
    }
    const modes = readSettings(test).packageManagerShimModes;
    assert.equal(modes.pnpm, owned ? 'managed' : 'system_first');
    assert.equal(modes.npm, 'managed');
    console.log(`${expected} pnpm and pnpx: cached paths survive setup and repeated direct calls.`);
  }
}

switch (process.argv[2]) {
  case 'refresh':
    fs.rmSync(path.join(path.dirname(source), '.vp-setup-complete'), { force: true });
    run(source, ['env', 'setup', '--refresh'], process.env);
    break;
  case 'preferences':
    verifyPreferences();
    break;
  case 'prepare-dispatch':
    prepareDispatch();
    break;
  default:
    verifyBashCache();
}
