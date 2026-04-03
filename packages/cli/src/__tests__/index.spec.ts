import { afterEach, beforeEach, expect, test } from '@voidzero-dev/vite-plus-test';

import {
  configDefaults,
  coverageConfigDefaults,
  defaultExclude,
  defaultInclude,
  defaultBrowserPort,
  defineConfig,
  defineProject,
  vitePlugins,
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
  expect(vitePlugins).toBeTypeOf('function');
  expect(configDefaults).toBeDefined();
  expect(coverageConfigDefaults).toBeDefined();
  expect(defaultExclude).toBeDefined();
  expect(defaultInclude).toBeDefined();
  expect(defaultBrowserPort).toBeDefined();
});

test('vitePlugins returns undefined when VP_COMMAND is unset', () => {
  delete process.env.VP_COMMAND;
  const result = vitePlugins(() => [{ name: 'test' }]);
  expect(result).toBeUndefined();
});

test('vitePlugins returns undefined when VP_COMMAND is empty string', () => {
  process.env.VP_COMMAND = '';
  const result = vitePlugins(() => [{ name: 'test' }]);
  expect(result).toBeUndefined();
});

test.each(['dev', 'build', 'test', 'preview'])(
  'vitePlugins executes callback when VP_COMMAND is %s',
  (cmd) => {
    process.env.VP_COMMAND = cmd;
    const result = vitePlugins(() => [{ name: 'my-plugin' }]);
    expect(result).toEqual([{ name: 'my-plugin' }]);
  },
);

test.each(['lint', 'fmt', 'check', 'pack', 'install', 'run'])(
  'vitePlugins returns undefined when VP_COMMAND is %s',
  (cmd) => {
    process.env.VP_COMMAND = cmd;
    const result = vitePlugins(() => [{ name: 'my-plugin' }]);
    expect(result).toBeUndefined();
  },
);

test('vitePlugins supports async callback', async () => {
  process.env.VP_COMMAND = 'build';
  const result = vitePlugins(async () => {
    const plugin = await Promise.resolve({ name: 'async-plugin' });
    return [plugin];
  });
  expect(result).toBeInstanceOf(Promise);
  expect(await result).toEqual([{ name: 'async-plugin' }]);
});

test('vitePlugins returns undefined for async callback when skipped', () => {
  process.env.VP_COMMAND = 'lint';
  const result = vitePlugins(async () => {
    return [{ name: 'async-plugin' }];
  });
  expect(result).toBeUndefined();
});
