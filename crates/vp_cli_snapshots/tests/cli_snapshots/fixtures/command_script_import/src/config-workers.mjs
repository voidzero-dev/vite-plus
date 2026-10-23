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
  return {
    value,
    writes: require('./config-fields.cts').writes,
    environment: Object.fromEntries(selectors.map(key => [key, process.env[key] ?? null])),
    sameEnvironment: selectors.every((key, index) => process.env[key] === original[index]),
  };
}

async function pair(configurations) {
  let readyCount = 0;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const tasks = configurations.map(async (overrides) => {
    const env = { ...process.env };
    for (const key of selectors) delete env[key];
    Object.assign(env, overrides);
    const worker = new Worker(new URL('./config-worker.mts', import.meta.url), { env });
    const signal = AbortSignal.timeout(15000);
    try {
      const [ready] = await once(worker, 'message', { signal });
      if (++readyCount === configurations.length) release();
      await gate;
      const result = once(worker, 'message', { signal });
      worker.postMessage('run');
      return { ready, result: (await result)[0] };
    } finally {
      await worker.terminate();
    }
  });
  // Attach rejection handlers immediately, including when a worker fails before the gate.
  const completed = Promise.all(tasks);
  await Promise.race([gate, completed]);
  const [workers, parentDuring] = await Promise.all([completed, checkParent()]);
  const parentAfter = await checkParent();
  return { workers, parentDuring, parentAfter };
}

const parentBefore = await checkParent();
const compat = await pair([
  { TSX_TSCONFIG_PATH: one },
  { TSX_TSCONFIG_PATH: two },
]);
// Worker environments differ from the main thread's OS environment. In particular,
// no missing variable in a worker may fall back to the parent's native getenv().
const upstream = await pair([
  { TS_NODE_PROJECT: one, OXC_TSCONFIG_PATH: two },
  { OXC_TSCONFIG_PATH: two },
]);
const explicitAndDefault = await pair([
  { VP_SCRIPT_TSCONFIG: one, TSX_TSCONFIG_PATH: two },
  {},
]);
console.log(JSON.stringify({ parentBefore, compat, upstream, explicitAndDefault }, null, 2));
