export default {
  clearScreen: false,
  optimizeDeps: { noDiscovery: true },
  plugins: [
    {
      name: 'report-listening-port',
      configureServer(server) {
        server.httpServer.once('listening', () => {
          const { port } = server.httpServer.address();
          console.log(`listening ${port} ${process.pid}`);
          // A caller that forks the launcher keeps its IPC channel.
          process.send?.({ pid: process.pid });
        });
      },
    },
  ],
};
