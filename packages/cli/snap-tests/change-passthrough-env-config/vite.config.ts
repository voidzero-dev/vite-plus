const passThroughEnvs = process.env.VITE_TASK_PASS_THROUGH_ENVS?.split(',') ?? ["MY_ENV"];

export default {
  tasks: {
    hello: {
      command: "node -p process.env.MY_ENV",
      passThroughEnvs,
      cache: true
    }
  }
};
