const vite = require('@voidzero-dev/vite-plus-core');

const vitest = require('vitest/config');

const { defineConfig, lazyPlugins } = require('./define-config');

module.exports = {
  ...vite,
  ...vitest,
  defineConfig,
  lazyPlugins,
};
