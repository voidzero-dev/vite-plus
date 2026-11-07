export default {
  plugins: [
    {
      name: 'task-plugin',
      config() {
        return {
          run: {
            tasks: {
              'build:prod': {
                command: 'vpt print build:prod from plugin config hook',
              },
            },
          },
        };
      },
    },
  ],
};
