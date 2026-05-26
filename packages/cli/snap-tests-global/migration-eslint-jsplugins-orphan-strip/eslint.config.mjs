// Configures a `fictional/*` rule via a plugin namespace that resolves
// to neither a native Oxlint plugin nor an installed JS plugin package.
// Mirrors the WeakAuras-style failure where `@oxlint/migrate` emits
// `jsPlugins: ["eslint-plugin-fictional"]` and rules under `fictional/*`,
// even though `eslint-plugin-fictional` is not in node_modules.
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
];
