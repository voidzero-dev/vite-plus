// Exercises the sanitizer:
//   1. base-level `fictional/*` rule via an inline plugin namespace that
//      doesn't resolve to a native Oxlint plugin nor an installed
//      package — translates into `jsPlugins: ['eslint-plugin-fictional']`
//      + rule under `fictional/*` (the WeakAuras-style failure shape).
//   2. an OVERRIDE that introduces a second unresolvable plugin
//      (`./*.test.js` files only) — verifies the per-override sanitize
//      path strips both the override's `jsPlugins` entry and its rules.
export default [
  {
    plugins: {
      fictional: {
        rules: {
          'no-fiction': {
            meta: { type: 'problem' },
            create() {
              return {};
            },
          },
        },
      },
    },
    rules: {
      'fictional/no-fiction': 'warn',
    },
  },
  {
    files: ['**/*.test.js'],
    plugins: {
      'override-only': {
        rules: {
          'no-skip': {
            meta: { type: 'problem' },
            create() {
              return {};
            },
          },
        },
      },
    },
    rules: {
      'override-only/no-skip': 'error',
    },
  },
];
