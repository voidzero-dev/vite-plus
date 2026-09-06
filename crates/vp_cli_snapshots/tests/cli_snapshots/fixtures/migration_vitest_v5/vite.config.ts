import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    reporters: ['json', ['junit', {}]],
    projects: [{ test: { name: 'unit' } }, { extends: true, test: { name: 'inherited' } }],
  },
});
