export {
  defineConfig,
  defineProject,
  lazyPlugins,
  resolveConfigFromFile,
} from './define-config.ts';
export type { ResolveConfigFromFileOptions } from './define-config.ts';

export * from 'vite';

export {
  configDefaults,
  coverageConfigDefaults,
  defaultBrowserPort,
  defaultExclude,
  defaultInclude,
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
