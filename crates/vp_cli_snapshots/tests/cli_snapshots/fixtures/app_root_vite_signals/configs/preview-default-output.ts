import { emitMilestone } from './milestone';

export default {
  clearScreen: false,
  plugins: [
    {
      name: 'preview-ready-milestone',
      configurePreviewServer() {
        return () => emitMilestone('preview-server:ready');
      },
    },
  ],
};
