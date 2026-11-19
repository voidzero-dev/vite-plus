export default {
  run: {
    tasks: {
      codegen: {
        command: 'node generate.mjs',
      },
      'codegen-with-glob': {
        command: 'node generate.mjs',
        cache: {
          output: [{ auto: true }, 'generated/**'],
        },
      },
    },
  },
};
