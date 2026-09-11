const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { chmodSync, copyFileSync, mkdirSync } = require('node:fs');
const path = require('node:path');

const runtimeDir = path.resolve('custom-runtime');
mkdirSync(runtimeDir);
const runtime = path.join(runtimeDir, process.platform === 'win32' ? 'custom node.exe' : 'custom node');
copyFileSync(process.execPath, runtime);
chmodSync(runtime, 0o755);
const vp = path.join(path.dirname(require.resolve('vite-plus/package.json')), 'bin', 'vp');
const env = { ...process.env };
for (const key of Object.keys(env)) {
  if (key.toUpperCase() === 'PATH') delete env[key];
}
env.PATH = runtimeDir;

for (const tool of ['lint', 'fmt']) {
  const child = spawnSync(runtime, [vp, tool, '--version'], { env, encoding: 'utf8', timeout: 30000 });
  assert.equal(child.status, 0, child.stderr || child.error?.message);
  assert.match(child.stdout, /\d+\.\d+\.\d+/);
  console.log(`${tool} reused the current runtime without node on PATH`);
}
