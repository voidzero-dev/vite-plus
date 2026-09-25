const untrackedEnv = process.env.VITE_TASK_PASS_THROUGH_ENVS?.split(',');
const cwd = process.env.VITE_TASK_CWD;

export default {
  run: {
    tasks: {
      hello: {
        command: 'node hello.mjs',
        cache: {
          env: ['FOO', 'BAR'],
          ...(untrackedEnv && { untrackedEnv }),
        },
        ...(cwd && { cwd }),
      },
    },
  },
};
