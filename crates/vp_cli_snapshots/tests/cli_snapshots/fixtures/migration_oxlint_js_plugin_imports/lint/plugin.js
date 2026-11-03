import { definePlugin } from '@oxlint/plugins';

import { noFoo } from './no-foo.js';

export default definePlugin({
  meta: { name: 'local' },
  rules: { 'no-foo': noFoo },
});
