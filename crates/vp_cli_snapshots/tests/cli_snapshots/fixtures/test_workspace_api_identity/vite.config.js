import { defineConfig } from 'vite-plus';

const mode = process.env.IDENTITY_MODE ?? 'single';
const include = ['packages/child/identity.test.js'];
const configurations = {
  single: { include },
  shared: { projects: [{ test: { name: 'child', include } }] },
  separate: { sharedViteServer: false, projects: ['packages/child'] },
};
export default defineConfig({
  test: configurations[mode],
});
