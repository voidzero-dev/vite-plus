const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { copyFileSync, mkdirSync, writeFileSync } = require('node:fs');
const path = require('node:path');

assert.ok(process.env.VP_PATH_INJECTED_TOOLS.split(',').includes('node'));
const bin = path.join(process.env.VP_HOME, 'bin');
const paths = [bin];
if (process.argv[2] === 'trampoline') {
  const other = path.resolve('other-installation');
  mkdirSync(other);
  const exe = path.join(other, process.platform === 'win32' ? 'node.exe' : 'node');
  if (process.platform === 'win32') {
    copyFileSync(path.join(bin, 'node.exe'), exe);
  } else {
    writeFileSync(exe, '#!/bin/sh\necho "Unexpected trampoline execution" >&2\nexit 99\n', {
      mode: 0o755,
    });
  }
  writeFileSync(
    path.join(other, 'node.shim'),
    `vite-plus-shim-v1\nlayout=single-root\ndata=${process.env.VP_HOME}\n`,
  );
  paths.push(other, path.dirname(process.execPath));
}
const version = execFileSync('node', ['--version'], {
  env: { ...process.env, PATH: paths.join(path.delimiter) },
  encoding: 'utf8',
  timeout: 10000,
}).trim();
assert.equal(version, process.version);
console.log(`Node selection survives ${process.argv[2]} PATH`);
