export default {
  meta: { name: 'local' },
  rules: {
    'no-foo': {
      meta: { schema: [] },
      create(context) {
        return {
          Identifier(node) {
            if (node.name === 'foo') {
              context.report({ node, message: 'Do not use foo.' });
            }
          },
        };
      },
    },
  },
};
