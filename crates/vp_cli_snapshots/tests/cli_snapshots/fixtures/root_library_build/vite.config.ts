import { emitMilestone } from './milestone';

export default {
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 0,
  },
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
        // The listening event precedes CLI shortcut registration. Wait until
        // the input handler is attached before the runner sends `q`.
        const bindCLIShortcuts = server.bindCLIShortcuts.bind(server);
        server.bindCLIShortcuts = (options) => {
          bindCLIShortcuts(options);
          emitMilestone('dev-server:ready');
        };
      },
    },
  ],
};
