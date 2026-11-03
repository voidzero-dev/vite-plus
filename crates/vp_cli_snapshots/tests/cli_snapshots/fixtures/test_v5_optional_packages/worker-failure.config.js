import { defineConfig } from 'vite-plus';
export default defineConfig({ test: {
  pool: 'forks',
  execArgv: ['--vitest-invalid-worker-option'],
  include: ['custom.test.js'],
} });
