import { expect, test } from '@voidzero-dev/vite-plus-test';

import {
  configDefaults,
  coverageConfigDefaults,
  defaultExclude,
  defaultInclude,
  defaultBrowserPort,
  defineConfig,
  defineProject,
} from '../index.js';

test('should keep vitest exports stable', () => {
  expect(defineConfig).toBeTypeOf('function');
  expect(defineProject).toBeTypeOf('function');
  expect(configDefaults).toBeDefined();
  expect(coverageConfigDefaults).toBeDefined();
  expect(defaultExclude).toBeDefined();
  expect(defaultInclude).toBeDefined();
  expect(defaultBrowserPort).toBeDefined();
});

test('should support lazy loading of plugins', async () => {
  const config = await defineConfig({
    lazy: () => Promise.resolve({ plugins: [{ name: 'test' }] }),
  });
  expect(config.plugins?.length).toBe(1);
});

test('should merge lazy plugins with existing plugins', async () => {
  const config = await defineConfig({
    plugins: [{ name: 'existing' }],
    lazy: () => Promise.resolve({ plugins: [{ name: 'lazy' }] }),
  });
  expect(config.plugins?.length).toBe(2);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('existing');
  expect((config.plugins?.[1] as { name: string })?.name).toBe('lazy');
});

test('should handle lazy with empty plugins array', async () => {
  const config = await defineConfig({
    lazy: () => Promise.resolve({ plugins: [] }),
  });
  expect(config.plugins?.length).toBe(0);
});

test('should handle lazy returning undefined plugins', async () => {
  const config = await defineConfig({
    lazy: () => Promise.resolve({}),
  });
  expect(config.plugins?.length).toBe(0);
});

test('should handle Promise config with lazy', async () => {
  const config = await defineConfig(
    Promise.resolve({
      lazy: () => Promise.resolve({ plugins: [{ name: 'lazy-from-promise' }] }),
    }),
  );
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('lazy-from-promise');
});

test('should handle Promise config with lazy and existing plugins', async () => {
  const config = await defineConfig(
    Promise.resolve({
      plugins: [{ name: 'existing' }],
      lazy: () => Promise.resolve({ plugins: [{ name: 'lazy' }] }),
    }),
  );
  expect(config.plugins?.length).toBe(2);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('existing');
  expect((config.plugins?.[1] as { name: string })?.name).toBe('lazy');
});

test('should handle Promise config without lazy', async () => {
  const config = await defineConfig(
    Promise.resolve({
      plugins: [{ name: 'no-lazy' }],
    }),
  );
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('no-lazy');
});

test('should handle function config with lazy', async () => {
  const configFn = defineConfig(() => ({
    lazy: () => Promise.resolve({ plugins: [{ name: 'lazy-from-fn' }] }),
  }));
  expect(typeof configFn).toBe('function');
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('lazy-from-fn');
});

test('should handle function config with lazy and existing plugins', async () => {
  const configFn = defineConfig(() => ({
    plugins: [{ name: 'existing' }],
    lazy: () => Promise.resolve({ plugins: [{ name: 'lazy' }] }),
  }));
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(2);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('existing');
  expect((config.plugins?.[1] as { name: string })?.name).toBe('lazy');
});

test('should handle function config without lazy', () => {
  const configFn = defineConfig(() => ({
    plugins: [{ name: 'no-lazy' }],
  }));
  const config = configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('no-lazy');
});

test('should handle async function config with lazy', async () => {
  const configFn = defineConfig(async () => ({
    lazy: () => Promise.resolve({ plugins: [{ name: 'lazy-from-async-fn' }] }),
  }));
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('lazy-from-async-fn');
});

test('should handle async function config with lazy and existing plugins', async () => {
  const configFn = defineConfig(async () => ({
    plugins: [{ name: 'existing' }],
    lazy: () => Promise.resolve({ plugins: [{ name: 'lazy' }] }),
  }));
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(2);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('existing');
  expect((config.plugins?.[1] as { name: string })?.name).toBe('lazy');
});

test('should handle async function config without lazy', async () => {
  const configFn = defineConfig(async () => ({
    plugins: [{ name: 'no-lazy' }],
  }));
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('no-lazy');
});
