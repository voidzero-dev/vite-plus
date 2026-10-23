import assert from 'node:assert/strict';
import cluster from 'node:cluster';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';

cluster.setupPrimary({ exec: fileURLToPath(new URL('./child.ts', import.meta.url)) });
const child = cluster.fork();
const exited = once(child, 'exit');
const [message] = await once(child, 'message');
assert.equal(message, 'fork: compat');
assert.equal((await exited)[0], 0);
console.log('cluster inherits registration and custom tsconfig');
