import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

import writePayload from './payload.cjs';

const action = process.argv[2];
const localBootstrap = action === 'bootstrap';
const root = path.resolve('isolated');
const home = path.join(root, 'home');
const project = path.join(root, 'project');
const emptyPath = path.join(root, 'empty-path');
const binName = process.platform === 'win32' ? 'vp.exe' : 'vp';
const nodeName = process.platform === 'win32' ? 'node.exe' : 'node';
const external = path.join(root, 'external/bin', binName);
const version = process.versions.node;
const source = fs.realpathSync(path.join(process.env.VP_HOME, 'bin', binName));
const lockfile = "lockfileVersion: '9.0'\n# Controlled lazy-install fixture.\n";

function write(file, text) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
}

fs.mkdirSync(project, { recursive: true });
fs.mkdirSync(emptyPath, { recursive: true });
fs.mkdirSync(path.dirname(external), { recursive: true });
fs.copyFileSync(source, external);
fs.chmodSync(external, 0o755);
write(path.join(path.dirname(external), 'vp-install-lock.yaml'), lockfile);
if (process.platform === 'win32') {
  fs.copyFileSync(
    path.join(path.dirname(source), 'vp-shim.exe'),
    path.join(path.dirname(external), 'vp-shim.exe'),
  );
}
write(path.join(project, 'package.json'), '{"name":"lazy-setup-test","private":true}');
// Stop local CLI resolution at this stub instead of finding the runner's checkout.
const localPackage = path.join(project, 'node_modules/vite-plus');
write(path.join(localPackage, 'package.json'), '{"name":"vite-plus","version":"0.3.2"}');

const env = { ...process.env };
for (const key of Object.keys(env)) {
  if (key.startsWith('VP_')) delete env[key];
}
Object.assign(env, {
  VP_HOME: home,
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  VP_NODE_MANAGER: 'yes',
  VP_PM_MANAGER: 'yes',
  VP_ENV_USE_EVAL_ENABLE: '1',
  VP_SHELL: 'bash',
  VP_NODE_DIST_MIRROR: 'http://127.0.0.1:9',
  NPM_CONFIG_REGISTRY: 'http://127.0.0.1:9',
  npm_config_registry: 'http://127.0.0.1:9',
  CI: 'true',
  NO_COLOR: '1',
  PATH: emptyPath,
});

let defaultBinary = external;

function run(args, extra = {}, status = 0, binary = defaultBinary) {
  const result = spawnSync(binary, args, {
    cwd: project,
    env: { ...env, ...extra },
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, status, result.error?.message ?? result.stdout + result.stderr);
  return result;
}

run(
  ['env', '--help'],
  localBootstrap ? { VP_SKIP_DEPS_INSTALL: '1', VP_VERSION: 'local-dev-test' } : {},
);
const installed = fs.realpathSync(path.join(home, 'current'));
if (localBootstrap) defaultBinary = path.join(installed, 'bin', binName);
const payload = path.join(installed, 'node_modules/vite-plus');
const pending = path.join(installed, '.vp-js-deps-pending');
const receipt = path.join(installed, 'node_modules/.vp-install-complete');
const companion = path.join(installed, 'bin/vp-install-lock.yaml');
assert.ok(fs.existsSync(path.join(installed, 'bin/.vp-setup-complete')));
if (localBootstrap) {
  assert.ok(fs.existsSync(path.join(installed, '.vp-local-deps')));
  assert.ok(!fs.existsSync(companion));
} else {
  assert.equal(fs.readFileSync(companion, 'utf8'), lockfile);
}
assert.ok(!fs.existsSync(payload));
assert.ok(!fs.existsSync(receipt));
assert.ok(!fs.existsSync(pending));
assert.ok(!fs.existsSync(path.join(home, 'js_runtime')));
assert.ok(!fs.existsSync(path.join(home, 'package_manager')));
console.log('Fresh native setup completes without Node, pnpm, or a JavaScript payload');

// Simulate a later shell without the installer's registry environment.
delete env.NPM_CONFIG_REGISTRY;
delete env.npm_config_registry;

function seedIndex() {
  write(
    path.join(home, 'js_runtime/node/index_cache.json'),
    JSON.stringify({
      expires_at: Math.floor(Date.now() / 1000) + 3600,
      versions: [
        { version: `v${version}`, lts: 'Test' },
        { version: 'v16.20.2', lts: 'Gallium' },
      ],
    }),
  );
}

function seedInstaller() {
  seedIndex();
  const node =
    process.platform === 'win32'
      ? path.join(home, 'js_runtime/node', version, nodeName)
      : path.join(home, 'js_runtime/node', version, 'bin', nodeName);
  fs.mkdirSync(path.dirname(node), { recursive: true });
  fs.copyFileSync(process.execPath, node);
  fs.chmodSync(node, 0o755);
  write(path.join(project, '.node-version'), `${version}\n`);
  const wrapper = JSON.parse(fs.readFileSync(path.join(installed, 'package.json'), 'utf8'));
  const pnpmPackage = path.join(home, 'package_manager/pnpm', wrapper.packageManager.slice(5), 'pnpm');
  write(
    path.join(pnpmPackage, 'package.json'),
    JSON.stringify({ name: 'pnpm', engines: { node: '>=18.12' } }),
  );
  const pnpm = path.join(pnpmPackage, 'bin');
  write(path.join(pnpm, 'pnpm.cjs'), fs.readFileSync(path.resolve('install.cjs')));
  write(path.join(pnpm, 'payload.cjs'), fs.readFileSync(path.resolve('payload.cjs')));
  for (const suffix of ['', '.cmd', '.ps1']) write(path.join(pnpm, `pnpm${suffix}`), 'cached');
}

function installCount() {
  const file = path.join(installed, 'install-count');
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8').trim().split('\n').length : 0;
}

function checkOutput(result, args) {
  assert.deepEqual(JSON.parse(result.stdout), { args, node: version });
}

function checkInstalled() {
  assert.ok(!fs.existsSync(pending));
  assert.deepEqual(fs.readFileSync(receipt), createHash('sha256').update(lockfile).digest());
  assert.equal(fs.readFileSync(path.join(installed, 'pnpm-lock.yaml'), 'utf8'), lockfile);
}

if (action === 'native') {
  seedIndex();
  run(['env', 'pin', '16.20.2', '--no-install']);
  run(['env', 'default', '16.20.2']);
  assert.match(run(['env', 'use', '16.20.2', '--no-install']).stdout, /VP_NODE_VERSION=16\.20\.2/);
  const current = JSON.parse(run(['env', 'current', 'node', '--json']).stdout);
  assert.equal(current.node.version, '16.20.2');
  assert.equal(current.node.installed, false);
  write(path.join(emptyPath, nodeName), 'This is not a working Node executable.');
  fs.chmodSync(path.join(emptyPath, nodeName), 0o755);
  run(['env', 'use', '16.20.2', '--no-install']);
  run(['env', 'unpin', 'node']);
  assert.ok(!fs.existsSync(payload));
  assert.ok(!fs.existsSync(path.join(home, 'package_manager')));
  assert.deepEqual(fs.readdirSync(path.join(home, 'js_runtime/node')), ['index_cache.json']);
  console.log('Node 16 selection works with absent and unusable Node executables');
  console.log('Environment management does not install the JavaScript toolchain');
} else {
  seedInstaller();
  const globalArgs = ['lint', '--init'];
  if (action === 'bootstrap') {
    assert.ok(!Object.hasOwn(env, 'VP_SKIP_DEPS_INSTALL'));
    assert.ok(!Object.hasOwn(env, 'VP_VERSION'));
    const failed = run(globalArgs, {}, 1);
    assert.match(failed.stderr, /finish the local bootstrap/);
    run(['env', '--help']);
    assert.equal(installCount(), 0);
    assert.ok(!fs.existsSync(pending));
    console.log('An incomplete local bootstrap reports how to finish without installing release dependencies');

    writePayload(payload, version);
    checkOutput(run(globalArgs, { TEST_INSTALL_FAIL: '1' }), globalArgs);
    const toolchain = JSON.parse(run(['toolchain', '--global', '--json']).stdout);
    assert.equal(toolchain.source.scope, 'global');
    assert.equal(installCount(), 0);
    assert.ok(!fs.existsSync(companion));
    assert.ok(!fs.existsSync(receipt));
    assert.ok(!fs.existsSync(pending));
    console.log('The completed local payload runs offline without an install receipt or companion lockfile');
  } else if (action === 'explicit') {
    const selected = path.join(root, 'selected-node');
    const result = run(
      [],
      {
        VP_SELF_SETUP_SHELL: 'sh',
        VP_SETUP_NODE_VERSION: version,
        VP_SETUP_NODE_VERSION_FILE: selected,
        NPM_CONFIG_REGISTRY: 'http://127.0.0.1:9',
      },
      0,
      path.join(installed, 'bin', binName),
    );
    assert.deepEqual(
      result.stdout.trim().split(/\r?\n/).map((line) => line.split('=')[0]),
      ['INSTALL_DIR', 'SHIM_DIR', 'CACHE_DIR', 'CONFIG_DIR', 'STATE_DIR'],
    );
    assert.equal(fs.readFileSync(selected, 'utf8'), version);
    assert.equal(installCount(), 1);
    checkInstalled();
    checkOutput(run(globalArgs), globalArgs);
    assert.equal(installCount(), 1);
    console.log('Explicit setup completes eager frozen installation after native setup was already marked');
    console.log('The installer returns shell paths and the selected Node version');
  } else if (action === 'local') {
    write(path.join(localPackage, 'dist/bin.js'), "console.log('project-local CLI');");
    assert.equal(run(['build']).stdout.trim(), 'project-local CLI');
    assert.ok(!fs.existsSync(payload));
    assert.equal(installCount(), 0);
    console.log('Project-local execution does not install the global payload');
  } else if (action === 'retry') {
    const failed = run(globalArgs, { TEST_INSTALL_FAIL: '1' }, 1);
    assert.match(failed.stderr, /Failed to install production dependencies/);
    assert.ok(fs.existsSync(path.join(payload, 'dist/bin.js')));
    assert.ok(fs.existsSync(pending));
    assert.ok(!fs.existsSync(receipt));
    run(['env', 'use', '16.20.2', '--no-install']);
    assert.equal(installCount(), 1);
    checkOutput(run(globalArgs), globalArgs);
    assert.equal(installCount(), 2);
    checkInstalled();
    console.log('A partial failed install leaves environment management usable');
    console.log('The next global command retries despite existing package files');
  } else if (action === 'concurrent') {
    await checkConcurrent(globalArgs);
  } else if (action === 'receipt') {
    checkOutput(run(globalArgs), globalArgs);
    assert.equal(installCount(), 1);
    checkInstalled();
    for (const state of ['missing', 'invalid']) {
      if (state === 'missing') fs.unlinkSync(receipt);
      else write(receipt, 'not the release lockfile digest');
      const count = installCount();
      run(['env', '--help']);
      assert.equal(installCount(), count);
      checkOutput(run(globalArgs), globalArgs);
      assert.equal(installCount(), count + 1);
      checkInstalled();
    }
    checkOutput(run(globalArgs), globalArgs);
    assert.equal(installCount(), 3);
    console.log('Existing payloads with missing or invalid receipts are reinstalled');
    console.log('Native commands remain usable and completed frozen installs are reused');
  } else if (action === 'vpx') {
    const vpx = path.join(home, 'bin', process.platform === 'win32' ? 'vpx.exe' : 'vpx');
    const args = ['./script.cjs', 'value'];
    write(
      path.join(project, 'script.cjs'),
      'console.log(JSON.stringify({ args: process.argv.slice(2), loader: globalThis.lazyLoader, node: process.versions.node }));',
    );
    function writeLoader(dir, name) {
      write(path.join(dir, 'script-preload.cjs'), `globalThis.lazyLoader = ${JSON.stringify(name)};`);
      write(path.join(dir, 'script-register.js'), '');
    }
    function checkScript(loader, extra = {}) {
      assert.deepEqual(JSON.parse(run(args, extra, 0, vpx).stdout), {
        args: ['value'],
        loader,
        node: version,
      });
    }
    const override = path.join(root, 'loader-override');
    writeLoader(override, 'override');
    checkScript('override', { VP_GLOBAL_CLI_JS_SCRIPTS_DIR: override });
    assert.equal(installCount(), 0);
    assert.ok(!fs.existsSync(payload));

    writeLoader(path.join(localPackage, 'dist'), 'local');
    checkScript('local', { TEST_INSTALL_FAIL: '1' });
    assert.equal(installCount(), 0);
    assert.ok(!fs.existsSync(payload));
    console.log('vpx uses explicit or project-local loaders without installing the global payload');

    fs.unlinkSync(path.join(localPackage, 'dist/script-register.js'));
    const failed = run(args, { TEST_INSTALL_FAIL: '1' }, 1, vpx);
    assert.match(failed.stderr, /Failed to install production dependencies/);
    assert.doesNotMatch(failed.stderr, /The script loader was not found/);
    assert.equal(installCount(), 1);
    assert.ok(fs.existsSync(pending));
    run(['env', '--help']);
    assert.equal(installCount(), 1);

    checkScript('global');
    assert.equal(installCount(), 2);
    checkInstalled();
    assert.deepEqual(JSON.parse(run(['--watch', ...args], {}, 0, vpx).stdout), {
      args,
      loader: 'global-watch',
      node: version,
    });
    assert.equal(installCount(), 2);
    console.log('vpx reports install failures and retries the global script loader on next use');
    console.log('Script and watch commands reuse the completed global payload');
  } else {
    if (action === 'global') write(path.join(project, '.node-version'), '16.20.2\n');
    const args =
      action === 'global'
        ? globalArgs
        : action === 'fallback'
          ? ['build']
          : ['toolchain', '--global', '--json'];
    const result = run(args);
    if (action === 'toolchain') {
      assert.equal(JSON.parse(result.stdout).source.scope, 'global');
    } else {
      checkOutput(result, args);
    }
    assert.equal(installCount(), 1);
    checkInstalled();
    if (action === 'global') {
      assert.equal(fs.readFileSync(path.join(project, '.node-version'), 'utf8'), '16.20.2\n');
      const current = JSON.parse(run(['env', 'current', 'node', '--json']).stdout);
      assert.equal(current.node.version, '16.20.2');
      assert.equal(current.node.installed, false);
    }
    checkOutput(run(globalArgs), globalArgs);
    assert.equal(installCount(), 1);
    console.log(`${action}: first use installs the global payload with the saved registry`);
    console.log('Later commands reuse it and stdout contains only command output');
  }
}

async function checkConcurrent(args) {
  let release;
  const entered = new Promise((resolve) => {
    release = resolve;
  });
  const server = http.createServer((_request, response) => release(response));
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const extra = {
    TEST_INSTALL_GATE: `http://127.0.0.1:${server.address().port}`,
    VP_LOG: 'vp_setup=debug',
  };
  const children = [];
  function start() {
    const child = spawn(external, args, { cwd: project, env: { ...env, ...extra } });
    children.push(child);
    let stdout = '';
    let stderr = '';
    let waiting;
    const lock = new Promise((resolve) => {
      waiting = resolve;
    });
    const checkLock = () => {
      if ((stdout + stderr).includes('Waiting for global JavaScript dependency installation lock'))
        waiting();
    };
    child.stdout.on('data', (data) => {
      stdout += data;
      checkLock();
    });
    child.stderr.on('data', (data) => {
      stderr += data;
      checkLock();
    });
    const result = new Promise((resolve, reject) => {
      child.on('error', reject);
      child.on('close', (code) =>
        code === 0 ? resolve({ stdout, stderr }) : reject(new Error(stderr)),
      );
    });
    return { lock, result };
  }
  try {
    const first = start();
    const response = await entered;
    const second = start();
    await second.lock;
    assert.equal(installCount(), 1);
    response.end('continue');
    for (const result of await Promise.all([first.result, second.result])) {
      // VP_LOG writes tracing to stdout; the final line belongs to the mock CLI.
      checkOutput({ stdout: result.stdout.trim().split('\n').at(-1) }, args);
    }
    assert.equal(installCount(), 1);
    checkInstalled();
    console.log('Concurrent commands share one successful dependency installation');
  } finally {
    for (const child of children) child.kill();
    server.closeAllConnections();
    server.close();
  }
}

// The runner must fail on an assertion even while recording snapshots.
// A missing completion milestone is a trial error, not recordable command output.
if (process.env.VP_EMIT_MILESTONES === '1') {
  const name = Buffer.from('lazy-setup:verify:complete').toString('base64url');
  process.stdout.write(`\x1b]2;vt-pty-terminal-test:${'0'.repeat(32)}:${name}\x1b\\`);
}
