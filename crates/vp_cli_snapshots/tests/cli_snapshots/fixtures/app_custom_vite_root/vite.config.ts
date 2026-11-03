import { defineConfig } from 'vite-plus';

function emitMilestone(name: string): void {
  // Wait one event-loop turn so Vite can print the startup banner.
  setImmediate(() => {
    const encodedName = Buffer.from(name).toString('base64url');
    process.stdout.write(
      `\x1b]2;vt-pty-terminal-test:${'0'.repeat(32)}:${encodedName}\x1b\\`,
    );
  });
}

export default defineConfig({
  clearScreen: false,
  root: 'src',
  // This routing fixture has no dependencies. Avoid a background scan racing shutdown.
  optimizeDeps: { noDiscovery: true },
  server: {
    host: '127.0.0.1',
    port: 0,
  },
  preview: {
    host: '127.0.0.1',
    port: 0,
  },
  plugins: [
    {
      name: 'server-ready-milestones',
      configureServer(server) {
        server.httpServer?.once('listening', () => {
          emitMilestone('dev-server:ready');
        });
      },
      configurePreviewServer(server) {
        server.httpServer.once('listening', () => {
          emitMilestone('preview-server:ready');
        });
      },
    },
  ],
});
