const passThroughEnvs = process.env.VITE_TASK_PASS_THROUGH_ENVS?.split(',');
const cwd = process.env.VITE_TASK_CWD;

export default {
  tasks: {
    hello: {
      command: "node hello.mjs",
      envs: ["FOO", "BAR"],
      cache: true,
      ...(passThroughEnvs && { passThroughEnvs }),
      ...(cwd && { cwd })
    }
  }
};
