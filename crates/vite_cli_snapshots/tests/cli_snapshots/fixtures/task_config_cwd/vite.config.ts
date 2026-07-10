export default {
  run: {
    tasks: {
      hello: {
        command: 'node a.js',
        cwd: 'subfolder',
        cache: true,
      },
    },
  },
};
