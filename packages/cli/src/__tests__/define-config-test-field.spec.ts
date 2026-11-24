import { expect, describe, it } from 'vitest';

import { defineConfig } from '../index.js';

describe('defineConfig test field typing', () => {
  it('accepts test config without TS error', () => {
    const cfg = defineConfig({
      test: {
        globals: true,
        environment: 'node',
        include: ['src/**/*.spec.ts'],
      },
    });
    expect(cfg.test).toMatchObject({ globals: true, environment: 'node' });
  });

  it('accepts test config alongside other vite-plus fields', () => {
    const cfg = defineConfig({
      test: {
        globals: true,
        environment: 'jsdom',
      },
      lint: {},
    });
    expect(cfg.test).toMatchObject({ globals: true, environment: 'jsdom' });
  });
});
