import { constants } from 'node:os';
import { isMainThread } from 'node:worker_threads';

// Node also runs command-line preloads in Workers. Load the addon only here.
if (isMainThread && process.execArgv.at(-1)?.endsWith('/dist/vite-dev-terminal.cjs')) {
  // Keep this private preload out of the Node options inherited by child processes.
  process.execArgv.pop();
  const { captureStdinTerminal } =
    require('../binding/index.cjs') as typeof import('../binding/index.cjs');
  const terminal = captureStdinTerminal();
  if (terminal) {
    const restoreTerminal = () => terminal.restore();
    process.once('exit', restoreTerminal);

    function restoreOnHangup() {
      if (process.listenerCount('SIGHUP') === 0) {
        restoreTerminal();
        process.kill(process.pid, 'SIGHUP');
        return;
      }

      // The once listener has already removed itself. Other handlers can keep
      // running, or clean up and re-send SIGHUP after removing their listeners.
      // oxlint-disable-next-line typescript/unbound-method -- Keep the original identity and call with process as the receiver.
      const kill = process.kill;
      const killAfterRestore: typeof process.kill = (pid, signal) => {
        if (
          pid === process.pid &&
          (signal === 'SIGHUP' || signal === constants.signals.SIGHUP) &&
          process.listenerCount('SIGHUP') === 0
        ) {
          restoreTerminal();
        }
        return kill.call(process, pid, signal);
      };
      process.kill = killAfterRestore;
      queueMicrotask(() => {
        if (process.kill === killAfterRestore) {
          process.kill = kill;
        }
        process.prependOnceListener('SIGHUP', restoreOnHangup);
      });
    }

    // Keep SIGINT and SIGTERM on Node's native path while Vite loads config.
    process.prependOnceListener('SIGHUP', restoreOnHangup);
  }
}
