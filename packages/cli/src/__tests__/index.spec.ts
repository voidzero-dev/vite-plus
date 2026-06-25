import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { withConfigMetadataResolution } from '../define-config.ts';
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

let originalMetadataEnv: string | undefined;

beforeEach(() => {
  originalMetadataEnv = process.env.VP_RESOLVING_CONFIG_METADATA;
  delete process.env.VP_RESOLVING_CONFIG_METADATA;
});

afterEach(() => {
  if (originalMetadataEnv === undefined) {
    delete process.env.VP_RESOLVING_CONFIG_METADATA;
  } else {
    process.env.VP_RESOLVING_CONFIG_METADATA = originalMetadataEnv;
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

// lazyPlugins tests — plugins load by default, and are skipped only while a
// config-metadata resolution is in progress (withConfigMetadataResolution).
// The decision does not depend on which command is running, so a build spawned
// by a `vp run` verbatim task or a `vp exec` child keeps its plugins.

test('lazyPlugins executes the callback by default', () => {
  const result = lazyPlugins(() => [{ name: 'test' }]);
  expect(result).toEqual([{ name: 'test' }]);
});

test('lazyPlugins returns undefined during a config-metadata resolution', () => {
  process.env.VP_RESOLVING_CONFIG_METADATA = '1';
  const cb = vi.fn(() => [{ name: 'my-plugin' }]);
  const result = lazyPlugins(cb);
  expect(result).toBeUndefined();
  expect(cb).not.toHaveBeenCalled();
});

test('withConfigMetadataResolution skips plugins during the resolution and restores after', async () => {
  const cb = vi.fn(() => [{ name: 'my-plugin' }]);
  let during: ReturnType<typeof lazyPlugins>;
  const returned = await withConfigMetadataResolution(async () => {
    during = lazyPlugins(cb);
    return 'result';
  });
  expect(returned).toBe('result');
  expect(during).toBeUndefined();
  expect(cb).not.toHaveBeenCalled();
  // marker cleared after → plugins load again
  expect(lazyPlugins(() => [{ name: 'after' }])).toEqual([{ name: 'after' }]);
});

test('withConfigMetadataResolution restores a pre-existing marker (nesting)', async () => {
  process.env.VP_RESOLVING_CONFIG_METADATA = '1';
  await withConfigMetadataResolution(async () => {
    expect(process.env.VP_RESOLVING_CONFIG_METADATA).toBe('1');
  });
  expect(process.env.VP_RESOLVING_CONFIG_METADATA).toBe('1');
});

test('withConfigMetadataResolution keeps the marker set across overlapping resolutions', async () => {
  let releaseFirst!: () => void;
  const firstPending = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  // Two metadata resolutions overlap; the second finishes while the first is
  // still awaiting. The marker must stay set until BOTH complete.
  const first = withConfigMetadataResolution(async () => {
    await firstPending;
    return 'first';
  });
  const second = withConfigMetadataResolution(async () => 'second');
  expect(await second).toBe('second');
  // first is still in flight → lazyPlugins must still skip
  expect(lazyPlugins(() => [{ name: 'plugin' }])).toBeUndefined();
  releaseFirst();
  expect(await first).toBe('first');
  // both done → marker cleared, plugins load again
  expect(lazyPlugins(() => [{ name: 'after' }])).toEqual([{ name: 'after' }]);
});

test('lazyPlugins supports async callback', async () => {
  const result = lazyPlugins(async () => {
    const plugin = await Promise.resolve({ name: 'async-plugin' });
    return [plugin];
  });
  // Async factory wraps the promise in an array for Vite's asyncFlatten
  expect(Array.isArray(result)).toBe(true);
});

test('lazyPlugins returns undefined for async callback during metadata resolution', () => {
  process.env.VP_RESOLVING_CONFIG_METADATA = '1';
  const result = lazyPlugins(async () => {
    return [{ name: 'async-plugin' }];
  });
  expect(result).toBeUndefined();
});

test('lazyPlugins wraps sync function returning a Promise into array', () => {
  // A sync function that returns a Promise (not an async function) — same handling as async
  const result = lazyPlugins(() => Promise.resolve([{ name: 'sync-promise-plugin' }]));
  expect(Array.isArray(result)).toBe(true);
  expect(result).not.toBeInstanceOf(Promise);
});

// defineConfig auto-injects three internal plugins before user-supplied
// plugins: vite-plus:vitest-resolver, vite-plus:auto-inline-matcher-deps, and
// vite-plus:coverage-version-guard. The helper below strips those prefix
// entries so tests can assert on user-supplied plugins only.
const RESOLVER_PLUGIN_NAME = 'vite-plus:vitest-resolver';
const AUTO_INLINE_PLUGIN_NAME = 'vite-plus:auto-inline-matcher-deps';
const COVERAGE_GUARD_PLUGIN_NAME = 'vite-plus:coverage-version-guard';
const userPlugins = (plugins: unknown): unknown[] => {
  expect(Array.isArray(plugins)).toBe(true);
  const arr = plugins as unknown[];
  expect((arr[0] as { name?: string })?.name).toBe(RESOLVER_PLUGIN_NAME);
  expect((arr[1] as { name?: string })?.name).toBe(AUTO_INLINE_PLUGIN_NAME);
  expect((arr[2] as { name?: string })?.name).toBe(COVERAGE_GUARD_PLUGIN_NAME);
  return arr.slice(3);
};

// lazyPlugins type compatibility tests — these verify at compile time that
// lazyPlugins return types satisfy Vite's plugins?: PluginOption[] field.

test('lazyPlugins sync return type satisfies plugins field', () => {
  // Must compile: plugins accepts PluginOption[] | undefined
  const config = defineConfig({
    plugins: lazyPlugins(() => [{ name: 'sync-type-test' }]),
  });
  expect(userPlugins(config.plugins).length).toBe(1);
});

test('lazyPlugins async return type satisfies plugins field', () => {
  // Must compile: async overload returns PluginOption[] | undefined, not Promise
  const config = defineConfig({
    plugins: lazyPlugins(async () => {
      return [{ name: 'async-type-test' }];
    }),
  });
  expect(Array.isArray(config.plugins)).toBe(true);
});

test('lazyPlugins undefined return satisfies plugins field', () => {
  process.env.VP_RESOLVING_CONFIG_METADATA = '1';
  // Must compile: undefined is accepted by plugins?: PluginOption[]
  const config = defineConfig({
    plugins: lazyPlugins(() => [{ name: 'skipped' }]),
  });
  // lazyPlugins returns undefined, but defineConfig still injects its rewrite plugin.
  expect(userPlugins(config.plugins).length).toBe(0);
});

test('lazyPlugins with vitest configureVitest plugin satisfies plugins field', () => {
  const config = defineConfig({
    plugins: lazyPlugins(() => [
      {
        name: 'vitest-plugin',
        configureVitest() {},
      },
    ]),
  });
  expect(userPlugins(config.plugins).length).toBe(1);
});

// defineConfig compatibility tests

test('defineConfig passes through plain plugins array', () => {
  const config = defineConfig({
    plugins: [{ name: 'test-plugin' }],
  });
  expect(userPlugins(config.plugins).length).toBe(1);
});

test('defineConfig supports Plugin objects in plugins array', () => {
  const config = defineConfig({
    plugins: [{ name: 'plugin-a' }, { name: 'plugin-b' }],
  });
  expect(userPlugins(config.plugins).length).toBe(2);
});

test('defineConfig supports falsy values in plugins array', () => {
  const config = defineConfig({
    plugins: [{ name: 'real-plugin' }, false, null, undefined],
  });
  expect(userPlugins(config.plugins).length).toBe(4);
});

test('defineConfig supports nested plugin arrays', () => {
  const config = defineConfig({
    plugins: [[{ name: 'nested-a' }, { name: 'nested-b' }], { name: 'top-level' }],
  });
  expect(userPlugins(config.plugins).length).toBe(2);
});

test('defineConfig supports Promise<Plugin> in plugins array', () => {
  const config = defineConfig({
    plugins: [Promise.resolve({ name: 'async-plugin' })],
  });
  expect(userPlugins(config.plugins).length).toBe(1);
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
  expect(userPlugins(config.plugins).length).toBe(6);
});

test('defineConfig supports empty plugins array', () => {
  const config = defineConfig({
    plugins: [],
  });
  expect(userPlugins(config.plugins).length).toBe(0);
});

test('defineConfig supports config without plugins', () => {
  const config = defineConfig({});
  // defineConfig always injects its rewrite plugin even when user omits `plugins`.
  expect(userPlugins(config.plugins).length).toBe(0);
});

test('defineConfig supports function config with plain plugins array', () => {
  const configFn = defineConfig(() => ({
    plugins: [{ name: 'fn-plugin' }],
  }));
  const config = configFn({ command: 'build', mode: 'production' });
  expect(userPlugins(config.plugins).length).toBe(1);
});

test('defineConfig supports async function config with plain plugins array', async () => {
  const configFn = defineConfig(async () => ({
    plugins: [{ name: 'async-fn-plugin' }],
  }));
  const config = await configFn({ command: 'build', mode: 'production' });
  expect(userPlugins(config.plugins).length).toBe(1);
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
  const userOnly = userPlugins(config.plugins);
  expect(userOnly.length).toBe(1);
  expect((userOnly[0] as { name: string })?.name).toBe('vitest-plugin');
});
