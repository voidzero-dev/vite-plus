import { defineConfig, lazyPlugins } from './define-config.ts';

export * from '@voidzero-dev/vite-plus-core';

export {
  configDefaults,
  coverageConfigDefaults,
  defaultBrowserPort,
  defaultExclude,
  defaultInclude,
  defineProject,
} from 'vitest/config';

export type {
  TestProjectConfiguration,
  TestProjectInlineConfiguration,
  TestTagDefinition,
  TestUserConfig,
  UserProjectConfigExport,
  UserProjectConfigFn,
  UserWorkspaceConfig,
  ViteUserConfig,
  ViteUserConfigExport,
  ViteUserConfigFn,
  ViteUserConfigFnObject,
  ViteUserConfigFnPromise,
  WatcherTriggerPattern,
} from 'vitest/config';

export { defineConfig, lazyPlugins };
