export default {
  test: {
    include: ['coverage.test.js'],
    coverage: {
      enabled: true,
      provider: 'v8',
      include: ['src/**'],
      exclude: ['src/ignored.js'],
      reporter: ['json'],
    },
  },
};
