export default {
  clearScreen: false,
  plugins: [
    {
      name: 'dev-server-ready-milestone',
      configureServer(server) {
        server.httpServer?.once('listening', () => {
          // Let Vite print its startup banner after server.listen() resolves.
          setImmediate(() => {
            const name = Buffer.from('dev-server:ready').toString('base64url');
            process.stdout.write(`\x1b]2;pty-terminal-test:${'0'.repeat(32)}:${name}\x1b\\`);
          });
        });
      },
    },
  ],
  run: {
    cache: true,
  },
};
