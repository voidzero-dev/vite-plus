import { writeFileSync, writeSync } from 'node:fs';
import { setTimeout } from 'node:timers/promises';

if (process.env.VP_PROBE_SYNC_CONFIG === '1') {
  writeSync(1, 'Evaluating Vite config\n');
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 6_000);
}

export default {
  clearScreen: false,
  optimizeDeps: { noDiscovery: true },
  plugins: [
    {
      name: 'report-listening-port',
      configureServer(server) {
        if (process.env.VP_PROBE_RERAISE_SIGINT === '1') {
          process.once('SIGINT', () => {
            writeSync(2, 'Vite plugin re-sent SIGINT\n');
            process.kill(process.pid, 'SIGINT');
          });
        }
        server.httpServer.once('listening', () => {
          if (process.env.VP_PROBE_RAW === '1') {
            process.stdin.setRawMode(true);
          }
          console.log(`listening ${server.httpServer.address().port} ${process.pid}`);
        });
      },
      async closeBundle() {
        const marker = process.env.VP_PROBE_SHUTDOWN_MARKER;
        if (marker) {
          writeFileSync(marker, '');
          await setTimeout(6_000);
        }
      },
    },
  ],
};
