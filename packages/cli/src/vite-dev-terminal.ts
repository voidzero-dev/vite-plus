import { isMainThread } from 'node:worker_threads';

import { captureStdinTerminal } from '../binding/index.cjs';

if (isMainThread && process.execArgv.at(-1)?.endsWith('/dist/vite-dev-terminal.cjs')) {
  // Keep this private preload out of the Node options inherited by child processes.
  process.execArgv.pop();
  const terminal = captureStdinTerminal();
  if (terminal) {
    const restoreTerminal = () => terminal.restore();
    process.once('exit', restoreTerminal);
    // Node resets its terminal mode on SIGINT and SIGTERM. Restore it on
    // SIGHUP too, while letting a plugin with its own handler handle the signal.
    const restoreAndReraise = () => {
      if (process.listenerCount('SIGHUP') === 1) {
        restoreTerminal();
        process.removeListener('SIGHUP', restoreAndReraise);
        process.kill(process.pid, 'SIGHUP');
      }
    };
    process.prependListener('SIGHUP', restoreAndReraise);
  }
}
