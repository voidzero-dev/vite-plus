export default {
  run: {
    cache: true,
    tasks: {
      check: {
        command: 'node check.js',
        untrackedEnv: ['NO_COLOR'],
      },
    },
  },
};
