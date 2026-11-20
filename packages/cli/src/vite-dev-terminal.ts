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
    terminal.restoreOnDefaultSignals();
    // After Node removes its last signal listener, a plugin can re-send the
    // signal and terminate before the exit event runs.
    process.on('removeListener', (event) => {
      if ((event === 'SIGHUP' || event === 'SIGINT') && process.listenerCount(event) === 0) {
        terminal.restoreOnDefaultSignals();
      }
    });
  }
}
