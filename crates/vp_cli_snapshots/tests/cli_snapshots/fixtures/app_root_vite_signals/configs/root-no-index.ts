import { emitMilestone } from './milestone';

const appRoot = 'src';

export default {
  clearScreen: false,
  root: appRoot,
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
