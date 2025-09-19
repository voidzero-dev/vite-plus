import { defineConfig } from 'vite';

export default defineConfig({
  test: {
    projects: ['apps/*', 'packages/*', 'tools/*'],
  },
});
