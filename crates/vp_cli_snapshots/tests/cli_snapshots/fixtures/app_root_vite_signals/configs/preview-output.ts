import { emitMilestone } from './milestone';

const build = {
  outDir: 'output',
};

export default {
  clearScreen: false,
  build,
  preview: {
    host: '127.0.0.1',
    port: 0,
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
