import { defineProject } from 'vite-plus';
export default defineProject({
  test: {
    name: 'nested-parent',
    provide: { value: 'nested' },
    projects: [{ test: { name: 'nested-child', include: ['identity.test.js'] } }],
  },
});
