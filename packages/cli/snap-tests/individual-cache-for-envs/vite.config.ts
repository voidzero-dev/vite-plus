export default {
  tasks: {
    hello: {
      command: "node -p process.env.FOO",
      envs: ["FOO"],
      cache: true
    }
  }
};
