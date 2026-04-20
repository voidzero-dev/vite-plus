const vite = require('@voidzero-dev/vite-plus-core');

const vitest = require('@voidzero-dev/vite-plus-test/config');

const { defineConfig, lazyPlugins } = require('./define-config');

module.exports = {
  ...vite,
  ...vitest,
  defineConfig,
  lazyPlugins,
};
