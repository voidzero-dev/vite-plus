import { defineConfig } from './define-config';

export default defineConfig({
  run: {
    tasks: {
      selected: {
        command: 'node static.js',
        dependsOn: [],
      },
    },
  },
});
