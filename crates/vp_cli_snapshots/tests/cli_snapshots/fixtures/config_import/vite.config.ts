import { defineConfig } from 'vite-plus';

// The computed command defeats static config extraction on purpose, forcing
// the JS loader to run and the bare `vite-plus` import to actually resolve.
export default defineConfig({
  run: {
    tasks: {
      hello: {
        command: ['vpt', 'print', 'config-loaded'].join(' '),
      },
    },
  },
});
