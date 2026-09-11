import { describe, expect, it } from 'vitest';

import {
  configDefaults,
  coverageConfigDefaults,
  defineConfig,
  loadConfigFromFile,
  mergeConfig,
} from '../index.js';

describe('defineConfig test field typing', () => {
  it('accepts test config without TS error', () => {
    const cfg = defineConfig({
      test: {
        globals: true,
        environment: 'node',
        include: ['src/**/*.spec.ts'],
      },
    });
    void cfg;
  });

  it('accepts test config alongside other vite-plus fields', () => {
    const cfg = defineConfig({
      test: {
        globals: true,
        environment: 'jsdom',
      },
      lint: {},
    });
    void cfg;
  });

  it('re-exports shared and vitest-specific names without star-export conflicts', () => {
    expect(typeof defineConfig).toBe('function');
    expect(typeof mergeConfig).toBe('function');
    expect(typeof loadConfigFromFile).toBe('function');
    expect(typeof configDefaults).toBe('object');
    expect(typeof coverageConfigDefaults).toBe('object');

    const merged = mergeConfig({ test: { globals: true } }, { test: { environment: 'node' } });
    expect(merged.test.globals).toBe(true);
    expect(merged.test.environment).toBe('node');
  });
});
