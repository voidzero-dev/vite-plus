import { expect, test } from '@voidzero-dev/vite-plus-test';

import {
  build,
  defineConfig,
  globalLogger,
  mergeConfig,
  resolveUserConfig,
  buildWithConfigs,
  enableDebug,
} from '../pack.js';

test('should export all pack APIs from @voidzero-dev/vite-plus-core/pack', () => {
  expect(defineConfig).toBeTypeOf('function');
  expect(build).toBeTypeOf('function');
  expect(globalLogger).toBeDefined();
  expect(mergeConfig).toBeTypeOf('function');
  expect(resolveUserConfig).toBeTypeOf('function');
  expect(buildWithConfigs).toBeTypeOf('function');
  expect(enableDebug).toBeTypeOf('function');
});
