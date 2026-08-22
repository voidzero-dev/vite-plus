import { emitMilestone } from './milestone';

const appType = 'custom';

export default {
  appType,
  clearScreen: false,
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
