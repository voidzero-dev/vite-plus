import { afterEach, beforeEach, expect, test, vi } from '@voidzero-dev/vite-plus-test';

import {
  configDefaults,
  coverageConfigDefaults,
  defaultExclude,
  defaultInclude,
  defaultBrowserPort,
  defineConfig,
  defineProject,
  lazyPlugins,
} from '../index.js';

let originalVpCommand: string | undefined;

beforeEach(() => {
  originalVpCommand = process.env.VP_COMMAND;
});

afterEach(() => {
  if (originalVpCommand === undefined) {
    delete process.env.VP_COMMAND;
  } else {
    process.env.VP_COMMAND = originalVpCommand;
  }
});

test('should keep vitest exports stable', () => {
  expect(defineConfig).toBeTypeOf('function');
  expect(defineProject).toBeTypeOf('function');
  expect(lazyPlugins).toBeTypeOf('function');
  expect(configDefaults).toBeDefined();
  expect(coverageConfigDefaults).toBeDefined();
  expect(defaultExclude).toBeDefined();
  expect(defaultInclude).toBeDefined();
  expect(defaultBrowserPort).toBeDefined();
});

// lazyPlugins tests

test('lazyPlugins executes callback when VP_COMMAND is unset', () => {
  delete process.env.VP_COMMAND;
  const result = lazyPlugins(() => [{ name: 'test' }]);
  expect(result).toEqual([{ name: 'test' }]);
});

test.each(['dev', 'build', 'test', 'preview'])(
  'lazyPlugins executes callback when VP_COMMAND is %s',
  (cmd) => {
    process.env.VP_COMMAND = cmd;
    const result = lazyPlugins(() => [{ name: 'my-plugin' }]);
    expect(result).toEqual([{ name: 'my-plugin' }]);
  },
);

test.each(['lint', 'fmt', 'check', 'pack', 'install', 'run'])(
  'lazyPlugins returns undefined when VP_COMMAND is %s',
  (cmd) => {
    process.env.VP_COMMAND = cmd;
    const cb = vi.fn(() => [{ name: 'my-plugin' }]);
    const result = lazyPlugins(cb);
    expect(result).toBeUndefined();
    expect(cb).not.toHaveBeenCalled();
  },
);

test('lazyPlugins supports async callback', async () => {
  process.env.VP_COMMAND = 'build';
  const result = lazyPlugins(async () => {
    const plugin = await Promise.resolve({ name: 'async-plugin' });
    return [plugin];
  });
  // Async factory wraps the promise in an array for Vite's asyncFlatten
  expect(Array.isArray(result)).toBe(true);
});

test('lazyPlugins returns undefined for async callback when skipped', () => {
  process.env.VP_COMMAND = 'lint';
  const result = lazyPlugins(async () => {
    return [{ name: 'async-plugin' }];
  });
  expect(result).toBeUndefined();
});

test('lazyPlugins wraps sync function returning a Promise into array', () => {
  process.env.VP_COMMAND = 'build';
  // A sync function that returns a Promise (not an async function) — same handling as async
  const result = lazyPlugins(() => Promise.resolve([{ name: 'sync-promise-plugin' }]));
  expect(Array.isArray(result)).toBe(true);
  expect(result).not.toBeInstanceOf(Promise);
});

// lazyPlugins type compatibility tests — these verify at compile time that
// lazyPlugins return types satisfy Vite's plugins?: PluginOption[] field.

test('lazyPlugins sync return type satisfies plugins field', () => {
  process.env.VP_COMMAND = 'build';
  // Must compile: plugins accepts PluginOption[] | undefined
  const config = defineConfig({
    plugins: lazyPlugins(() => [{ name: 'sync-type-test' }]),
  });
  expect(config.plugins?.length).toBe(1);
});

test('lazyPlugins async return type satisfies plugins field', () => {
  process.env.VP_COMMAND = 'build';
  // Must compile: async overload returns PluginOption[] | undefined, not Promise
  const config = defineConfig({
    plugins: lazyPlugins(async () => {
      return [{ name: 'async-type-test' }];
    }),
  });
  expect(Array.isArray(config.plugins)).toBe(true);
});

test('lazyPlugins undefined return satisfies plugins field', () => {
  process.env.VP_COMMAND = 'lint';
  // Must compile: undefined is accepted by plugins?: PluginOption[]
  const config = defineConfig({
    plugins: lazyPlugins(() => [{ name: 'skipped' }]),
  });
  expect(config.plugins).toBeUndefined();
});

test('lazyPlugins with vitest configureVitest plugin satisfies plugins field', () => {
  process.env.VP_COMMAND = 'test';
  const config = defineConfig({
    plugins: lazyPlugins(() => [
      {
        name: 'vitest-plugin',
        configureVitest() {},
      },
    ]),
  });
  expect(config.plugins?.length).toBe(1);
});

// defineConfig compatibility tests

test('defineConfig passes through plain plugins array', () => {
  const config = defineConfig({
    plugins: [{ name: 'test-plugin' }],
  });
  expect(config.plugins?.length).toBe(1);
});

test('defineConfig supports Plugin objects in plugins array', () => {
  const config = defineConfig({
    plugins: [{ name: 'plugin-a' }, { name: 'plugin-b' }],
  });
  expect(config.plugins?.length).toBe(2);
});

test('defineConfig supports falsy values in plugins array', () => {
  const config = defineConfig({
    plugins: [{ name: 'real-plugin' }, false, null, undefined],
  });
  expect(config.plugins?.length).toBe(4);
});

test('defineConfig supports nested plugin arrays', () => {
  const config = defineConfig({
    plugins: [[{ name: 'nested-a' }, { name: 'nested-b' }], { name: 'top-level' }],
  });
  expect(config.plugins?.length).toBe(2);
});

test('defineConfig supports Promise<Plugin> in plugins array', () => {
  const config = defineConfig({
    plugins: [Promise.resolve({ name: 'async-plugin' })],
  });
  expect(config.plugins?.length).toBe(1);
});

test('defineConfig supports mixed PluginOption types in array', () => {
  const config = defineConfig({
    plugins: [
      { name: 'sync-plugin' },
      false,
      Promise.resolve({ name: 'promised-plugin' }),
      [{ name: 'nested-plugin' }],
      null,
      undefined,
    ],
  });
  expect(config.plugins?.length).toBe(6);
});

test('defineConfig supports empty plugins array', () => {
  const config = defineConfig({
    plugins: [],
  });
  expect(config.plugins?.length).toBe(0);
});

test('defineConfig supports config without plugins', () => {
  const config = defineConfig({});
  expect(config.plugins).toBeUndefined();
});

test('defineConfig supports function config with plain plugins array', () => {
  const configFn = defineConfig(() => ({
    plugins: [{ name: 'fn-plugin' }],
  }));
  const config = configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(1);
});

test('defineConfig supports async function config with plain plugins array', async () => {
  const configFn = defineConfig(async () => ({
    plugins: [{ name: 'async-fn-plugin' }],
  }));
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(config.plugins?.length).toBe(1);
});

test('defineConfig supports vitest plugin with configureVitest hook', () => {
  const config = defineConfig({
    plugins: [
      {
        name: 'vitest-plugin',
        configureVitest() {
          // vitest plugin hook
        },
      },
    ],
  });
  expect(config.plugins?.length).toBe(1);
  expect((config.plugins?.[0] as { name: string })?.name).toBe('vitest-plugin');
});
