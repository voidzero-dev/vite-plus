export default {
  server: { host: '127.0.0.1', port: 0 },
  plugins: [
    {
      name: 'server-ready-milestone',
      configureServer(server) {
        server.httpServer.once('listening', () => {
          setImmediate(() => {
            const name = Buffer.from('dev-server:ready').toString('base64url');
            process.stdout.write(`\x1b]2;vt-pty-terminal-test:${'0'.repeat(32)}:${name}\x1b\\`);
          });
        });
      },
    },
  ],
};
