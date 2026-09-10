import { defineConfig } from 'oxlint';
import type { OxlintOverride } from 'oxlint';

export const override: OxlintOverride = { files: ['**/*.ts'] };

export default defineConfig({ overrides: [override] });
export { 'defineConfig' as config } from 'oxlint';
export type { 'OxlintOverride' as Override } from 'oxlint';
