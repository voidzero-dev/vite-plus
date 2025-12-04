import { expect, test } from '@voidzero-dev/vite-plus-test';

import {
  configDefaults,
  coverageConfigDefaults,
  defaultExclude,
  defaultInclude,
  defaultBrowserPort,
  defineConfig,
  defineProject,
} from '../index';

test('should keep vitest exports stable', () => {
  expect(defineConfig).toBeTypeOf('function');
  expect(defineProject).toBeTypeOf('function');
  expect(configDefaults).toBeDefined();
  expect(coverageConfigDefaults).toBeDefined();
  expect(defaultExclude).toBeDefined();
  expect(defaultInclude).toBeDefined();
  expect(defaultBrowserPort).toBeDefined();
});
