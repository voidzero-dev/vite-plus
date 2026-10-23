import { spawnSync } from 'node:child_process';

const child = spawnSync(process.execPath, ['./src/main.ts'], {
  encoding: 'utf8',
  timeout: 15000,
});
if (child.error) throw child.error;
console.log(JSON.stringify({ execArgv: process.execArgv, exitCode: child.status, signal: child.signal }, null, 2));
process.stdout.write(child.stdout);
process.stderr.write(child.stderr);
