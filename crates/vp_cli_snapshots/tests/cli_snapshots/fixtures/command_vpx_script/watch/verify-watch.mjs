// Drives `vpx --watch`: edit an imported .ts file and wait for the restart.
import { spawn } from 'node:child_process';
import { writeFile } from 'node:fs/promises';

const child = spawn('vpx', ['--watch', './main.ts'], { stdio: ['ignore', 'pipe', 'pipe'] });
let logs = '';
const waiters = [];
const onData = (data) => {
  logs += data;
  for (const waiter of waiters) {
    waiter();
  }
};
child.stdout.on('data', onData);
child.stderr.on('data', onData);

function waitFor(text) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`timed out waiting for ${text}`)), 20000);
    const check = () => {
      if (logs.includes(text)) {
        clearTimeout(timer);
        resolve();
      }
    };
    waiters.push(check);
    check();
  });
}

try {
  await waitFor('value first');
  await writeFile('dep.ts', "export const value: string = 'second';\n");
  await waitFor('value second');
  console.log('vpx --watch restarted after an imported .ts file changed');
} catch (error) {
  console.error(logs);
  throw error;
} finally {
  await writeFile('dep.ts', "export const value: string = 'first';\n");
  child.kill();
}
