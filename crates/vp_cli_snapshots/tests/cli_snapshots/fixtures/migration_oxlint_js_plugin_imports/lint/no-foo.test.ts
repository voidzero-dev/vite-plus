import type { Context } from 'oxlint';
import { RuleTester } from 'oxlint/plugins-dev';

import { noFoo } from './no-foo.js';

export type RuleContext = Context;

new RuleTester().run('no-foo', noFoo, {
  valid: ['const bar = 1;'],
  invalid: [{ code: 'const foo = 1;', errors: 1 }],
});
