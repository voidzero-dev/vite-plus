export default {
  run: {
    tasks: {
      oidc: {
        command: 'node check-oidc.mjs',
        input: ['check-oidc.mjs'],
        output: [],
      },
      tracking: {
        command: 'node many-files.mjs',
        output: [],
      },
    },
  },
};
