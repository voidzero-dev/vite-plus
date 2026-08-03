const fs = require('fs');
const path = require('path');
const { spawn, spawnSync } = require('child_process');
const { promisify } = require('util');

const sleep = promisify(setTimeout);
const readyPath = path.join(process.env.VP_HOME, 'env-install-interrupt-ready');
fs.rmSync(readyPath, { force: true });

const child = spawn('vp', ['install', '-g', './long-time-install-package'], {
  env: { ...process.env, VP_TEST_INTERRUPT_INSTALL: '1' },
  stdio: 'inherit',
});

function listWindowsChildren(pid) {
  const result = spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-Command',
      `(Get-CimInstance Win32_Process -Filter "ParentProcessId=${pid}").ProcessId`,
    ],
    { encoding: 'utf8' },
  );
  if (result.status !== 0 || !result.stdout) {
    return [];
  }
  return result.stdout.split(/\s+/).filter(Boolean);
}

function killInstall() {
  if (process.platform === 'win32') {
    // taskkill /T terminates the tree in unspecified order. When the
    // postinstall child dies before vp.exe, vp treats the reinstall as
    // failed and removes the partial install dir, so no stale dir is left
    // for the check-stale step. Kill vp.exe first so it cannot react, then
    // sweep the orphaned installer children (killing vp.exe alone would
    // leave them running in the background). The tree is stable while
    // postinstall sleeps, so enumerating children before the kill is safe.
    const children = listWindowsChildren(child.pid);
    const killed =
      spawnSync('taskkill.exe', ['/PID', String(child.pid), '/F'], {
        stdio: 'ignore',
      }).status === 0;
    for (const pid of children) {
      spawnSync('taskkill.exe', ['/PID', pid, '/T', '/F'], { stdio: 'ignore' });
    }
    return killed;
  }
  return child.kill('SIGKILL');
}

let interrupted = false;

(async () => {
  for (let attempt = 0; attempt < 500; attempt++) {
    if (fs.existsSync(readyPath)) {
      if (!killInstall()) {
        throw new Error('failed to interrupt reinstall');
      }
      interrupted = true;
      return;
    }
    if (child.exitCode !== null || child.signalCode !== null) {
      throw new Error('reinstall exited before reaching postinstall');
    }
    await sleep(20);
  }
  throw new Error('timed out waiting for reinstall postinstall');
})().catch((error) => {
  console.error(error.message);
  killInstall();
  process.exitCode = 1;
});

child.on('close', (code) => {
  fs.rmSync(readyPath, { force: true });
  if (!interrupted) {
    process.exitCode ||= code || 1;
  }
});
