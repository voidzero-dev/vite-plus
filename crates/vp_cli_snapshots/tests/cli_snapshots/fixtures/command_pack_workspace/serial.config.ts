let building = false;

export default {
  pack: {
    workspace: 'packages/*',
    entry: ['index.ts'],
    plugins: [
      {
        name: 'check-build-limit',
        async buildStart() {
          if (building) throw new Error('Workspace builds overlapped');
          building = true;
          // Keep the hook pending so a second unscheduled build can enter it.
          await new Promise((resolve) => setTimeout(resolve, 50));
        },
        buildEnd() {
          building = false;
        },
      },
    ],
  },
};
