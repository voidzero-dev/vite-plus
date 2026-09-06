import { defineConfig } from 'vite-plus';

export default defineConfig({ test: {
  include: ['coverage.test.js'],
  coverage: {
    enabled: true,
    provider: process.env.COVERAGE_PROVIDER,
    reporter: [],
    include: ['src/**'],
    exclude: ['src/ignored.js'],
    thresholds: { 'src/**': { perFile: process.env.PER_FILE === 'true', lines: 50 } },
  },
} });
