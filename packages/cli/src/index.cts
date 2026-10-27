const vite = require('vite');

const {
  configDefaults,
  coverageConfigDefaults,
  defaultBrowserPort,
  defaultExclude,
  defaultInclude,
} = require('vitest/config');

const {
  defineConfig,
  defineProject,
  lazyPlugins,
  resolveConfigFromFile,
} = require('./define-config');

module.exports = {
  ...vite,
  configDefaults,
  coverageConfigDefaults,
  defaultBrowserPort,
  defaultExclude,
  defaultInclude,
  defineProject,
  defineConfig,
  lazyPlugins,
  resolveConfigFromFile,
};
