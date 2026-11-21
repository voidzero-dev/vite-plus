import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

// Each setting ties a debugger session or exit-time output to the launcher.
const settings = [
  { name: 'NODE_OPTIONS=--inspect', env: () => ({ NODE_OPTIONS: '--inspect=127.0.0.1:0' }) },
  {
    name: 'NODE_OPTIONS=--cpu-prof',
    env: (dir) => ({ NODE_OPTIONS: `--cpu-prof --cpu-prof-dir=${dir}` }),
    output: (pid) => new RegExp(`^CPU\\.\\d+\\.\\d+\\.${pid}\\.\\d+\\.\\d+\\.cpuprofile$`),
  },
  {
    name: 'NODE_V8_COVERAGE',
    env: (dir) => ({ NODE_V8_COVERAGE: dir }),
    output: (pid) => new RegExp(`^coverage-${pid}-.*\\.json$`),
  },
];
const args = ['dev', '--host', '127.0.0.1', '--port', '0'];

async function vitePid(stdout) {
  for await (const line of createInterface({ input: stdout })) {
    const match = /^listening \d+ (\d+)$/.exec(line);
    if (match) {
      return Number(match[1]);
    }
  }
  assert.fail('Vite did not start listening');
}

async function check(setting) {
  const dir = mkdtempSync(join(tmpdir(), 'vp-dev-diagnostics-'));
  const extra = setting.env(dir);
  const env = { ...process.env, ...extra };
  if (extra.NODE_OPTIONS) {
    env.NODE_OPTIONS = [extra.NODE_OPTIONS, process.env.NODE_OPTIONS].filter(Boolean).join(' ');
  }
  // The inspector prints a random URL to stderr, so keep stderr out of the snapshot.
  const child = spawn('vp', args, { stdio: ['ignore', 'pipe', 'pipe'], env });
  let stderr = '';
  child.stderr.on('data', (chunk) => (stderr += chunk));
  try {
    const pid = await vitePid(child.stdout);
    assert.notEqual(pid, child.pid, `${setting.name}: Vite replaced the launcher`);
    process.kill(pid, 'SIGTERM');
    const [code, signal] = await once(child, 'exit', { signal: AbortSignal.timeout(5_000) });
    assert.deepEqual([code, signal], [143, null]);
    let message = `${setting.name} kept Vite as a child process`;
    if (setting.output) {
      const pattern = setting.output(child.pid);
      assert.ok(
        readdirSync(dir).some((name) => pattern.test(name)),
        `${setting.name}: the launcher wrote its output`,
      );
      message += ', and the launcher wrote its output';
    }
    console.log(message);
  } catch (error) {
    process.stderr.write(stderr);
    throw error;
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill('SIGKILL');
    }
    rmSync(dir, { recursive: true, force: true });
  }
}

for (const setting of settings) {
  await check(setting);
}
