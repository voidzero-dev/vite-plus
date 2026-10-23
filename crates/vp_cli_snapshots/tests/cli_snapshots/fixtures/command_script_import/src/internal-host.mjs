import assert from 'node:assert/strict';
import { once } from 'node:events';
import { register } from 'node:module';
import { MessageChannel } from 'node:worker_threads';

const { port1, port2 } = new MessageChannel();
const message = once(port1, 'message', { signal: AbortSignal.timeout(15000) });
register(new URL('./internal-loader.mjs', import.meta.url), {
  parentURL: import.meta.url, data: { port: port2 }, transferList: [port2],
});
assert.deepEqual((await message)[0], { internal: true, unchanged: true });
port1.close();
console.log('the internal loader thread skips public registration');
