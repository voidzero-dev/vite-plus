const untrackedEnv = process.env.VITE_TASK_PASS_THROUGH_ENVS?.split(',') ?? ['MY_ENV'];

export default {
  run: {
    tasks: {
      hello: {
        command: 'node -p process.env.MY_ENV',
        untrackedEnv,
        cache: true,
      },
    },
  },
};
