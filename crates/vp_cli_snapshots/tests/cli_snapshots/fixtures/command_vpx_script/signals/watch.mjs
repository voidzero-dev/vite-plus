// `vpx --watch` runs the watcher in the vpx process and the script in a child of it. A
// SIGINT sent to the vpx PID alone is passed on to the script. Ctrl+C from a terminal
// signals the whole process group, so the script has it already; it tells the watcher,
// which then does not send a second one.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createInterface } from 'node:readline';

const WATCHER_LOG = /^\d\d:\d\d:\d\d \[vpx\] /;

async function interrupt(target) {
  // Its own process group, as a terminal gives a job, without a terminal on stdin.
  const child = spawn('vpx', ['--watch', './count.ts'], {
    detached: true,
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const exited = once(child, 'exit');
  for await (const line of createInterface({ input: child.stdout })) {
    const pid = /^ready (\d+)$/.exec(line)?.[1];
    if (pid === undefined) {
      if (!WATCHER_LOG.test(line)) {
        console.log(`${target}: ${line}`);
      }
      continue;
    }
    console.log(`${target}: the script runs in a child of vpx: ${Number(pid) !== child.pid}`);
    process.kill(target === 'group' ? -child.pid : child.pid, 'SIGINT');
  }
  const [code, signal] = await exited;
  console.log(`${target}: exit code ${code}, signal ${signal}`);
}

const timer = setTimeout(() => {
  console.log('timeout: the script kept running');
  process.exit(1);
}, 20_000);
await interrupt('pid');
await interrupt('group');
clearTimeout(timer);
