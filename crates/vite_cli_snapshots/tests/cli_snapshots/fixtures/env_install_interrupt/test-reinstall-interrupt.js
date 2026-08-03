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

function killInstall() {
  if (process.platform === 'win32') {
    // Killing vp.exe alone leaves its installer child running in the background.
    return (
      spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {
        stdio: 'ignore',
      }).status === 0
    );
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
