import { defineConfig } from 'vite-plus';

export default defineConfig({
  create: {
    templates: [
      {
        name: 'recorder',
        description: 'Record each template argument.',
        template: './packages/args-template',
      },
    ],
  },
});
