function myTaskPlugin() {
  return {
    name: 'my-task-plugin',
    config() {
      return {
        run: {
          tasks: {
            'build:prod': {
              command: "echo 'build:prod from plugin config hook'",
            },
          },
        },
      };
    },
  };
}

export default {
  plugins: [myTaskPlugin()],
};
