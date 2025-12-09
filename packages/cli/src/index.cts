const vite = require('@voidzero-dev/vite-plus-core');

const vitest = require('@voidzero-dev/vite-plus-test/config');

module.exports = {
  ...vite,
  ...vitest,
};
