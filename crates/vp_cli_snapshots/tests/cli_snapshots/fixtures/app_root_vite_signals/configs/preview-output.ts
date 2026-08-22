import { emitMilestone } from './milestone';

export default {
  clearScreen: false,
  build: {
    outDir: 'output',
  },
  plugins: [
    {
      name: 'preview-ready-milestone',
      configurePreviewServer(server) {
        server.httpServer.once('listening', () => {
          emitMilestone('preview-server:ready');
        });
      },
    },
  ],
};
