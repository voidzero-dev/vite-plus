/* oxlint-disable typescript/no-require-imports -- This is the synchronous CommonJS entry for the require export condition. */
import vite = require('vite');
import vitestConfig = require('vitest/config');
import vitePlusConfig = require('./define-config');
/* oxlint-enable typescript/no-require-imports */

const {
  configDefaults,
  coverageConfigDefaults,
  defaultBrowserPort,
  defaultExclude,
  defaultInclude,
} = vitestConfig;

const { defineConfig, defineProject, lazyPlugins, resolveConfigFromFile } = vitePlusConfig;

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
