import { defineRule } from 'oxlint';
import { definePlugin } from '@oxlint/plugins';
import { RuleTester } from 'oxlint/plugins-dev';

export { defineRule, definePlugin, RuleTester };
export { 'defineRule' as rule } from 'oxlint';
export type { 'Context' as RuleContext } from 'oxlint';
