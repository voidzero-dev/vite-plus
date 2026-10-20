export default {
  meta: { name: 'fixture' },
  rules: {
    'require-setting': {
      meta: { schema: [] },
      create(context) {
        return {
          Program(node) {
            if (context.settings.fixture?.enabled !== true) {
              context.report({ node, message: 'The fixture setting must be enabled.' });
            }
          },
        };
      },
    },
  },
};
