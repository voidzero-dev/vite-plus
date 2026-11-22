import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const binDir = fileURLToPath(new URL('./bin/', import.meta.resolve('vite-plus/package.json')));
const env = { ...process.env };
const pathKey = Object.keys(env).find((key) => key.toUpperCase() === 'PATH');
const vpExecutables = ['vp', 'vp.exe', 'vp.cmd', 'vp.bat'];
const pathsWithoutVp = env[pathKey]
  .split(path.delimiter)
  .filter((dir) => !vpExecutables.some((bin) => existsSync(path.join(dir, bin))));
env[pathKey] = [path.dirname(process.execPath), ...pathsWithoutVp].join(path.delimiter);
delete env.VP_CLI_BIN;
// A local invocation must use its own package version, even under another vp.
env.VP_USER_AGENT = 'vp/0.0.0-parent';
env.VP_GLOBAL_VERSION = '0.0.0-parent';
assert.equal(spawnSync('vp', ['--version'], { env }).error?.code, 'ENOENT');

for (const [bin, args] of [
  ['vp', ['exec', 'node', 'check-agent.cjs', 'current']],
  ['vpr', ['check-agent']],
  ['vpx', ['./check-agent.cjs', 'current']],
  ['vp', ['dlx', '-s', '-p', 'vp-test-echo', '-c', 'node check-agent.cjs current pnpm']],
]) {
  const result = spawnSync(process.execPath, [path.join(binDir, bin), ...args], {
    env,
    encoding: 'utf8',
    timeout: 120000,
  });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}\n${result.error ?? ''}`);
  assert.match(result.stdout, /Node descendants preserve the marker\./);
  console.log(`${bin} ${args[0]} advertises the local version without global vp.`);
}
