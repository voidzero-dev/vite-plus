import { spawnSync } from 'node:child_process';

// The parent supplies its identity so the payload can report any extra process hop.
const child = spawnSync(process.execPath, process.argv.slice(2), {
  env: {
    ...process.env,
    EXPECTED_EXEC_PATH: process.execPath,
    EXPECTED_PARENT_PID: String(process.pid),
  },
  stdio: 'inherit',
  timeout: 30000,
});
if (child.error) throw child.error;
process.exitCode = child.status ?? 1;
