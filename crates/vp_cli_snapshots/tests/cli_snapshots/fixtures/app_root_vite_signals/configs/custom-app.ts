import { emitMilestone } from './milestone';

export default {
  appType: 'custom',
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
