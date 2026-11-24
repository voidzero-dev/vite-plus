import { emitMilestone } from './milestone.ts';

const appRoot = 'src';

export default {
  clearScreen: false,
  root: appRoot,
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
