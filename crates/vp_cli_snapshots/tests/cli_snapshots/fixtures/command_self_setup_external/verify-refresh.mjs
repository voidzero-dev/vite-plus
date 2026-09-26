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
    PATH: [
      path.join(home, 'bin'),
      path.join(directory, 'brew/bin'),
      system,
      env.PATH,
      path.join(home, 'fallback-bin'),
    ].join(path.delimiter),
  };
}

function run(binary, args, cwd, env, expectedStatus = 0) {
  const result = spawnSync(binary, args, { cwd, env, encoding: 'utf8', timeout: 30000 });
  assert.equal(
    result.status,
    expectedStatus,
    result.error?.message ?? result.stdout + result.stderr,
  );
  return result.stdout;
}

function readSettings(env) {
  return JSON.parse(fs.readFileSync(path.join(env.VP_HOME, 'config.json'), 'utf8'));
}

function verifyHomebrewSources(source) {
  for (const [label, tap, name, sourceLabel] of [
    ['core', 'homebrew/core', 'vite-plus', 'Homebrew Core'],
    ['official', 'voidzero-dev/vite-plus', 'vp', 'Vite+ Homebrew tap'],
    ['other', 'example/tools', 'vp', 'Homebrew'],
    ['unknown', null, 'vite-plus', 'Homebrew'],
  ]) {
    const directory = path.resolve(`source-${label}`);
    const prefix = path.join(directory, 'Cellar', name, 'test');
    const binary = createBundle(source, prefix, 'bundled CLI');
    const formula = tap && tap !== 'homebrew/core' ? `${tap}/${name}` : name;
    fs.writeFileSync(
      path.join(prefix, 'INSTALL_RECEIPT.json'),
      JSON.stringify({
        homebrew_version: '7.0.2',
        source: { tap, path: `/tap/Formula/${name}.rb` },
      }),
    );
    const env = createEnvironment(directory);
    run(binary, ['--help'], directory, { ...env, VP_NODE_MANAGER: 'no', VP_PM_MANAGER: 'no' });
    const invoke = (args, expectedStatus = 0) => {
      const result = spawnSync(binary, args, { cwd: directory, env, encoding: 'utf8' });
      const text = (result.stdout + result.stderr).replace(/\u001b\[[0-9;]*m/g, '').trim();
      assert.equal(result.status, expectedStatus, text);
      return text;
    };
    console.log(`${label} installation`);
    const doctor = invoke(['env', 'doctor', 'node']);
    assert.ok(doctor.includes(sourceLabel), doctor);
    console.log(
      doctor
        .split('\n')
        .filter((line) => /CLI source|CLI formula/.test(line))
        .join('\n'),
    );
    for (const [args, status, action] of [
      [['upgrade'], 1, 'upgrade'],
      [['upgrade', '--check'], 0, 'outdated'],
    ]) {
      const text = invoke(args, status);
      assert.ok(text.includes(`${sourceLabel} manages this installation`), text);
      assert.ok(text.includes(`brew ${action} ${formula}`), text);
      assert.equal(text.includes('To switch to the official Vite+ tap'), label === 'core', text);
      console.log(text);
    }
    assert.equal(invoke(['upgrade', '--check', '--silent']), '');
    assert.ok(!invoke(['upgrade', '--silent'], 1).includes('To switch'));
    const removed = invoke(['implode', '--yes']);
    const notice = `The ${sourceLabel} package remains installed. Run \`brew uninstall ${formula}\` to remove it.`;
    assert.ok(removed.includes(notice), removed);
    assert.ok(fs.existsSync(binary));
    assert.ok(!fs.existsSync(env.VP_HOME));
    console.log(notice);
  }
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
  const fallbackBin = path.join(env.VP_HOME, 'fallback-bin');
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
        { ...env, PATH: [...paths, fallbackBin].join(path.delimiter) },
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

function verifyHomebrewCollision(source) {
  for (const layout of ['direct', 'directory-alias', 'fallback-alias']) {
    for (const refresh of [false, true]) {
      const directory = path.resolve(`${layout}-${refresh}`);
      const prefix = path.join(directory, 'brew/Cellar/vite-plus/old');
      createBundle(source, prefix, 'bundled CLI');
      fs.writeFileSync(path.join(prefix, 'INSTALL_RECEIPT.json'), '{"homebrew_version":"7.0.2"}');
      const publicBin = path.join(directory, 'brew/bin');
      const publicBinary = path.join(publicBin, 'vp');
      const link = '../Cellar/vite-plus/old/bin/vp';
      fs.mkdirSync(publicBin, { recursive: true });
      fs.symlinkSync(link, publicBinary);
      const env = { ...createEnvironment(directory), ...mixed };
      const data = path.join(directory, 'data');
      fs.mkdirSync(data);
      const splitEnv = {
        ...env,
        VP_HOME: undefined,
        VP_BIN_DIR: path.join(directory, 'user-bin'),
        VP_DATA_DIR: data,
        VP_CACHE_DIR: path.join(directory, 'cache'),
      };
      if (refresh) run(publicBinary, ['--help'], directory, splitEnv);
      let bin = publicBin;
      if (layout === 'directory-alias') {
        bin = path.join(directory, 'bin-alias');
        fs.symlinkSync(publicBin, bin);
      } else if (layout === 'fallback-alias') {
        bin = splitEnv.VP_BIN_DIR;
        fs.rmSync(path.join(data, 'fallback-bin'), { recursive: true, force: true });
        fs.symlinkSync(publicBin, path.join(data, 'fallback-bin'));
      }
      const result = spawnSync(publicBinary, refresh ? ['env', 'setup', '--refresh'] : ['--help'], {
        cwd: directory,
        env: { ...splitEnv, VP_BIN_DIR: bin },
        encoding: 'utf8',
        timeout: 30000,
      });
      assert.equal(fs.readlinkSync(publicBinary), link, 'Homebrew entrypoint was replaced');
      assert.equal(result.status, 1, result.error?.message ?? result.stdout + result.stderr);
      assert.ok(result.stderr.includes('shim directories'), result.stderr);
      run(publicBinary, ['--help'], directory, env);
    }
    console.log(`${layout}: first setup and refresh preserve Homebrew's entrypoint`);
  }
}

function verifyHomebrewMigration(source) {
  for (const refresh of [false, true]) {
    const directory = path.resolve(`migration-${refresh}`);
    const old = path.join(directory, 'brew/Cellar/vite-plus/old');
    const next = path.join(directory, 'brew/Cellar/vp/new');
    const previous = createBundle(source, old, 'old CLI');
    const replacement = createBundle(source, next, 'new CLI');
    for (const prefix of [old, next]) {
      fs.writeFileSync(path.join(prefix, 'INSTALL_RECEIPT.json'), '{"homebrew_version":"7.0.2"}');
    }
    const publicBinary = path.join(directory, 'brew/bin/vp');
    fs.mkdirSync(path.dirname(publicBinary), { recursive: true });
    fs.symlinkSync(previous, publicBinary);
    const env = { ...createEnvironment(directory), ...mixed };
    run(publicBinary, ['--help'], directory, env);
    fs.unlinkSync(publicBinary);
    fs.symlinkSync(replacement, publicBinary);
    if (refresh) run(publicBinary, ['--help'], directory, env);

    const bin = path.join(env.VP_HOME, 'bin');
    const configs = path.join(env.VP_HOME, 'bins');
    fs.mkdirSync(configs);
    const links = {
      absolute: previous,
      relative: path.relative(bin, previous),
      foreign: path.join(directory, 'system/bin/node'),
      dangling: path.join(directory, 'missing/tool'),
      npm: previous,
      untracked: previous,
    };
    for (const name of [...Object.keys(links), 'file', 'directory', 'absent']) {
      if (name !== 'untracked') {
        fs.writeFileSync(
          path.join(configs, `${name}.json`),
          JSON.stringify({
            name,
            package: 'fixture',
            version: '1.0.0',
            nodeVersion: process.versions.node,
            source: name === 'npm' ? 'npm' : 'vp',
          }),
        );
      }
    }
    for (const [name, target] of Object.entries(links))
      fs.symlinkSync(target, path.join(bin, name));
    fs.writeFileSync(path.join(bin, 'file'), 'foreign executable');
    fs.mkdirSync(path.join(bin, 'directory'));
    fs.rmSync(old, { recursive: true });
    for (const name of ['absolute', 'relative']) assert.ok(!fs.existsSync(path.join(bin, name)));

    run(publicBinary, refresh ? ['env', 'setup', '--refresh'] : ['--help'], directory, env);
    for (const name of ['absolute', 'relative']) {
      assert.equal(fs.readlinkSync(path.join(bin, name)), publicBinary);
      assert.equal(fs.realpathSync(path.join(bin, name)), fs.realpathSync(replacement));
      assert.ok(fs.existsSync(path.join(configs, `${name}.json`)));
    }
    for (const name of ['foreign', 'dangling', 'npm', 'untracked']) {
      assert.equal(fs.readlinkSync(path.join(bin, name)), links[name]);
    }
    assert.equal(fs.readFileSync(path.join(bin, 'file'), 'utf8'), 'foreign executable');
    assert.ok(fs.statSync(path.join(bin, 'directory')).isDirectory());
    assert.ok(!fs.existsSync(path.join(bin, 'absent')));
    console.log(
      `${refresh ? 'Explicit refresh' : 'First launch'} repairs owned links after core keg removal`,
    );
  }
  console.log(
    'Foreign files, directories, package links, npm-owned links, and untracked links stay unchanged',
  );
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

  const source = process.env.TEST_VP_BINARY ?? path.join(process.env.VP_HOME, 'bin/vp');
  if (action === 'homebrew-sources') {
    verifyHomebrewSources(source);
    return;
  }
  if (action === 'homebrew-collision') {
    verifyHomebrewCollision(source);
    return;
  }
  if (action === 'homebrew-migration') {
    verifyHomebrewMigration(source);
    return;
  }
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
