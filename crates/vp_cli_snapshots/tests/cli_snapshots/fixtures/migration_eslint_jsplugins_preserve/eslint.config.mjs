// Inline-defined `survives` plugin — @oxlint/migrate translates it into
// `lint.jsPlugins: ["eslint-plugin-survives"]`. The package is listed
// in this fixture's package.json devDependencies, so:
//   1. The cleanup step should NOT delete `eslint-plugin-survives`
//      from package.json (it's referenced by the generated jsPlugins
//      array — removing it would invalidate the lint config we just
//      generated).
//   2. The sanitizer should NOT strip the jsPlugins entry (the
//      package is present in the workspace).
//   3. The `survives/no-fiction` rule should survive in the merged
//      `lint.rules` (the `survives` namespace is backed by the kept
//      jsPlugin).
export default [
  {
    plugins: {
      survives: {
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
      'survives/no-fiction': 'warn',
    },
  },
];
