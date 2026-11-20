import { writeFileSync } from 'node:fs';

import { defineConfig } from 'vite-plus';

writeFileSync('vite-argv.json', JSON.stringify(process.argv.slice(2)));

export default defineConfig({
  clearScreen: false,
  optimizeDeps: { noDiscovery: true },
  server: { host: '127.0.0.1', port: 0 },
  plugins: [
    {
      name: 'server-ready-milestone',
      configureServer(server) {
        server.httpServer?.once('listening', () => {
          // Wait until Vite prints its startup banner.
          setImmediate(() => {
            const milestone = Buffer.from('dev-server:ready').toString('base64url');
            process.stdout.write(
              `\x1b]2;vt-pty-terminal-test:${'0'.repeat(32)}:${milestone}\x1b\\`,
            );
          });
        });
      },
    },
  ],
});
