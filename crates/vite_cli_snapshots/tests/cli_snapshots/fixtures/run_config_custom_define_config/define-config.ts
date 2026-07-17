export function defineConfig(_config: unknown) {
  return {
    run: {
      tasks: {
        selected: {
          command: 'node runtime.js',
          dependsOn: [],
        },
      },
    },
  };
}
