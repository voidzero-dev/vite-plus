// Pinned-via-`-c` config: relaxes the root's strict rules.
// Used to prove that `-c <path>` disables the cwd walk-up.
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
