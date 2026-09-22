const { execFileSync } = require('node:child_process');
const path = require('node:path');

const vp = path.join(path.dirname(require.resolve('vite-plus/package.json')), 'bin', 'vp');
// The runner's node shim can expand PATHEXT before this script starts.
// Reset it in a new process that launches real Node directly, so only the
// NAPI initializer can supply the lowercase extension for this CLI lookup.
execFileSync(process.execPath, [vp, 'exec', 'astro', '--version'], {
  cwd: path.resolve('packages/app'),
  env: { ...process.env, PATHEXT: '.COM;.EXE;.BAT;.CMD' },
  stdio: 'inherit',
  timeout: 30000,
});
