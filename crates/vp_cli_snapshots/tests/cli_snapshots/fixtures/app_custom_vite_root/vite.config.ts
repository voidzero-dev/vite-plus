import { defineConfig } from 'vite-plus';

function emitMilestone(name: string) {
  // Let Vite print its startup banner after server.listen() resolves.
  setImmediate(() => {
    const encodedName = Buffer.from(name).toString('base64url');
    process.stdout.write(
      `\x1b]2;pty-terminal-test:${'0'.repeat(32)}:${encodedName}\x1b\\`,
    );
  });
}

export default defineConfig({
  clearScreen: false,
  root: 'src',
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
