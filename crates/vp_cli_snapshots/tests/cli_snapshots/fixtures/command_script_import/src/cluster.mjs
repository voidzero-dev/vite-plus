import cluster from 'node:cluster';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';

cluster.setupPrimary({ exec: fileURLToPath(new URL('./child.ts', import.meta.url)) });
const child = cluster.fork();
const exited = once(child, 'exit');
const [message] = await once(child, 'message');
const [exitCode] = await exited;
console.log(JSON.stringify({ message, exitCode }, null, 2));
