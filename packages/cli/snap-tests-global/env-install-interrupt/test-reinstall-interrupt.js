const { spawn } = require('child_process');

const child = spawn('vp', ['install', '-g', './long-time-install-package'], {
  stdio: 'inherit',
});

setTimeout(() => {
  if (!child.killed) {
    child.kill('SIGKILL');
  }
}, 100);

child.on('close', (code) => {
  process.exit(code);
});
