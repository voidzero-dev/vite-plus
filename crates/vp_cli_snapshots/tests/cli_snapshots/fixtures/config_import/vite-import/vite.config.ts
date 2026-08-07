import { defineConfig } from 'vite';

// The computed command defeats static config extraction on purpose, forcing
// the JS loader to run and the bare `vite` import to actually resolve
// (aliased to the core package by the runner, like migrated projects).
export default defineConfig({
  run: {
    tasks: {
      hello: {
        command: ['vpt', 'print', 'vite-config-loaded'].join(' '),
      },
    },
  },
});
