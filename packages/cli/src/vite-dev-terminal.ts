import { isMainThread } from 'node:worker_threads';

// Node also runs command-line preloads in Workers. Load the addon only here.
if (isMainThread && process.execArgv.at(-1)?.endsWith('/dist/vite-dev-terminal.cjs')) {
  // Keep this private preload out of the Node options inherited by child processes.
  process.execArgv.pop();
  const { captureStdinTerminal } =
    require('../binding/index.cjs') as typeof import('../binding/index.cjs');
  const terminal = captureStdinTerminal();
  if (terminal) {
    // Share exit observation with plugins that use signal-exit. They can clean
    // up before the terminal is restored, and direct signal handlers still run.
    const { onExit } = require('signal-exit') as typeof import('signal-exit');
    onExit(() => terminal.restore(), { alwaysLast: true });
  }
}
