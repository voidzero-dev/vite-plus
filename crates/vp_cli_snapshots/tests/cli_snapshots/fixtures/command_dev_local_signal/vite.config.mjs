export default {
  clearScreen: false,
  optimizeDeps: { noDiscovery: true },
  plugins: [
    {
      name: 'report-listening-port',
      configureServer(server) {
        server.httpServer.once('listening', () => {
          if (process.env.VP_PROBE_RAW === '1') {
            process.stdin.setRawMode(true);
          }
          console.log(`listening ${server.httpServer.address().port} ${process.pid}`);
        });
      },
    },
  ],
};
