const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const [manager, mode, template] = process.argv.slice(2);
const systemVersion = manager === 'yarn' && (template === 'external' || template === 'classic') ? '1.22.22' : { npm: '10.9.3', pnpm: '10.18.0', yarn: '4.9.2', bun: '1.2.3' }[manager];
const managedVersion = manager === 'npm' ? '11.6.0' : '10.18.0';
const env = { ...process.env };
if (template === 'external' && process.platform !== 'win32') {
  // The runner exposes sh through a symlink. Use its canonical system path so
  // fspy can substitute an injectable shell on macOS when npm runs the template.
  env.npm_config_script_shell = '/bin/sh';
}
const entryVp = env.PATH.split(path.delimiter).map(dir => path.join(dir, 'vp')).find(file => fs.existsSync(file));
const globalVp = path.join(env.VP_HOME, 'bin', 'vp');
function run(args, envs = env) {
  const result = spawnSync(args[0] === 'env' ? globalVp : entryVp, args, { env: envs, encoding: 'utf8' });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, result.stdout + result.stderr);
  return result.stdout;
}

if (mode !== 'missing') {
  // Expose a real tool through a separate system PATH entry. Templates can
  // invoke the tool as well as query its version, so a version-only stub is insufficient.
  run(['env', 'install', `${manager}@${systemVersion}`]);
  const selection = JSON.parse(run(['env', 'current', manager, '--json'], {
    ...env, [`VP_${manager.toUpperCase()}_VERSION`]: systemVersion,
  })).package_manager;
  const binDir = path.resolve('system-bin');
  fs.mkdirSync(binDir);
  fs.writeFileSync(path.join(binDir, manager), `#!${process.execPath}\nconst result = require('node:child_process').spawnSync(${JSON.stringify(selection.bin_paths[manager])}, process.argv.slice(2), { stdio: 'inherit' });\nif (result.error) throw result.error;\nprocess.exit(result.status ?? 1);\n`, { mode: 0o755 });
  env.PATH = [path.join(env.VP_HOME, 'bin'), binDir, env.PATH].join(path.delimiter);
}
if (mode !== 'system') {
  fs.writeFileSync('package.json', JSON.stringify({ private: true, packageManager: `${manager}@${managedVersion}` }));
}
run(['env', mode === 'managed' ? 'on' : 'off', manager]);
const current = JSON.parse(run(['env', 'current', manager, '--json'])).package_manager;
if (mode === 'system') assert.equal(current.source, 'system PATH');
else assert.notEqual(current.source, 'system PATH');
const templateArgs = template === 'external' ? ['vite'] : ['vite:application', '--directory', 'app'];
const output = run(['create', ...templateArgs, '--package-manager', manager,
  '--no-interactive', '--no-agent', '--no-editor', '--no-hooks', '--no-git',
  ...(template === 'external' ? ['--', 'app', '--template', 'vanilla'] : [])]);
if (template === 'external') {
  assert.match(output, /Running: npx --yes create-vite/);
  const pkg = JSON.parse(fs.readFileSync('app/package.json', 'utf8'));
  assert.equal(pkg.name, 'app');
  assert.ok(fs.existsSync('app/index.html'));
  assert.equal(current.version, systemVersion);
  console.log(`${manager}: external template generated with npx`);
  process.exit(0);
}
const pin = JSON.parse(fs.readFileSync('app/package.json', 'utf8')).devEngines.packageManager;
assert.equal(pin.name, manager);
assert.equal(pin.version, mode === 'system' ? systemVersion : managedVersion);
console.log(`${manager}: ${mode === 'system' ? 'system version pinned' : 'managed version pinned'}`);
