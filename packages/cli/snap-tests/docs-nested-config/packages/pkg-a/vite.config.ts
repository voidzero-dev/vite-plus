// Nested config in pkg-a: relaxes the root's strict rules.
// cwd walk-up from pkg-a/ should pick this file, not the root.
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
