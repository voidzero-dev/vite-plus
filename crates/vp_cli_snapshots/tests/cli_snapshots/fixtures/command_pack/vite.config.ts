if (process.env.CONFIG_MUST_NOT_LOAD) {
  throw new Error('CONFIG_MUST_NOT_LOAD');
}

export default {
  run: {
    tasks: {
      pack: {
        command: 'vp pack src/index.ts',
      },
    },
  },
};
