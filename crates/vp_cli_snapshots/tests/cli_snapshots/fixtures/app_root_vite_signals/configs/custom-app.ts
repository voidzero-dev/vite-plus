import { emitMilestone } from './milestone';

const appType = 'custom';

export default {
  appType,
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 0,
  },
  plugins: [
    {
      name: 'dev-ready-milestone',
      configureServer(server) {
        server.httpServer?.once('listening', () => {
          emitMilestone('dev-server:ready');
        });
      },
    },
  ],
};
