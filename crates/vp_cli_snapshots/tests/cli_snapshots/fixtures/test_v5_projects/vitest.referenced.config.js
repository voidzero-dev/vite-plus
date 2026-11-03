import { defineProject } from 'vite-plus';
export default defineProject({
  test: {
    name: 'referenced',
    include: ['identity.test.js'],
    provide: { value: 'referenced' },
  },
});
