// Starts `vpx ./child.ts` with an IPC channel, as tools that fork a TypeScript runner do,
// and checks that messages reach the script and come back.
import { spawn } from 'node:child_process';
import { once } from 'node:events';

const child = spawn('vpx', ['./child.ts'], { stdio: ['inherit', 'inherit', 'inherit', 'ipc'] });
const timer = setTimeout(() => {
  console.log('timeout: no IPC message');
  child.kill();
  process.exit(1);
}, 20_000);

child.send('ping');
const [reply] = await once(child, 'message');
console.log('reply', JSON.stringify(reply));

let received = 0;
child.on('message', () => received++);
child.send('burst');
const [code] = await once(child, 'exit');
await new Promise((resolve) => setImmediate(resolve));
console.log('messages before exit', received, 'exit code', code);

// The parent closes the channel first; the script's exit code still comes back.
const second = spawn('vpx', ['./wait-for-disconnect.ts'], {
  stdio: ['inherit', 'inherit', 'inherit', 'ipc'],
});
second.send('ping');
await once(second, 'message');
second.disconnect();
const [secondCode] = await once(second, 'exit');
console.log('exit code after the parent disconnects', secondCode);
clearTimeout(timer);
