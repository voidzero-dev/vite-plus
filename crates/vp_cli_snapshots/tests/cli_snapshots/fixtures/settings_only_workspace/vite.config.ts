export default {
  plugins: [
    {
      name: 'root-only-build-input',
      config() {
        return {
          build: {
            rolldownOptions: {
              input: 'src/index.ts',
            },
          },
        };
      },
    },
  ],
  pack: {},
};
