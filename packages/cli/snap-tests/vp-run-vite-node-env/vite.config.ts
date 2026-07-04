export default {
  build: {
    modulePreload: false,
  },
  run: {
    tasks: {
      build: 'vp build',
      'db:migrate:prod': 'node -e ""',
      deploy: {
        command: 'node -e ""',
        dependsOn: ['build', 'db:migrate:prod'],
        cache: false,
      },
    },
  },
  plugins: [
    {
      name: 'node-env',
      config() {
        console.log(`NODE_ENV=${JSON.stringify(process.env.NODE_ENV)}`);
      },
    },
  ],
};
