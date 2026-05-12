export default {
  run: {
    cache: true,
    tasks: {
      check: {
        command: 'node --no-warnings check.js',
        untrackedEnv: ['NO_COLOR'],
      },
    },
  },
};
