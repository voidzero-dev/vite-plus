import { defineConfig } from 'vite-plus';

export default defineConfig({
  create: {
    templates: [
      {
        name: 'starter',
        description: 'A local starter template that wires fmt/lint via shorthand.',
        template: './packages/starter-template',
      },
    ],
  },
});
