import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { Worker } from 'node:worker_threads';

const require = createRequire(import.meta.url);
const selectors = ['VP_SCRIPT_TSCONFIG', 'TSX_TSCONFIG_PATH', 'TS_NODE_PROJECT', 'OXC_TSCONFIG_PATH'];
const original = selectors.map(key => process.env[key]);
const one = 'configs with spaces/worker-one.json';
const two = 'configs with spaces/worker-two.json';
let checks = 0;
async function checkParent() {
  const { value } = await import(`./config-parent-check.mts?check=${checks++}`);
  delete require.cache[require.resolve('./config-fields.cts')];
  assert.equal(value, 'parent');
  assert.equal(require('./config-fields.cts').writes, 0);
  assert.deepEqual(selectors.map(key => process.env[key]), original);
}

async function pair(configurations) {
  let readyCount = 0;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const tasks = configurations.map(async ({ env: overrides, value, writes }) => {
    const env = { ...process.env };
    for (const key of selectors) delete env[key];
    Object.assign(env, overrides);
    const worker = new Worker(new URL('./config-worker.mts', import.meta.url), { env });
    const signal = AbortSignal.timeout(15000);
    try {
      const [ready] = await once(worker, 'message', { signal });
      assert.deepEqual(ready, { stage: 'ready', initial: value });
      if (++readyCount === configurations.length) release();
      await gate;
      const result = once(worker, 'message', { signal });
      worker.postMessage('run');
      assert.deepEqual((await result)[0], { stage: 'result', value, writes });
    } finally {
      await worker.terminate();
    }
  });
  // Attach rejection handlers immediately, including when a worker fails before the gate.
  const completed = Promise.all(tasks);
  await Promise.race([gate, completed]);
  await Promise.all([completed, checkParent()]);
  await checkParent();
}

await checkParent();
await pair([
  { env: { TSX_TSCONFIG_PATH: one }, value: 'worker-one', writes: 1 },
  { env: { TSX_TSCONFIG_PATH: two }, value: 'worker-two', writes: 0 },
]);
// Worker environments differ from the main thread's OS environment. In particular,
// no missing variable in a worker may fall back to the parent's native getenv().
await pair([
  { env: { TS_NODE_PROJECT: one, OXC_TSCONFIG_PATH: two }, value: 'worker-one', writes: 1 },
  { env: { OXC_TSCONFIG_PATH: two }, value: 'worker-two', writes: 0 },
]);
await pair([
  { env: { VP_SCRIPT_TSCONFIG: one, TSX_TSCONFIG_PATH: two }, value: 'worker-one', writes: 1 },
  { env: {}, value: 'default', writes: 0 },
]);
console.log('concurrent worker configs isolate ESM aliases and CJS transforms from the parent');
