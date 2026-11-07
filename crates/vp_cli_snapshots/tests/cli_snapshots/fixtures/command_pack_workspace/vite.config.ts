export default {
  pack: {
    entry: ['index.ts'],
    workspace: { include: ['packages/*'], exclude: ['**/b'] },
  },
};
