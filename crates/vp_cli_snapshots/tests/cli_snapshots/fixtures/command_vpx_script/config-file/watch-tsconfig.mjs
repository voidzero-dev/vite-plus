// Runs `vpx --watch` with `--tsconfig` after a separate config file path, prints the
// tsconfig the script used, and stops the watcher.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const args = ['--watch', '--experimental-config-file', 'custom.json', '--tsconfig', 'cli.json'];
// Passed here: on this script's command line, Node.js would read `--env-file` for it too.
const child = spawn('vpx', [...args, '--env-file=.env', './alias.ts'], {
  detached: true,
  stdio: ['ignore', 'pipe', 'inherit'],
});
const timer = setTimeout(() => {
  console.log('timeout: the script printed no tsconfig');
  process.kill(-child.pid, 'SIGKILL');
  process.exit(1);
}, 20_000);
for await (const line of createInterface({ input: child.stdout })) {
  if (line.startsWith('tsconfig ')) {
    console.log(`vpx ${args.join(' ')} --env-file=.env ./alias.ts: ${line}`);
    clearTimeout(timer);
    process.kill(-child.pid, 'SIGTERM');
    break;
  }
}
