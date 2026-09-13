const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { mkdirSync, symlinkSync, writeFileSync } = require('node:fs');
const path = require('node:path');

assert.equal(process.version, 'v22.18.0');
if (process.argv[2] === 'setup') {
  mkdirSync('manager');
  mkdirSync('system-shims');
  const runtime = path.dirname(process.execPath).replaceAll("'", "'\\''");
  writeFileSync(
    'manager/tool-manager',
    `#!/bin/sh\nexec '${runtime}/'"\${0##*/}" "$@"\n`,
    { mode: 0o755 },
  );
  for (const tool of ['vp', 'node', 'npm', 'npx']) {
    symlinkSync('../manager/tool-manager', `system-shims/${tool}`);
  }
} else {
  assert.equal(process.env.VP_BYPASS, undefined);
  assert.equal(
    execFileSync('node', ['--version'], { encoding: 'utf8', timeout: 10000 }).trim(),
    process.version,
  );
  const preload = process.argv[2] === 'preload';
  const options = {
    encoding: 'utf8',
    timeout: 10000,
    env: preload
      ? { ...process.env, NODE_OPTIONS: '--require ./preload-output.cjs' }
      : process.env,
  };
  for (const tool of ['npm', 'npx']) {
    assert.equal(
      execFileSync(tool, ['--version'], options).trim(),
      preload ? 'preload start\n10.9.3\npreload exit' : '10.9.3',
    );
    const args = ['--offline', '--call', 'node -e "console.log(process.version)"'];
    if (tool === 'npm') args.unshift('exec');
    assert.equal(
      execFileSync(tool, args, options).trim(),
      preload
        ? `preload start\npreload start\n${process.version}\npreload exit\npreload exit`
        : process.version,
    );
  }
  console.log('Bundled npm/npx use the runtime behind the system Node shim');
}
