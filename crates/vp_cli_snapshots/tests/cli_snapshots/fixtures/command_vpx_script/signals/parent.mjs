// Starts `vpx` the way a tool does and sends SIGINT to the PID it got back. Both CLIs
// exec Node.js in place on Unix, so that PID is the script's and the script receives it.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createInterface } from 'node:readline';

async function interrupt(script) {
  const child = spawn('vpx', [script], { stdio: ['inherit', 'pipe', 'inherit'] });
  const exited = once(child, 'exit');
  for await (const line of createInterface({ input: child.stdout })) {
    const pid = /^ready (\d+)$/.exec(line)?.[1];
    if (pid === undefined) {
      console.log(`${script}: ${line}`);
      continue;
    }
    console.log(`${script}: the script runs in the vpx process: ${Number(pid) === child.pid}`);
    child.kill('SIGINT');
  }
  const [code, signal] = await exited;
  console.log(`${script}: exit code ${code}, signal ${signal}`);
}

const timer = setTimeout(() => {
  console.log('timeout: the script kept running');
  process.exit(1);
}, 20_000);
await interrupt('./wait.ts');
await interrupt('./graceful.ts');
clearTimeout(timer);
