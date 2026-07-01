export default {
  run: {
    tasks: {
      'build:site': {
        command: 'vitepress build',
        input: [
          { auto: true },
          '!.vitepress/.temp/**',
          '!.vitepress/dist/**',
          '!node_modules/.vite-temp/**',
        ],
      },
    },
  },
};
