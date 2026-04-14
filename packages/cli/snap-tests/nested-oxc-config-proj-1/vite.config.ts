// Root config: strict rules that should NOT apply when running from packages/proj-1.
// If proj-1's nested vite.config.ts is loaded correctly, these rules are overridden.
export default {
  lint: {
    rules: {
      'no-debugger': 'error',
    },
  },
  fmt: {
    singleQuote: true,
  },
};
