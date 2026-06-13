const vite = require('@voidzero-dev/vite-plus-core');

const {
  configDefaults,
  coverageConfigDefaults,
  defaultBrowserPort,
  defaultExclude,
  defaultInclude,
  defineProject,
} = require('vitest/config');

const { defineConfig, lazyPlugins } = require('./define-config');

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
};
