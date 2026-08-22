import { emitMilestone } from './milestone';

export default {
  clearScreen: false,
  build: {
    lib: {
      entry: './src/index.ts',
      formats: ['es'],
      fileName: 'index',
    },
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
