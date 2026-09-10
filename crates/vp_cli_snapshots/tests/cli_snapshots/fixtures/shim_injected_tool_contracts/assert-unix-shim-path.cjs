const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { copyFileSync, mkdirSync, symlinkSync } = require('node:fs');
const path = require('node:path');

const mode = process.argv[2];
assert.equal(process.version, mode === 'system' ? 'v20.18.0' : 'v22.18.0');
const bin = path.join(process.env.VP_HOME, 'bin');
const paths = [];
if (mode === 'multiple') {
  for (const name of ['installation-a', 'installation-b']) {
    const dir = path.resolve(name);
    mkdirSync(dir);
    copyFileSync(path.join(bin, 'vp'), path.join(dir, 'vp'));
    symlinkSync('vp', path.join(dir, 'node'));
    paths.push(dir);
  }
  paths.push(path.dirname(process.execPath));
} else {
  const dir = path.resolve('node-only');
  mkdirSync(dir);
  symlinkSync(process.execPath, path.join(dir, 'node'));
  paths.push(bin, dir);
  if (mode === 'system') {
    const systemNode = execFileSync(
      'vp',
      ['env', 'exec', '--node', '22.18.0', 'node', '-p', 'process.execPath'],
      { encoding: 'utf8' },
    ).trim();
    paths.push(path.dirname(systemNode));
  }
  paths.push('/usr/bin', '/bin');
}
const env = { ...process.env, PATH: paths.join(path.delimiter) };
if (mode === 'system') {
  // Only the selected Node is inherited; npm/npx now come from another runtime.
  env.VP_PATH_INJECTED_TOOLS = 'node';
}
const version = execFileSync('node', ['--version'], {
  env,
  encoding: 'utf8',
  timeout: 10000,
}).trim();
assert.equal(version, process.version);
if (mode === 'partial') {
  for (const tool of ['npm', 'npx']) {
    assert.equal(
      execFileSync(tool, ['--version'], { env, encoding: 'utf8', timeout: 10000 }).trim(),
      process.argv[3] || '10.9.3',
    );
  }
}
if (mode === 'system') {
  for (const tool of ['npm', 'npx']) {
    const args = ['--offline', '--call', 'node --version'];
    if (tool === 'npm') args.unshift('exec');
    assert.equal(
      execFileSync(tool, args, { env, encoding: 'utf8', timeout: 10000 }).trim(),
      process.version,
    );
  }
}
console.log(`Node and its tools survive ${mode} PATH`);
