const { spawn } = require('child_process');
const { constants } = require('os');

const child = spawn('vp', ['install', '-g', './long-time-install-package'], {
  stdio: 'inherit',
});

setTimeout(() => {
  if (!child.killed) {
    child.kill('SIGKILL');
  }
}, 100);

child.on('close', (code, signal) => {
  const signalNumber = signal && constants.signals[signal];
  process.exit(code ?? (signalNumber ? 128 + signalNumber : 1));
});
