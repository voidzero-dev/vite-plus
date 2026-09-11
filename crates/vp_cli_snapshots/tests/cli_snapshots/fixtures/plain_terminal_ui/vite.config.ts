const untrackedEnv = process.env.VITE_TASK_PASS_THROUGH_ENVS?.split(',');
const cwd = process.env.VITE_TASK_CWD;

export default {
  run: {
    tasks: {
      hello: {
        command: 'node hello.mjs',
        env: ['FOO', 'BAR'],
        cache: true,
        ...(untrackedEnv && { untrackedEnv }),
        ...(cwd && { cwd }),
      },
    },
  },
};
