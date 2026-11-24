import { describe, it } from 'vitest';

import { defineConfig } from '../index.js';

describe('defineConfig test field typing', () => {
  it('accepts test config without TS error', () => {
    defineConfig({
      test: {
        globals: true,
        environment: 'node',
        include: ['src/**/*.spec.ts'],
      },
    });
  });

  it('accepts test config alongside other vite-plus fields', () => {
    defineConfig({
      test: {
        globals: true,
        environment: 'jsdom',
      },
      lint: {},
    });
  });
});
