export default {
  run: {
    tasks: {
      'build:site': {
        command: 'vitepress build',
        // The docs and installer URLs depend on the explicit origin or Workers
        // branch, so different deploy targets must not share cached output.
        env: ['DOCS_SITE_ORIGIN', 'WORKERS_CI', 'WORKERS_CI_BRANCH'],
        input: [
          { auto: true },
          '!.vitepress/.temp/**',
          '!.vitepress/dist/**',
          '!node_modules',
          '!node_modules/.vite-temp/**',
        ],
        output: ['.vitepress/dist/**'],
      },
    },
  },
};
