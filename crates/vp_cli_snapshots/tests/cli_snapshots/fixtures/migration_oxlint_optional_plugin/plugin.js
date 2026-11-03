import { definePlugin, defineRule } from '@oxlint/plugins';

export default definePlugin({
  meta: { name: 'optional' },
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
