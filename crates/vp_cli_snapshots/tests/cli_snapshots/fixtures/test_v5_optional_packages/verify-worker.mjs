import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

const result = spawnSync(process.execPath, ['node_modules/vite-plus/dist/bin.js', 'test', 'run', '--config', 'worker-failure.config.js'], {
  encoding: 'utf8', env: process.env,
});
assert.equal(result.status, 1, `${result.stdout}\n${result.stderr}`);
const output = `${result.stdout}\n${result.stderr}`;
assert.match(output, /bad option: --vitest-invalid-worker-option/);
assert.match(output, /Worker exited unexpectedly with exit code 9 during starting state/);
assert.match(output, /Vitest caught 1 unhandled error/);
// Keep the actual diagnostic and status; omit OS-specific Node paths and stacks.
console.log('Vitest caught 1 unhandled error during the test run.');
console.log(output.match(/Worker exited unexpectedly with exit code 9 during starting state/)[0]);
process.exitCode = result.status;
