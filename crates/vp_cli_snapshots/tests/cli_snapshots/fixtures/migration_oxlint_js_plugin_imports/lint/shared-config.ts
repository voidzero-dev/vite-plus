import { defineConfig } from 'oxlint';
import type { OxlintOverride } from 'oxlint';

export const testOverride: OxlintOverride = {
  files: ['**/*.test.ts'],
  rules: { 'local/no-foo': 'off' },
};

export default defineConfig({ overrides: [testOverride] });
