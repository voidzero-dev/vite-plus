export default {
  clearScreen: false,
  optimizeDeps: { noDiscovery: true },
  plugins: [
    {
      name: 'report-listening-port',
      configureServer(server) {
        server.httpServer.once('listening', () => {
          console.log(`listening ${server.httpServer.address().port} ${process.pid}`);
        });
      },
    },
  ],
};
