import { isMainThread } from 'node:worker_threads';

// Node also runs command-line preloads in Workers. Load the addon only here.
if (isMainThread && process.execArgv.at(-1)?.endsWith('/dist/vite-dev-terminal.cjs')) {
  // Keep this private preload out of the Node options inherited by child processes.
  process.execArgv.pop();
  const { captureStdinTerminal } =
    require('../binding/index.cjs') as typeof import('../binding/index.cjs');
  const terminal = captureStdinTerminal();
  if (terminal) {
    process.once('exit', () => terminal.restore());
    terminal.restoreOnDefaultHangup();
    // Node owns SIGHUP while a plugin listens for it. When the last listener
    // leaves, Node restores the default disposition before our callback runs.
    process.on('removeListener', (event) => {
      if (event === 'SIGHUP' && process.listenerCount('SIGHUP') === 0) {
        terminal.restoreOnDefaultHangup();
      }
    });
  }
}
