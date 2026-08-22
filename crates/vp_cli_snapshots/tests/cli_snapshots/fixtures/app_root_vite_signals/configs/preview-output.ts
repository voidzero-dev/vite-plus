import { emitMilestone } from './milestone';

const build = {
  outDir: 'output',
};

export default {
  clearScreen: false,
  build,
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
