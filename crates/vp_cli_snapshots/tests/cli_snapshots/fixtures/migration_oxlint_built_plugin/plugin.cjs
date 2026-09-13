const { definePlugin, defineRule } = require('@oxlint/plugins');

module.exports = definePlugin({
  meta: { name: 'built' },
  rules: {
    'no-foo': defineRule({
      meta: { messages: { noFoo: 'Do not name things "foo".' } },
      create(context) {
        return {
          Identifier(node) {
            if (node.name === 'foo') {
              context.report({ node, messageId: 'noFoo' });
            }
          },
        };
      },
    }),
  },
});
