const base = {
  options: {
    typeAware: true,
  },
};

export default {
  lint: {
    extends: [base],
    options: {
      typeCheck: true,
    },
  },
};
