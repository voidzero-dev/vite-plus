export default {
  meta: { name: 'override' },
  rules: {
    'no-bar': {
      meta: { schema: [] },
      create(context) {
        return {
          Identifier(node) {
            if (node.name === 'bar') {
              context.report({ node, message: 'Do not use bar.' });
            }
          },
        };
      },
    },
  },
};
