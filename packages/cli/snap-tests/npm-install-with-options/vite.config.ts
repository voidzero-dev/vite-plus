export default {
  run: {
    cache: true,
    tasks: {
      install: {
        command: 'vp install --production --silent',
        input: [{ auto: true }, '!node_modules/**', '!package-lock.json'],
      },
    },
  },
};
