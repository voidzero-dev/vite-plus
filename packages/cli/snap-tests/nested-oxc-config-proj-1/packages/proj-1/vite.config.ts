// Nested proj-1 config: relaxes the root's strict rules.
// Running `vp lint` / `vp fmt` from packages/proj-1 should pick this up
// (matching oxc-project/oxc#20416), but currently the root config is used instead.
export default {
  lint: {
    rules: {
      'no-debugger': 'off',
    },
  },
  fmt: {
    singleQuote: false,
  },
};
