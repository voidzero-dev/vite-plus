const { spawn, spawnSync } = require('child_process');

const child = spawn('vp', ['install', '-g', './long-time-install-package'], {
  stdio: 'inherit',
});

setTimeout(() => {
  if (child.exitCode === null && child.signalCode === null) {
    if (process.platform === 'win32') {
      // Killing vp.exe alone leaves its installer child running in the background.
      spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {
        stdio: 'ignore',
      });
      return;
    }
    child.kill('SIGKILL');
  }
}, 100);

child.on('close', (code) => {
  process.exit(code);
});
