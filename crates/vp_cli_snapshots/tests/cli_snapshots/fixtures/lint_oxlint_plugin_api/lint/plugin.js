// Authored against the API vite-plus re-exports, with no `@oxlint/plugins`
// dependency of its own: the point of the test is that this resolves and loads.
import { definePlugin, defineRule } from 'vite-plus/lint/plugins';

const noFoo = defineRule({
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
});

export default definePlugin({
  meta: { name: 'local' },
  rules: { 'no-foo': noFoo },
});
