export default {
  test: {
    globals: true,
  },
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
    rules: {
      "typescript/no-unsafe-call": "error",
    },
  },
};
