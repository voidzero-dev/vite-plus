export default {
  run: {
    tasks: {
      'build:site': {
        command: 'vitepress build',
        // The install-URL rewrite in .vitepress/config.mts depends on this
        // variable, so different deploy targets must not share cached output.
        env: ['DOCS_SITE_ORIGIN'],
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
