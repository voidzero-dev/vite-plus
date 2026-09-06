import { defineConfig } from 'vite-plus';

export default defineConfig({ test: {
  projects: [
    { test: { name: 'jsdom', environment: 'jsdom', include: ['jsdom.test.js'] } },
    { test: { name: 'happy-dom', environment: 'happy-dom', include: ['happy-dom.test.js'] } },
    { test: { name: 'temporal', setupFiles: ['temporal-polyfill/global'], include: ['temporal.test.js'] } },
    { test: { name: 'custom', environment: './custom-environment.js', include: ['custom.test.js'] } },
  ],
} });
